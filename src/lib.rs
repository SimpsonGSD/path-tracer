#![cfg_attr(
    not(any(feature = "dx12", feature = "metal", feature = "vulkan")),
    allow(unused)
)]

// std library imports
use std::fs::File;
use std::io::Write;
use std::f64;
use std::time::{Instant, Duration};
use parking_lot::{RwLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

// 3rd party crate imports
#[cfg(target_os = "windows")]
extern crate winapi;
extern crate num_cpus;
extern crate lazy_static;
extern crate parking_lot;

#[cfg(feature = "dx12")]
pub type Backend = rendy::dx12::Backend;

#[cfg(feature = "metal")]
pub type Backend = rendy::metal::Backend;

#[cfg(feature = "vulkan")]
pub type Backend = rendy::vulkan::Backend;

#[cfg(feature = "empty")]
pub type Backend = rendy::empty::Backend;

extern crate rendy;
use rendy::{
    command::{Graphics, Supports},
    factory::{Factory, ImageState},
    graph::{present::PresentNode, render::*, GraphBuilder},
};

use rendy::hal;
use rendy::wsi::winit;
use winit::{ControlFlow, Event, WindowBuilder, WindowEvent};
use winit::dpi::LogicalSize;
use winit_utils::*;

// module imports
mod math;
mod hitable;
mod material;
mod texture;
mod camera;
mod sphere;
mod bvh;
mod trace;
mod winit_utils;
mod jobs;
mod node;

use math::*;
use hitable::*;
use camera::Camera;
use texture::{ConstantTexture, CheckerTexture};
use material::*;
use sphere::{Sphere, MovingSphere};
use bvh::BvhNode;
use trace::*;
use jobs::{Jobs, JobTask, MultiSliceReadWriteLock};

// For tracking multithreading bugs
const RUN_SINGLE_THREADED: bool = false;
const OUTPUT_IMAGE_ON_CLOSE: bool = false;
const FRAMES_IN_FLIGHT: u32 = 3;

// Returns the cargo manifest directory when running the executable with cargo
// or the directory in which the executable resides otherwise,
// traversing symlinks if necessary.
pub fn application_root_dir() -> String {
    match std::env::var("CARGO_MANIFEST_DIR") {
        Ok(_) => String::from(env!("CARGO_MANIFEST_DIR")),
        Err(_) => {
            let mut path = std::env::current_exe().expect("Failed to find executable path.");
            while let Ok(target) = std::fs::read_link(path.clone()) {
                path = target;
            }
            String::from(
                path.parent()
                    .expect("Failed to get parent directory of the executable.")
                    .to_str()
                    .unwrap(),
            )
        }
    }
}

#[derive(Clone, Copy)]
pub struct Config {
    realtime: bool
}

impl Config {
    pub fn new(realtime: bool) -> Self {
        Config {
            realtime
        }
    }
}

#[cfg(not(any(feature = "dx12", feature = "metal", feature = "vulkan")))]
pub fn run(config: Config) -> Result<(), failure::Error>{
    Err(failure::err_msg("run with --feature dx/metal/vulkan"))
}

#[cfg(any(feature = "dx12", feature = "metal", feature = "vulkan"))]
pub fn run(config: Config) -> Result<(), failure::Error>{

    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Warn)
        .filter_module("rendy", log::LevelFilter::Info)
        .init();

    let nx: u32 = 1280;
    let ny: u32 = 720;
    let ns: u32 = if config.realtime {1} else {100}; // number of samples
    let image_size = (nx,ny);

    let window_width = nx as f64;
    let window_height = ny as f64;

    let mut events_loop = winit::EventsLoop::new();
    let builder = WindowBuilder::new();
    let window = builder.with_dimensions(LogicalSize{width: window_width, height: window_height}).build(&events_loop).unwrap();
    window.set_title("Path Tracer");

    //+ Rendy integration
    let (mut factory, mut families): (Factory<Backend>, _) = {
        let config: rendy::factory::Config = Default::default();
        rendy::factory::init(config)?
    };
    let surface = factory.create_surface(&window);
    let hw_alignment = hal::adapter::PhysicalDevice::limits(factory.physical())
        .min_uniform_buffer_offset_alignment;
    let queue = families
        .as_slice()
        .iter()
        .find(|family| {
            if let Some(Graphics) = family.capability().supports() {
                true
            } else {
                false
            }
        })
        .unwrap()
        .as_slice()[0]
        .id();

    factory.maintain(&mut families);
    let mut graph_builder = GraphBuilder::<Backend, node::Aux>::new();
    let color = graph_builder.create_image(
        hal::image::Kind::D2(image_size.0, image_size.1, 1, 1),
        1,
        factory.get_surface_format(&surface),
        Some(hal::command::ClearValue::Color([0.1, 0.3, 0.4, 1.0].into())),
    );
    let tonemap_pass = graph_builder.add_node(
        node::tonemap::Pipeline::builder()
            .into_subpass()
            .with_color(color)
            .into_pass(),
    );
    graph_builder.add_node(PresentNode::builder(&factory, surface, color).with_dependency(tonemap_pass));
    
    let mut aux = node::Aux {
        frames: FRAMES_IN_FLIGHT as usize,
        hw_alignment,
        tonemapper_args: node::tonemap::TonemapperArgs {
            exposure: 1.0,
            clear_colour: [1.0, 0.0, 0.0],
        }
    };

    let mut frame_graph = graph_builder
        .with_frames_in_flight(FRAMES_IN_FLIGHT)
        .build(&mut factory, &mut families, &mut aux)?;
    //- Rendy integration

    let start_timer = Instant::now();
    
    update_window_title_status(&window, &format!("Starting.. image size ({} x {})", nx, ny));

    //let world = two_spheres();
    //let world = four_spheres();
    let world = random_scene(0.0, 1000.0);

    let lookfrom = Vec3::new(0.0,4.0,13.0);
    //let lookat = Vec3::new(0.0,0.0,0.0);
    //let lookfrom = Vec3::new(-2.0,2.0,1.0);
    let lookat = Vec3::new(0.0,0.0,-1.0);
    let dist_to_focus = 11.0;
    let aperture = 0.001;
    let aspect: f64 = (nx as f64)/(ny as f64);

   // let cam = Arc::new(RwLock::new(Camera::new(lookfrom, lookat, Vec3::new(0.0,1.0,0.0), 20.0, aspect, aperture, dist_to_focus, 0.0, 1.0)));
    let cam = Camera::new(lookfrom, lookat, Vec3::new(0.0,1.0,0.0), 20.0, aspect, aperture, dist_to_focus, 0.0, 1.0);

    // TODO(SS): This is temporary and will be handled by the GPU. Tonemap, gamma and convert to uint.
    let convert_to_u8_and_gamma_correct = |buffer: &Vec<f32>| -> Vec<u8>{
        let mut output = Vec::with_capacity(buffer.len());
         buffer.chunks(3).map(|chunk| {
            let colour = Vec3::new(chunk[2] as f64,chunk[1] as f64,chunk[0] as f64);
            reinhard_tonemap(&colour)
        }).for_each(|colour|{   output.push((255.99 * colour.z.sqrt()) as u8);
                                output.push((255.99 * colour.y.sqrt()) as u8);
                                output.push((255.99 * colour.x.sqrt()) as u8);});

        output
    };


    let buffer_size_bytes = (nx*ny*3) as usize;
    let bgr_texture = MultiSliceReadWriteLock::new(vec![0.0_f32; buffer_size_bytes]);
    update_window_framebuffer(&window, &mut convert_to_u8_and_gamma_correct(bgr_texture.read()), image_size);

    let num_cores = num_cpus::get();
    println!("Running on {} cores", num_cores);

    let task_dim_xy = (120, 120);
    // sanitize so num tasks divides exactly into image
    let task_dim_xy = (round_down_to_closest_factor(task_dim_xy.0, nx), round_down_to_closest_factor(task_dim_xy.1, ny));
    let num_tasks_xy = (nx / task_dim_xy.0, ny / task_dim_xy.1);
    let num_tasks = num_tasks_xy.0 * num_tasks_xy.1;
    let window_lock = AtomicBool::new(false);
    let remaining_tasks = AtomicUsize::new((num_tasks) as usize);

    update_window_title_status(&window, &format!("Tracing... {} tasks", num_tasks));

    let default_disable_emissive = config.realtime; // Disable emissive for realtime by default as it's noisy
    let default_sky_brightness = if default_disable_emissive {1.0} else {0.6};
    let scene_state = Arc::new(RwLock::new(SceneState::new(cam, world, window, 0.0, 1.0/60.0, default_sky_brightness, default_disable_emissive, config)));
    let scene_output = Arc::new(SceneOutput::new(bgr_texture, remaining_tasks, window_lock));

    if !config.realtime {
        if !RUN_SINGLE_THREADED {
            let mut batches: Vec<Arc<RwLock<dyn JobTask + Send + Sync + 'static>>> = vec![];
            for task_y in 0..num_tasks_xy.1 {
                for task_x in 0..num_tasks_xy.0 {
                    let start_xy = (task_dim_xy.0 * task_x, task_dim_xy.1 * task_y);
                    let end_xy = (start_xy.0 + task_dim_xy.0, start_xy.1 + task_dim_xy.1);
                    let batch = TraceSceneBatchJob::new(ns, 
                                                        start_xy, end_xy, 
                                                        image_size, 
                                                         scene_state.clone(),
                                                         scene_output.clone(),
                                                         config.realtime);
                    batches.push(Arc::new(RwLock::new(batch)));
                }
            }

            Jobs::dispatch_jobs(&batches);

            loop {
                // Poll message loop while we trace so we can early-exit
                events_loop.poll_events(|event| {
                    use winit::VirtualKeyCode;
                    match event {
                        Event::WindowEvent { event, .. } => match event {
                            WindowEvent::KeyboardInput { input, .. } => {
                                if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                                    std::process::exit(0);
                                }
                            }
                            WindowEvent::CloseRequested => std::process::exit(0),
                            _ => {},
                        },
                        _ => {},
                    }
                });

                // wait for threads to finish by checking atomic ref count on the shared image buffer
                // Note(SS): Could use condvars here but then wouldn't be able to poll the message queue
                if scene_output.remaining_tasks.compare_and_swap(0, 1, Ordering::Acquire) == 0 {
                    break;
                }

                let percent_done = ((num_tasks - scene_output.remaining_tasks.load(Ordering::Relaxed) as u32) as f32 / num_tasks as f32) * 100.0;
                update_window_title_status(&scene_state.read().window, &format!("Tracing... {} tasks, {} x {} {}spp. {}% done",  num_tasks, nx, ny, ns, percent_done));


                // yield thread
                thread::sleep(Duration::from_secs(1));
            }
        } else {
            let start_xy = (0, 0);
            let end_xy = image_size;
            let mut batch = TraceSceneBatchJob::new(ns, 
                                                start_xy, end_xy, 
                                                image_size, 
                                                scene_state.clone(), 
                                                scene_output.clone(),
                                                config.realtime);
            batch.run();
        }
        
        // stats
        let duration = start_timer.elapsed();
        let duration_in_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
        update_window_title_status(&scene_state.read().window, &format!("Done.. in {}s.", duration_in_secs));

        // write image 
        let image_file_name = "output.ppm";
        save_bgr_texture_as_ppm(image_file_name, &convert_to_u8_and_gamma_correct(scene_output.buffer.read()), image_size);

        events_loop.run_forever(|event| {
            use winit::VirtualKeyCode;
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput { input, .. } => {
                        if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
                            ControlFlow::Break
                        } else {
                            ControlFlow::Continue
                        }
                    }
                    WindowEvent::CloseRequested => winit::ControlFlow::Break,
                    WindowEvent::Resized(..) => {
                        update_window_framebuffer(&scene_state.read().window, &mut convert_to_u8_and_gamma_correct(scene_output.buffer.read()), image_size);
                        ControlFlow::Continue
                    },
                    _ => ControlFlow::Continue,
                },
                 _ => ControlFlow::Continue,
            }
        });

    } else  {
    
        let controls_string = "Controls: O/P - Decrease/Increase Sky Brightness;  B - Toggle Emissive";

        let mut batches = vec![];
        let mut jobs: Vec<Arc<RwLock<dyn JobTask + Send + Sync + 'static>>>  = vec![];
        for task_y in 0..num_tasks_xy.1 {
            for task_x in 0..num_tasks_xy.0 {
                let start_xy = (task_dim_xy.0 * task_x, task_dim_xy.1 * task_y);
                let end_xy = (start_xy.0 + task_dim_xy.0, start_xy.1 + task_dim_xy.1);
                let batch = TraceSceneBatchJob::new(ns, 
                                                    start_xy, end_xy, 
                                                     image_size, 
                                                     scene_state.clone(), 
                                                     scene_output.clone(),
                                                     config.realtime);
                let batch = Arc::new(RwLock::new(batch));
                batches.push(batch.clone());
                jobs.push(batch);
            }
        }

        const CAM_SPEED: f64 = 4.0;
        const MOUSE_LOOK_SPEED: f64 = 0.4;
        //const MOUSE_THRESHOLD: 
        let mut keep_running = true;
        let mut fps = 0.0;
        let mut move_forward = false;
        let mut frame_time = 1.0 / 60.0;
        let mut move_left = false;
        let mut move_right = false;
        let mut move_backward = false;
        let mut move_up = false;
        let mut move_down = false;
        let mut look_right = false;
        let mut look_left = false;
        let mut look_up = false;
        let mut look_down = false;
        let mut left_mouse_down = false;
        let mut right_mouse_down = false;
        let mut mouse_x = 0.0;
        let mut mouse_y = 0.0;
        let mut b_down  = false;
        while keep_running {

            let start_timer = Instant::now();

            //+ Rendy Integration
            factory.maintain(&mut families);
            //- Rendy Integration

            // App logic - modifying of shared state allowed
            {
                let mut scene_state_writable = scene_state.write();
                let mut clear_scene = false;

                // update time
                scene_state_writable.time0 = scene_state_writable.time1;
                scene_state_writable.time1 += frame_time;

                let dpi = scene_state_writable.window.get_current_monitor().get_hidpi_factor();

                // TODO(SS): debouncing, needs moving to struct
                let mouse_x_last_frame = mouse_x;
                let mouse_y_last_frame = mouse_y;
                let _left_mouse_down_last_frame = left_mouse_down;
                let right_mouse_down_last_frame = right_mouse_down;
                let b_down_last_frame = b_down;
                b_down = false;
              //  right_mouse_down = false;
              //  mouse_x = 0.0;
              //  mouse_y = 0.0;
              //  left_mouse_down = false;

                events_loop.poll_events(|event| {
                    use winit::VirtualKeyCode;
                    use winit::MouseButton;
                    use winit::ElementState;
                    match event {
                    Event::WindowEvent { event, .. } => match event {
                            WindowEvent::KeyboardInput { input, .. } => match input.virtual_keycode {
                                Some(VirtualKeyCode::Escape) => keep_running = false,
                                Some(VirtualKeyCode::W) => move_forward = true,
                                Some(VirtualKeyCode::S) => move_backward = true,
                                Some(VirtualKeyCode::D) => move_right = true,
                                Some(VirtualKeyCode::A) => move_left = true,
                                Some(VirtualKeyCode::Q) => move_down = true,
                                Some(VirtualKeyCode::E) => move_up = true,
                                Some(VirtualKeyCode::Right) => look_right = true,
                                Some(VirtualKeyCode::Left) => look_left = true,
                                Some(VirtualKeyCode::Up) => look_up = true,
                                Some(VirtualKeyCode::Down) => look_down = true,
                                Some(VirtualKeyCode::O) => {
                                    clear_scene = true;
                                    scene_state_writable.sky_brightness = (scene_state_writable.sky_brightness - 0.05).max(0.0);
                                },
                                Some(VirtualKeyCode::P) => {
                                    clear_scene = true;
                                    scene_state_writable.sky_brightness += 0.05;
                                },
                                Some(VirtualKeyCode::B) => b_down = true,
                                _ => {},
                            },
                            WindowEvent::MouseInput { state, button, .. } => {
                                if button == MouseButton::Left {
                                    left_mouse_down = if state == ElementState::Pressed {true} else {false};
                                }
                                if button == MouseButton::Right {
                                    right_mouse_down = if state == ElementState::Pressed {true} else {false};
                                }
                            },
                            WindowEvent::CursorMoved { position, .. } => {
                                // Note(SS): This position is not ideal for mouse movement as it contains OS overrides like mouse accel.
                                let physical_position = position.to_physical(dpi);
                                mouse_x = physical_position.x;
                                mouse_y = -physical_position.y;
                            },
                            WindowEvent::CloseRequested => keep_running = false,
                            WindowEvent::Resized(..) => update_window_framebuffer(&scene_state_writable.window, &mut convert_to_u8_and_gamma_correct(scene_output.buffer.read()), image_size),
                            _ => {},
                        },
                        _ => {},
                    }
                });

                if b_down_last_frame && !b_down {
                    scene_state_writable.disable_emissive = !scene_state_writable.disable_emissive;
                    clear_scene = true;
                }

                // handle input for camera
                // TODO(SS): Move state into app struct and move to function just to keep this loop tidier
                {
                    let cam = &mut scene_state_writable.cam;
                    let mut camera_moved = false;
                    if move_forward {
                        move_forward = false;
                        let cam_origin = cam.get_origin();
                        let cam_forward = cam.get_forward();
                        let diff = cam_forward * CAM_SPEED * frame_time;
                        cam.set_origin(cam_origin + &diff, true);
                        camera_moved = true;
                    } 

                    if move_backward {
                        move_backward = false;
                        let cam_origin = cam.get_origin();
                        let cam_forward = cam.get_forward();
                        let diff = -cam_forward * CAM_SPEED * frame_time;
                        cam.set_origin(cam_origin + &diff, true);
                        camera_moved = true;
                    }

                    if move_right {
                        move_right = false;
                        let cam_origin = cam.get_origin();
                        let cam_right = cam.get_right();
                        let diff = cam_right * CAM_SPEED * frame_time;
                        cam.set_origin(cam_origin + &diff, true);
                        camera_moved = true;
                        
                    }

                    if move_left {
                        move_left = false;
                        let cam_origin = cam.get_origin();
                        let cam_right = cam.get_right();
                        let diff = -cam_right * CAM_SPEED * frame_time;
                        cam.set_origin(cam_origin + &diff, true);
                        camera_moved = true;
                    }

                    if move_up {
                        move_up = false;
                        let cam_origin = cam.get_origin();
                        let cam_up = cam.get_up();
                        let diff = cam_up * CAM_SPEED * frame_time;
                        cam.set_origin(cam_origin + &diff, true);
                        camera_moved = true;
                    }

                    if move_down {
                        move_down = false;
                        let cam_origin = cam.get_origin();
                        let cam_up = cam.get_up();
                        let diff = -cam_up * CAM_SPEED * frame_time;
                        cam.set_origin(cam_origin + &diff, true);
                        camera_moved = true;
                    }
                    
                    if look_right {
                        look_right = false;
                        let cam_look_at = cam.get_look_at();
                        let cam_right = cam.get_right();
                        cam.set_look_at(cam_look_at + cam_right * CAM_SPEED * frame_time, true);
                        camera_moved = true;
                    }
                    if look_left {
                        look_left = false;
                        let cam_look_at = cam.get_look_at();
                        let cam_right = cam.get_right();
                        cam.set_look_at(cam_look_at + -cam_right * CAM_SPEED * frame_time, true);
                        camera_moved = true;
                    }
                    if look_up {
                        look_up = false;
                        let cam_look_at = cam.get_look_at();
                        let cam_up = cam.get_up();
                        cam.set_look_at(cam_look_at + cam_up * CAM_SPEED * frame_time, true);
                        camera_moved = true;
                    }
                    if look_down {
                        look_down = false;
                        let cam_look_at = cam.get_look_at();
                        let cam_up = cam.get_up();
                        cam.set_look_at(cam_look_at + -cam_up * CAM_SPEED * frame_time, true);
                        camera_moved = true;
                    }
                    if right_mouse_down && right_mouse_down_last_frame {
                        let mouse_x_delta = mouse_x - mouse_x_last_frame;
                        let mouse_y_delta = mouse_y - mouse_y_last_frame;
                        if mouse_x_delta != 0.0 || mouse_y_delta != 0.0
                        { 
                            let mut cam_look_at = cam.get_look_at();
                            let cam_right = cam.get_right();
                            let cam_up = cam.get_up();
                            if mouse_x_delta != 0.0 {
                                cam_look_at += cam_right * MOUSE_LOOK_SPEED * frame_time * mouse_x_delta
                            }
                            if mouse_y_delta != 0.0 {
                                cam_look_at += cam_up * MOUSE_LOOK_SPEED * frame_time * mouse_y_delta;
                            }

                            cam.set_look_at(cam_look_at, true);
                            camera_moved = true;
                        }
                    }


                    if camera_moved || clear_scene {
                        cam.update();
                        batches.iter().for_each(|batch| batch.write().clear_buffer());
                        let buffer = scene_output.buffer.write();
                        *buffer = vec![0.0_f32; buffer_size_bytes];

                    }
                }
            }

            let job_counter = Jobs::dispatch_jobs(&jobs);
            Jobs::wait_for_counter(&job_counter, 0);

            let scene_state_readable = scene_state.read();


            //+ Rendy Integration
            frame_graph.run(&mut factory, &mut families, &mut aux);
            //update_window_framebuffer(&scene_state_readable.window, &mut convert_to_u8_and_gamma_correct(scene_output.buffer.read()), image_size);
            //- Rendy Integration

            // throttle main thread to 60fps
            const SIXTY_HZ: Duration = Duration::from_micros(1_000_000 / 60);
            match SIXTY_HZ.checked_sub(start_timer.elapsed()) {
                Some(sleep_time) => {
                    thread::sleep(sleep_time);
                },
                None => {}
            };
            
            let frame_duration = start_timer.elapsed();
            frame_time = frame_duration.as_secs() as f64 + frame_duration.subsec_nanos() as f64 * 1e-9;

            fps = fps* 0.9 + 0.1 * (1.0 / frame_time);
            scene_state_readable.window.set_title(&format!("Path Tracer: FPS = {}  |  Sky Brightness = {}; Emissive = {}  |  {}", fps as i32,
                                                            scene_state_readable.sky_brightness, !scene_state_readable.disable_emissive, controls_string));
        }

        // write image 
        if OUTPUT_IMAGE_ON_CLOSE {
            let image_file_name = "output.ppm";
            save_bgr_texture_as_ppm(image_file_name, &convert_to_u8_and_gamma_correct(scene_output.buffer.read()), image_size);
        }

        frame_graph.dispose(&mut factory, &mut aux);
    }

    Ok(())
}

fn update_window_title_status(window: &winit::Window, status: &str) {
    println!("{}", status);
    window.set_title(&format!("Path Tracer: {}", status));
}

fn save_bgr_texture_as_ppm(filename: &str, bgr_buffer: &Vec<u8>, buffer_size: (u32,u32)) {
    
    let timer = Instant::now();
    
    // convert to rgb buffer and flip horizontally as (0,0) is bottom left for ppm
    let buffer_length = bgr_buffer.len();
    let mut rgb_buffer = vec![0; buffer_length];
    for j in 0..buffer_size.1 {
        let j_flipped = buffer_size.1 - j - 1;
        for i in 0..buffer_size.0 {
            let offset_x = i * 3;
            let rgb_offset = (j * buffer_size.0 * 3 + offset_x) as usize;
            let bgr_offset = (j_flipped * buffer_size.0  * 3 + offset_x) as usize;
            rgb_buffer[rgb_offset]   = bgr_buffer[bgr_offset+2];
            rgb_buffer[rgb_offset+1] = bgr_buffer[bgr_offset+1];
            rgb_buffer[rgb_offset+2] = bgr_buffer[bgr_offset];
        }
    }
    
    let mut output_image = File::create(filename).expect("Could not open file for write");
    let header = format!("P6 {} {} 255\n", buffer_size.0, buffer_size.1);
    output_image.write(header.as_bytes()).expect("failed to write to image file");
    output_image.write(&rgb_buffer).expect("failed to write to image");

    let duration = timer.elapsed();
    let duration_in_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    println!("{} saved in {}s", filename, duration_in_secs);
}

#[allow(dead_code)]
fn two_spheres() -> Box<dyn Hitable + Send + Sync + 'static> {
    let red_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(1.0, 0.0, 0.0))), 0.0));
    let blue_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.0, 0.0, 1.0))), 0.0));

    let list: Vec<Arc<dyn Hitable + Send + Sync + 'static>> = vec![
        Arc::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Arc::new(Sphere::new(Vec3::new(0.0,  10.0, 0.0), 10.0, blue_material)),
    ];

    Box::new(HitableList::new(list))
}

#[allow(dead_code)]
fn four_spheres() -> Box<dyn Hitable + Send + Sync + 'static> {
    let red_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.9, 0.0, 0.0))), 0.0));
    let blue_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.3, 0.3, 0.3))), 0.0));
    let green_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.0, 0.9, 0.0))), 0.0));
    let yellow_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.9, 0.9, 0.0))), 0.0));

    let dielectric_material = Arc::new(Dielectric::new(1.6));
    let metal_material = Arc::new(Metal::new(Vec3::from_float(1.0), 0.0));

    let list: Vec<Arc<dyn Hitable + Send + Sync + 'static>> = vec![
        Arc::new(Sphere::new(Vec3::new(4.0, -0.3, 0.7), 0.3, red_material)),
        Arc::new(Sphere::new(Vec3::new(0.0,  -100.5, -1.0), 100.0, blue_material)),
        Arc::new(Sphere::new(Vec3::new(1.0,  0.0, -1.0), 0.5, green_material)),
        Arc::new(Sphere::new(Vec3::new(-1.0,  0.0, 0.0), 0.5, yellow_material)),
        Arc::new(Sphere::new(Vec3::new(0.4,  -0.25, -0.3), 0.25, dielectric_material.clone())),
        Arc::new(Sphere::new(Vec3::new(2.5,  -0.15, -0.4), 0.25, dielectric_material.clone())),
        //Arc::new(Sphere::new(Vec3::new(0.4,  0.0, 0.0), 0.1, dielectric_material)),
        Arc::new(Sphere::new(Vec3::new(2.0,  0.0, -1.0), 0.5, metal_material.clone())),
        Arc::new(Sphere::new(Vec3::new(1.6,  0.0, 1.0), 0.5, metal_material.clone())),
    ];

    Box::new(BvhNode::from_list(list, 0.0, 1.0))
    //Box::new(HitableList::new(list))
}

#[allow(dead_code)]
fn random_scene(t_min: f64, t_max: f64) -> Box<dyn Hitable + Send + Sync + 'static> {
    let checker_texture = Arc::new(CheckerTexture::new(Arc::new(ConstantTexture::new(Vec3::new(0.2, 0.3, 0.1))), 
                                                      Arc::new(ConstantTexture::new(Vec3::new(0.9, 0.9, 0.9)))));

    let mut list: Vec<Arc<dyn Hitable + Send + Sync + 'static>> = vec![];

    list.push(Arc::new(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, Arc::new(Lambertian::new(checker_texture.clone(), 0.0)))));

    // TODO
    const MOVING_SPHERES: bool = false;

    if true {
        for a in -11..11 {
            for b in -11..11 {
                let choose_mat = random::rand();
                let mut center = Vec3::new(a as f64 + 0.9 * random::rand(), 0.2, b as f64 + 0.9 * random::rand());
                if (&center - Vec3::new(4.0, 0.2, 0.0)).length() > 0.9 {
                    let material: Arc<dyn Material + Send + Sync + 'static>;
                    let mut is_emissive = false;
                    if choose_mat < 0.6 { // diffuse 
                        is_emissive = random::rand() < 0.1;
                        let emissive = if is_emissive {30.0} else {0.0};
                        material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(
                                                        random::rand()*random::rand(), 
                                                        random::rand()*random::rand(), 
                                                        random::rand()*random::rand()))), emissive));
                                                

                    } else if choose_mat < 0.8 { // metal
                        material = Arc::new(Metal::new(Vec3::new(0.5*(1.0+random::rand()),0.5*(1.0+random::rand()),0.5*(1.0+random::rand())),
                                                        0.2*random::rand()));
                    } else { // glass
                        material = Arc::new( Dielectric::new(1.5));
                    }
                    
                    if MOVING_SPHERES {
                        list.push(Arc::new(MovingSphere::new(center.clone(), &center+Vec3::new(0.0,0.5*random::rand(),0.0), 0.0, 1.0, 0.2, material)));
                    } else {
                        let radius = if is_emissive {1.0} else {0.2};
                        center.y += if is_emissive {10.0} else {0.0};
                        list.push(Arc::new(Sphere::new(center.clone(),radius, material)));
                    }
                }
            }
        }
    }

    list.push(Arc::new(Sphere::new(Vec3::new(0.0, 1.0, 0.0), 1.0,Arc::new(Dielectric::new(1.5)))));
    list.push(Arc::new(Sphere::new(Vec3::new(-4.0, 1.0, 0.0), 1.0,Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.4, 0.2, 0.1))), 0.0)))));
    list.push(Arc::new(Sphere::new(Vec3::new(4.0, 1.0, 0.0), 1.0,Arc::new(Metal::new(Vec3::new(0.7, 0.6, 0.5), 0.0)))));

    Box::new(BvhNode::from_list(list, t_min, t_max))
}