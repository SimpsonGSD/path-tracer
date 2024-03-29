#![cfg_attr(
    not(any(feature = "dx12", feature = "metal", feature = "vulkan")),
    allow(unused)
)]
#![allow(dead_code)]

// std library imports
use std::fs::File;
use std::io::Write;
use std::f64;
use std::time::{Instant, Duration};
use parking_lot::{RwLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;

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
    graph::{present::PresentNode, render::*, GraphBuilder},
    resource::{BufferInfo, Buffer, Escape},
    memory::{Write as rendy_write}
};

//use rendy::init::winit;
//use ::winit;
extern crate winit;
use rendy::hal;
use winit::{ 
    event::{VirtualKeyCode},
    window::WindowBuilder,
    dpi::LogicalSize,
};

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
mod input;
mod rect;
mod axis_aligned_box;
mod scene;
mod volume;
mod onb;

use math::*;
use hitable::*;
use camera::Camera;
use texture::*;
use material::*;
use rect::*;
use axis_aligned_box::*;
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
    realtime: bool,
    max_depth: i32,
    spp: u32, // samples per pixel
}

impl Config {
    pub fn new() -> Self {
        Config {
            realtime: true,
            max_depth: 10,
            spp: 1,
        }
    }

    pub fn from_cmdline(args: &Vec<String>) -> Self {

        let mut config = Config::new();

        if args.len() > 1 {
            // set up some defaults first if offline is detected 
            if args.contains(&String::from("-offline")) {
                config.realtime = false;
                config.max_depth = 50;
                config.spp = 100;
            }

            for arg in args {
                if arg.starts_with("-spp=") {
                    let spp = &arg[5..];
                    config.spp = spp.parse().unwrap();
                }
            }
        }

        config
    }
}

#[derive(Default)]
pub struct Aux<B: hal::Backend> {
    pub frames: usize,
    pub hw_alignment: u64,
    pub tonemapper_args: node::tonemap::TonemapperArgs,
    pub source_buffer: Option<Escape<Buffer<B>>>
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
        .filter_module("path-tracer", log::LevelFilter::Trace)
        .init();

    println!("Config:\nrealtime={}\nspp={}\nmax depth={}", config.realtime, config.spp, config.max_depth);

    let nx: u32 = 500;
    let ny: u32 = 500;
    let ns: u32 = config.spp;
    let image_size = (nx,ny);

    let window_width = nx as f64;
    let window_height = ny as f64;

    let buffer_size_elements = (nx*ny*4) as usize;
    let rgba_texture = MultiSliceReadWriteLock::new(vec![0.0_f32; buffer_size_elements]);

    if false {
        for (pixel_index, colour) in rgba_texture.write().chunks_mut(4).enumerate() {
            let u = (pixel_index as f32 % nx as f32) / nx as f32;
            let v = (pixel_index as f32 / nx as f32) / ny as f32;
            for (i, pixel) in colour.iter_mut().enumerate() {
                match i {
                    0 => *pixel = u,
                    1 => *pixel = v,
                    2 => *pixel = 0.0,
                    3 => *pixel = 0.0,
                    _ => {}
                }
                //println!("u {}, v {}, i {} pixel_index {}", u, v, i, pixel_index);
            }
        }
    }

    let mut events_loop = winit::event_loop::EventLoop::new();
    let builder = WindowBuilder::new();
    let mut window = builder.with_inner_size(LogicalSize{width: window_width, height: window_height}).build(&events_loop).unwrap();
    window.set_title("Path Tracer");

    //+ Rendy integration
    let mut rendy: rendy::init::Rendy<Backend> = {
        let config: rendy::factory::Config = Default::default();
        rendy::init::Rendy::<Backend>::init(&config).map_err(|_|failure::err_msg("Could not initialise rendy"))?
      //  AnyWindowedRendy::init_auto(&config, window, &events_loop).unwrap()
    };
    let surface = rendy.factory.create_surface(&window).map_err(|_|failure::err_msg("Could create backbuffer surface"))?;
    let hw_alignment = hal::adapter::PhysicalDevice::limits(rendy.factory.physical())
        .min_uniform_buffer_offset_alignment;

    let source_buffer_size: u64 = (image_size.0 * image_size.1) as u64 * 4 * std::mem::size_of::<f32>() as u64;
    let mut source_buffer = rendy.factory
        .create_buffer(
            BufferInfo {
                size: source_buffer_size,
                usage: hal::buffer::Usage::TRANSFER_SRC
            },
            rendy::memory::Upload
        )
        .map_err(|_| failure::err_msg("Unable to create source buffer"))?;

    let source_buffer_size = source_buffer.size();
    let mut mapped_buffer = source_buffer
        .map(rendy.factory.device(), 0..source_buffer_size)
        .map_err(|_| failure::err_msg("Unable to map source buffer"))?;

    unsafe {
        let buffer = rgba_texture.read();
        let buffer_size = buffer.len() * std::mem::size_of::<f32>();
        let mut writer = mapped_buffer
            .write(rendy.factory.device(), 0..(buffer_size as u64))
            .map_err(|_| failure::err_msg("Unable to map source buffer"))?;
        writer.write(buffer.as_slice());
    }

    let mut graph_builder = GraphBuilder::<Backend, Aux<Backend>>::new();

    let source_image = graph_builder.create_image(
        hal::image::Kind::D2(image_size.0, image_size.1, 1, 1), 
        1, 
        hal::format::Format::Rgba32Sfloat, 
        Some(hal::command::ClearValue {
            color: hal::command::ClearColor {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        }),
    );

    let color = graph_builder.create_image(
        hal::image::Kind::D2(image_size.0, image_size.1, 1, 1),
        1,
        rendy.factory.get_surface_format(&surface),
        Some(hal::command::ClearValue {
            color: hal::command::ClearColor {
                float32: [1.0, 1.0, 1.0, 1.0],
            },
        }),
    );

    let copy_texture_node = graph_builder.add_node(
        node::copy_image::CopyToTexture::<Backend>::builder(
            source_image
        )
    );

    let tonemap_pass = graph_builder.add_node(
        node::tonemap::Pipeline::builder()
                .with_image(source_image)
                .into_subpass()
                .with_dependency(copy_texture_node)
                .with_color(color)
                .into_pass(),
    );
    graph_builder.add_node(PresentNode::builder(&rendy.factory, surface, color).with_dependency(tonemap_pass));
    
    let mut aux = Aux {
        frames: FRAMES_IN_FLIGHT as usize,
        hw_alignment,
        tonemapper_args: node::tonemap::TonemapperArgs {
            exposure_numframes_xx: [1.3, 1.0, 0.0, 0.0],
        },
        source_buffer: Some(source_buffer)
    };

    let frame_graph = graph_builder
        .with_frames_in_flight(FRAMES_IN_FLIGHT)
        .build(&mut rendy.factory, &mut rendy.families, &mut aux).map_err(|_|failure::err_msg("Could not build graph"))?;

    let mut frame_graph = Some(frame_graph);
    //- Rendy integration

    update_window_title_status(&window, &format!("Starting.. image size ({} x {})", nx, ny));

    //let world = two_spheres();
    //let world = four_spheres();
    //let world = random_scene(0.0, 1000.0);
    //let world = two_perlin_spheres();
    //let world = textured_sphere();
    //let world = simple_light();
    //let world = cornell_smoke();
    //let world = final_book_two();

    //let lookfrom = Vec3::new(-2.0,2.0,1.0);
    //let lookfrom = Vec3::new(26.0,2.0,3.0);
    //let lookfrom = Vec3::new(178.0,278.0,-700.0);
    //let lookfrom = Vec3::new(543.8453940318894, 271.36936326134946, -500.1594145740549);
    
    //let lookat = Vec3::new(0.0,0.0,0.0);
    //let lookat = Vec3::new(0.0,0.0,0.0);
    //let lookat = Vec3::new(278.0,278.0,0.0);
    //let lookat = Vec3::new(378.0,278.0,-300.0);
    //let lookat = Vec3::new(387.0484383920753, 253.65844851757655, -70.12524450632391);

    //let dist_to_focus = 10.0;
    //let aperture = 0.0;
    let aspect: f64 = (nx as f64)/(ny as f64);
    //let fov = 20.0;
    //let fov = 40.0;

   // let cam = Arc::new(RwLock::new(Camera::new(lookfrom, lookat, Vec3::new(0.0,1.0,0.0), 20.0, aspect, aperture, dist_to_focus, 0.0, 1.0)));
    //let cam = Camera::new(lookfrom, lookat, Vec3::new(0.0,1.0,0.0), fov, aspect, aperture, dist_to_focus, 0.0, 1.0);

    let (world, cam) = cornell_box(aspect);

    let convert_to_rgb_u8_and_gamma_correct = |buffer: &Vec<f32>| -> Vec<u8>{
        let mut output = Vec::with_capacity(buffer.len());
         buffer.chunks(4).map(|chunk| {
            let colour = Vec3::new(chunk[0] as f64,chunk[1] as f64,chunk[2] as f64);
            reinhard_tonemap(&colour)
        }).for_each(|colour|{   output.push((255.99 * colour.x.sqrt()) as u8);
                                output.push((255.99 * colour.y.sqrt()) as u8);
                                output.push((255.99 * colour.z.sqrt()) as u8);});

        output
    };

    let num_cores = num_cpus::get();
    println!("Running on {} cores", num_cores);

    let task_dim_xy = (nx / 9, ny / 9);
    println!("Task Dimensions = {}x{}", task_dim_xy.0, task_dim_xy.1);
    // sanitize so num tasks divides exactly into image
    let task_dim_xy = (round_down_to_closest_factor(task_dim_xy.0, nx), round_down_to_closest_factor(task_dim_xy.1, ny));
    println!("Task Dimensions fitted to image size = {}x{}", task_dim_xy.0, task_dim_xy.1);
    let num_tasks_xy = (nx / task_dim_xy.0, ny / task_dim_xy.1);
    let num_tasks = num_tasks_xy.0 * num_tasks_xy.1;
    let window_lock = AtomicBool::new(false);
    let remaining_tasks = AtomicUsize::new((num_tasks) as usize);

    update_window_title_status(&window, &format!("Tracing... {} tasks", num_tasks));

    let default_disable_emissive = false;//config.realtime; // Disable emissive for realtime by default as it's noisy
    let default_sky_brightness = 0.0;
    let scene_state = Arc::new(RwLock::new(SceneState::new(cam, world, 0.0, 1.0/60.0, default_sky_brightness, default_disable_emissive, config)));
    let scene_output = Arc::new(SceneOutput::new(rgba_texture, remaining_tasks, window_lock));
    let mut app_user_input_state: input::AppUserInputState = Default::default();


    if RUN_SINGLE_THREADED {
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
    
    let controls_string = "Decrease/Increase Sky Brightness = O/P | Toggle Emissive = B | Decrease/Increase Exposure = R/T";

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

    // if offline just kick off straight away
    if !config.realtime {
        Jobs::dispatch_jobs(&jobs);
    }

    let mut fps = 0.0;
    let mut frame_time = 1.0 / 60.0;
    let mut frame_counter = 0;
    let app_start_timer = Instant::now();
    let mut trace_completed = false;
    
    loop {

        let start_timer = Instant::now();

        let mut clear_scene = false;
        if config.realtime {
            let mut scene_state_writable = scene_state.write();
            // update time
            scene_state_writable.time0 = scene_state_writable.time1;
            scene_state_writable.time1 += frame_time;
        }

        aux.tonemapper_args.exposure_numframes_xx[1] += 1.0;

        let user_input = input::UserInput::poll_events_loop(&mut events_loop, &mut window, &mut app_user_input_state);  

        if app_user_input_state.grabbed {
            if config.realtime {
                if user_input.keys_pressed.contains(&VirtualKeyCode::T) {
                    aux.tonemapper_args.exposure_numframes_xx[0] += 0.1;
                } else if user_input.keys_pressed.contains(&VirtualKeyCode::R) {
                    aux.tonemapper_args.exposure_numframes_xx[0] -= 0.1;
                }

                if user_input.keys_pressed.contains(&VirtualKeyCode::O) {
                    clear_scene = true;
                    let mut scene_state_writable = scene_state.write();
                    scene_state_writable.sky_brightness = (scene_state_writable.sky_brightness - 0.05).max(0.0);
                }
                
                if user_input.keys_pressed.contains(&VirtualKeyCode::P) {
                    clear_scene = true;
                    let mut scene_state_writable = scene_state.write();
                    scene_state_writable.sky_brightness += 0.05;
                }
                
                if user_input.keys_pressed.contains(&VirtualKeyCode::B) {
                    let mut scene_state_writable = scene_state.write();
                    scene_state_writable.disable_emissive = !scene_state_writable.disable_emissive;
                    clear_scene = true;
                }

                if user_input.keys_pressed.contains(&VirtualKeyCode::K) {
                    let mut scene_state_writable = scene_state.write();
                    let cam = &mut scene_state_writable.cam;
                    println!("Camera Details\nOrigin = {}\nLookAt= {}", cam.get_origin(), cam.get_look_at());
                }

                // handle input for camera
                {
                        
                    let mut scene_state_writable = scene_state.write();
                    let cam = &mut scene_state_writable.cam;
                    let camera_moved = cam.update_from_input(&user_input, frame_time);

                    if camera_moved || clear_scene {
                        cam.update();
                        batches.iter().for_each(|batch| batch.write().clear_buffer());
                        let buffer = scene_output.buffer.write();
                        *buffer = vec![0.0_f32; buffer_size_elements];
                        aux.tonemapper_args.exposure_numframes_xx[1] = 1.0;

                    }
                }
            }
        }

        
        // if realtime we wait for all jobs to finish, else we poll.
        if config.realtime {
            let job_counter = Jobs::dispatch_jobs(&jobs);
            Jobs::wait_for_counter(&job_counter, 0);
        } else {
            // poll completion 
            if !trace_completed {
                if scene_output.remaining_tasks.compare_and_swap(0, 1, Ordering::Acquire) == 0 {
                    trace_completed = true;
                        // stats taken to complete
                    let duration = app_start_timer.elapsed();
                    let duration_in_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
                    update_window_title_status(&window, &format!("Done.. in {}s.", duration_in_secs));
                } else if frame_counter % 50 == 0 {
                    let percent_done = ((num_tasks - scene_output.remaining_tasks.load(Ordering::Relaxed) as u32) as f32 / num_tasks as f32) * 100.0;
                    update_window_title_status(&window, &format!("Tracing... {} tasks, {} x {} {}spp. {}% done",  num_tasks, nx, ny, ns,percent_done));
                }
            }
        }
        let scene_state_readable = scene_state.read();

        let source_buffer_size = aux.source_buffer.as_ref().unwrap().size();
        let mut mapped_buffer = aux.source_buffer
            .as_mut()
            .unwrap()
            .map(rendy.factory.device(), 0..source_buffer_size).unwrap();

        unsafe {
            let buffer = scene_output.buffer.read();
            let buffer_size = buffer.len() * std::mem::size_of::<f32>();
            let mut writer = mapped_buffer
                .write(rendy.factory.device(), 0..(buffer_size as u64))
                .unwrap();
            writer.write(buffer.as_slice());
        }

        //+ Rendy Integration
        rendy.factory.maintain(&mut rendy.families);
        if let Some(ref mut frame_graph) = frame_graph {
            frame_graph.run(&mut rendy.factory, &mut rendy.families, &mut aux);
        }
        frame_counter += 1;

        // throttle main thread to 60fps
        const SIXTY_HZ: Duration = Duration::from_micros(1_000_000 / 60);
        let frame_duration = start_timer.elapsed();
        match SIXTY_HZ.checked_sub(frame_duration) {
            Some(sleep_time) => {
               // println!("frame time {:?} sleeping for {:?}", frame_time, sleep_time);
                 std::thread::sleep(sleep_time);
            },
            None => {}
        };
        
        let frame_duration = start_timer.elapsed();
        frame_time = frame_duration.as_secs() as f64 + frame_duration.subsec_nanos() as f64 * 1e-9;

        if config.realtime {
            fps = fps* 0.9 + 0.1 * (1.0 / frame_time);
            window
                .set_title(
                    &format!("Path Tracer: FPS = {} (time={:.2}ms) |  Frame = {} | Sky Brightness = {:.2} | Emissive = {} | Exposure = {:.1} | {}", 
                            fps as i32, frame_time*1000.0, frame_counter,scene_state_readable.sky_brightness, !scene_state_readable.disable_emissive, aux.tonemapper_args.exposure_numframes_xx[0], controls_string));
        } 
        
        if user_input.exit_requested {

            // write image 
            if OUTPUT_IMAGE_ON_CLOSE || !config.realtime {
                // save up to 10 versions so we can have some sort of local history for comparisons
                let image_file_name = "output";
                let image_file_ext = ".ppm";
                let mut oldest_file_version = 0;
                let mut oldest_file_time = std::time::SystemTime::now();
                for i in 0..10 {
                    let image_path_string = [image_file_name, &(i as u32).to_string(), image_file_ext].concat();
                    let image_path = std::path::Path::new(&image_path_string);
                    if !image_path.exists() {
                        oldest_file_version = i;
                        break;
                    } else {
                        let file_time = image_path.metadata().unwrap().modified().unwrap();
                        if oldest_file_time > file_time {
                            oldest_file_time = file_time;
                            oldest_file_version = i;
                        } 
                    }
                }
                let image_path_string = [image_file_name, &(oldest_file_version as u32).to_string(), image_file_ext].concat();
                let image_path = std::path::Path::new(&image_path_string);
                save_rgb_texture_as_ppm(&image_path, &convert_to_rgb_u8_and_gamma_correct(scene_output.buffer.read()), image_size);
            }

            frame_graph.take().unwrap().dispose(&mut rendy.factory, &mut aux);
            println!("Exit requested");
            break;
        }
    }

    Ok(())
}

fn update_window_title_status(window: &winit::window::Window, status: &str) {
    println!("{}", status);
    window.set_title(&format!("Path Tracer: {}", status));
}

#[allow(dead_code)]
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
fn save_rgb_texture_as_ppm(filename: &std::path::Path, buffer: &Vec<u8>, buffer_size: (u32,u32)) {
    
    let timer = Instant::now();
    
    // convert to rgb buffer and flip horizontally as (0,0) is bottom left for ppm
    let buffer_length = buffer.len();
    let mut rgb_buffer = vec![0; buffer_length];
    for j in 0..buffer_size.1 {
        let j_flipped = buffer_size.1 - j - 1;
        for i in 0..buffer_size.0 {
            let rgb_offset_x = i * 3;
            let rgb_offset = (j * buffer_size.0 * 3 + rgb_offset_x) as usize;
            let buffer_offset = (j_flipped * buffer_size.0  * 3 + rgb_offset_x) as usize;
            rgb_buffer[rgb_offset]   = buffer[buffer_offset];
            rgb_buffer[rgb_offset+1] = buffer[buffer_offset+1];
            rgb_buffer[rgb_offset+2] = buffer[buffer_offset+2];
        }
    }
    
    let mut output_image = File::create(filename).expect("Could not open file for write");
    let header = format!("P6 {} {} 255\n", buffer_size.0, buffer_size.1);
    output_image.write(header.as_bytes()).expect("failed to write to image file");
    output_image.write(&rgb_buffer).expect("failed to write to image");

    let duration = timer.elapsed();
    let duration_in_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    println!("{} saved in {}s", filename.file_name().unwrap().to_str().unwrap(), duration_in_secs);
}

#[allow(dead_code)]
fn save_rgba_texture_as_ppm(filename: &str, rgba_buffer: &Vec<u8>, buffer_size: (u32,u32)) {
    
    let timer = Instant::now();
    
    // convert to rgb buffer and flip horizontally as (0,0) is bottom left for ppm
    let buffer_length = rgba_buffer.len();
    let mut rgb_buffer = vec![0; buffer_length];
    for j in 0..buffer_size.1 {
        let j_flipped = buffer_size.1 - j - 1;
        for i in 0..buffer_size.0 {
            let rgb_offset_x = i * 3;
            let rgba_offset_x = i * 4;
            let rgb_offset = (j * buffer_size.0 * 3 + rgb_offset_x) as usize;
            let rgba_offset = (j_flipped * buffer_size.0  * 3 + rgba_offset_x) as usize;
            rgb_buffer[rgb_offset]   = rgba_buffer[rgba_offset];
            rgb_buffer[rgb_offset+1] = rgba_buffer[rgba_offset+1];
            rgb_buffer[rgb_offset+2] = rgba_buffer[rgba_offset+2];
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

pub static EARTH_TEXTURE_BYTES: &[u8] = include_bytes!("../assets/textures/earthmap.jpg");

fn two_perlin_spheres() -> Box<dyn Hitable + Send + Sync + 'static> {
    let perlin_texture = Arc::new(texture::NoiseTexture::new(4.0));
    let mut list: Vec<Arc<dyn Hitable + Send + Sync + 'static>> = vec![];
    list.push(Arc::new(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, Arc::new(Lambertian::new(perlin_texture.clone(), 0.0)))));
    list.push(Arc::new(Sphere::new(Vec3::new(0.0, 2.0, 0.0), 2.0, Arc::new(Lambertian::new(perlin_texture.clone(), 0.0)))));
    Box::new(BvhNode::from_list(list, 0.0, 1.0))
}

fn textured_sphere() -> Box<dyn Hitable + Send + Sync + 'static> {
    let mut list: Vec<Arc<dyn Hitable + Send + Sync + 'static>> = vec![];
    list.push(Arc::new(Sphere::new(Vec3::new(0.0, 2.0, 0.0), 2.0, Arc::new(Lambertian::new(Arc::new(texture::ImageTexture::new(EARTH_TEXTURE_BYTES)), 0.0)))));
    list.push(Arc::new(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::from_float(0.4))), 0.0)))));
    Box::new(BvhNode::from_list(list, 0.0, 1.0))
}

fn simple_light() -> Box<dyn Hitable + Send + Sync + 'static> {
    let perlin_texture = Arc::new(texture::NoiseTexture::new(4.0));
    let mut list: Vec<Arc<dyn Hitable + Send + Sync + 'static>> = vec![];
    list.push(Arc::new(Sphere::new(Vec3::new(0.0, -1000.0, 0.0), 1000.0, Arc::new(Lambertian::new(perlin_texture.clone(), 0.0)))));
    list.push(Arc::new(Sphere::new(Vec3::new(0.0, 2.0, 0.0), 2.0, Arc::new(Lambertian::new(perlin_texture.clone(), 0.0)))));

    let diffuse_material = Arc::new(material::DiffuseLight::new(Arc::new(ConstantTexture::new(Vec3::from_float(0.4)))));

    list.push(Arc::new(Sphere::new(Vec3::new(0.0, 7.0, 0.0), 2.0, diffuse_material.clone())));
    list.push(Arc::new(rect::AxisAlignedRect::new(3.0, 5.0, 1.0, 3.0, -2.0, rect::AxisAlignedRectAxis::Z, diffuse_material.clone())));
    Box::new(BvhNode::from_list(list, 0.0, 1.0))
}

fn cornell_box(aspect: f64) -> (Box<ThreadsafeHitable>, Camera) {

    let mut material_builder = MaterialBuilder::new();

    let red_mat = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::new(0.65, 0.05, 0.05)))
        )
        .lambertian();

    let green_mat = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::new(0.12, 0.45, 0.15)))
        )
        .lambertian();

    let white_mat = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::from_float(0.73)))
        )
        .lambertian();

    let light = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::from_float(15.0)))
        )
        .diffuse_light();

    let alluminium = material_builder
        .set_albedo(Vec3::new(0.8, 0.85, 0.88))
        .metal();
    
    let glass = material_builder
        .set_refraction_index(1.5)
        .dielectric();

    let mut scene_builder = scene::SceneBuilder::new();
    
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 555.0, AxisAlignedRectAxis::X, green_mat))
        )
        .flip_normals();
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 0.0, AxisAlignedRectAxis::X, red_mat))
        );
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(213.0, 343.0, 227.0, 332.0, 554.0, AxisAlignedRectAxis::Y, light))
        )
        .flip_normals();
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 555.0, AxisAlignedRectAxis::Y, white_mat.clone()))
        )
        .flip_normals();
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 0.0, AxisAlignedRectAxis::Y, white_mat.clone()))
        );
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 555.0, AxisAlignedRectAxis::Z, white_mat.clone()))
        )
        .flip_normals();
    //scene_builder
        //.add_hitable(
           // Arc::new(AxisAlignedBox::new(Vec3::new_zero_vector(), Vec3::new(165.0, 165.0, 165.0), white_mat.clone()))
        //)
        //.rotate_y(-18.0)
        //.translate(Vec3::new(130.0, 0.0, 65.0));
        
    scene_builder
        .add_hitable(
            Arc::new(Sphere::new(Vec3::new(190.0, 90.0, 190.0), 90.0, glass.clone()))
        );

    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedBox::new(Vec3::new_zero_vector(), Vec3::new(165.0, 330.0, 165.0), white_mat.clone()))
        )
        .rotate_y(15.0)
        .translate(Vec3::new(265.0, 0.0, 295.0));

    let lookfrom = Vec3::new(278.0, 278.0, -800.0);
    let lookat = Vec3::new(278.0, 278.0, 0.0);
    let dist_to_focus = 10.0;
    let aperture = 0.0;
    let vfov = 40.0;
    let cam = Camera::new(lookfrom, lookat, Vec3::new(0.0, 1.0, 0.0),
                        vfov, aspect, aperture, dist_to_focus, 0.0, 1.0);


    (scene_builder.as_bvh(), cam)
}

fn cornell_smoke(aspect: f64) -> (Box<ThreadsafeHitable>, Camera) {

    let mut material_builder = MaterialBuilder::new();

    let red_mat = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::new(0.65, 0.05, 0.05)))
        )
        .lambertian();

    let green_mat = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::new(0.12, 0.45, 0.15)))
        )
        .lambertian();

    let white_mat = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::from_float(0.73)))
        )
        .lambertian();

    let light = material_builder
        .with_texture(
            Arc::new(ConstantTexture::new(Vec3::from_float(7.0)))
        )
        .diffuse_light();

    let mut scene_builder = scene::SceneBuilder::new();
    
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 555.0, AxisAlignedRectAxis::X, green_mat))
        )
        .flip_normals();
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 0.0, AxisAlignedRectAxis::X, red_mat))
        );
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(113.0, 443.0, 127.0, 432.0, 554.0, AxisAlignedRectAxis::Y, light))
        );
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 555.0, AxisAlignedRectAxis::Y, white_mat.clone()))
        )
        .flip_normals();
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 0.0, AxisAlignedRectAxis::Y, white_mat.clone()))
        );
    scene_builder
        .add_hitable(
            Arc::new(AxisAlignedRect::new(0.0, 555.0, 0.0, 555.0, 555.0, AxisAlignedRectAxis::Z, white_mat.clone()))
        )
        .flip_normals();

    let medium_boundary = {
        let mut scene_builder = scene::SceneBuilder::new();
        scene_builder
            .add_hitable(
                Arc::new(AxisAlignedBox::new(Vec3::new_zero_vector(), Vec3::new(165.0, 165.0, 165.0), white_mat.clone()))
            )
            .rotate_y(-18.0)
            .translate(Vec3::new(130.0, 0.0, 65.0));
        scene_builder.as_hitable()
    };

    scene_builder.add_hitable(Arc::new(volume::ConstantMedium::new(medium_boundary, 0.01, Arc::new(ConstantTexture::new(Vec3::from_float(1.0))))));
    
    let medium_boundary = {
        let mut scene_builder = scene::SceneBuilder::new();
        scene_builder
            .add_hitable(
                Arc::new(AxisAlignedBox::new(Vec3::new_zero_vector(), Vec3::new(165.0, 330.0, 165.0), white_mat.clone()))
            )
            .rotate_y(15.0)
            .translate(Vec3::new(265.0, 0.0, 295.0));
        scene_builder.as_hitable()
    };

    scene_builder.add_hitable(Arc::new(volume::ConstantMedium::new(medium_boundary, 0.01, Arc::new(ConstantTexture::new(Vec3::from_float(0.0))))));

    let lookfrom = Vec3::new(278.0, 278.0, -800.0);
    let lookat = Vec3::new(278.0, 278.0, 0.0);
    let dist_to_focus = 10.0;
    let aperture = 0.0;
    let vfov = 40.0;
    let cam = Camera::new(lookfrom, lookat, Vec3::new(0.0, 1.0, 0.0),
                      vfov, aspect, aperture, dist_to_focus, 0.0, 1.0);


    (scene_builder.as_bvh(), cam)
}

fn final_book_two() -> Box<ThreadsafeHitable> {

    let mut material_builder = MaterialBuilder::new();
    
    let white = material_builder
        .with_texture(Arc::new(ConstantTexture::new(Vec3::new(0.73, 0.73, 0.73))))
        .lambertian();
    let ground = material_builder
        .with_texture(Arc::new(ConstantTexture::new(Vec3::new(0.48, 0.83, 0.53))))
        .lambertian();
    let light = material_builder
        .with_texture(Arc::new(ConstantTexture::new(Vec3::from_float(7.0))))
        .diffuse_light();

    let mut scene_builder = scene::SceneBuilder::new();

    let mut floor_scene_builder = scene::SceneBuilder::new();
    
    let num_boxes = 20;
    for i in 0..num_boxes {
        for j in 0..num_boxes {
            let (i_f64, j_f64) = (i as f64, j as f64);
            let w = 100.0;
            let x0 = -1000.0 + i_f64*w;
            let z0 = -1000.0 + j_f64*w;
            let y0 = 0.0;
            let x1 = x0 + w;
            let y1 = 100.0*(random::rand()+0.01);
            let z1 = z0 + w;
            floor_scene_builder.add_hitable(
                Arc::new(AxisAlignedBox::new(Vec3::new(x0, y0, z0), Vec3::new(x1, y1, z1), ground.clone()))
            );
        }
    }

    scene_builder.add_hitable(floor_scene_builder.as_bvh_node());

    scene_builder.add_hitable(
        Arc::new(AxisAlignedRect::new(123.0, 423.0, 147.0, 412.0, 554.0, AxisAlignedRectAxis::Y, light.clone()))
    );
    
    let center = Vec3::new(400.0, 400.0, 200.0);
    scene_builder.add_hitable(
        Arc::new(MovingSphere::new(center, center+Vec3::new(30.0,0.0,0.0), 0.0, 1.0, 50.0, Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.7, 0.3, 0.1))), 0.0))))
    );
    scene_builder.add_hitable(
        Arc::new(Sphere::new(Vec3::new(260.0, 150.0, 45.0), 50.0, Arc::new(Dielectric::new(1.5))))
    );
    scene_builder.add_hitable(
        Arc::new(Sphere::new(Vec3::new(0.0, 150.0, 145.0), 50.0, Arc::new(Metal::new(Vec3::new(0.8, 0.8, 0.9), 10.0))))
    );

    let boundary = Arc::new(Sphere::new(Vec3::new(360.0, 150.0, 145.0), 70.0, Arc::new(Dielectric::new(1.5))));
    scene_builder.add_hitable(boundary.clone());
    scene_builder.add_hitable(
        Arc::new(volume::ConstantMedium::new(boundary.clone(), 0.2, Arc::new(ConstantTexture::new(Vec3::new(0.2, 0.4, 0.9)))))
    );

    let boundary = Arc::new(Sphere::new(Vec3::new(0.0, 0.0, 0.0), 5000.0, Arc::new(Dielectric::new(1.5))));
    scene_builder.add_hitable(boundary.clone());
    scene_builder.add_hitable(
        Arc::new(volume::ConstantMedium::new(boundary.clone(), 0.0001, Arc::new(ConstantTexture::new(Vec3::from_float(1.0)))))
    );

    let earth_material = material_builder
        .with_texture(Arc::new(ImageTexture::new(EARTH_TEXTURE_BYTES)))
        .lambertian();
    scene_builder.add_hitable(
        Arc::new(Sphere::new(Vec3::new(400.0, 200.0, 400.0), 100.0, earth_material))
    );
    scene_builder.add_hitable(
        Arc::new(Sphere::new(Vec3::new(220.0, 280.0, 300.0), 80.0, Arc::new(Lambertian::new(Arc::new(NoiseTexture::new(0.1)), 0.0))))
    );

    let ns = 1000;
    let mut sphere_scene_builder = scene::SceneBuilder::new();
    for _ in 0..ns {
        sphere_scene_builder.add_hitable(
            Arc::new(Sphere::new(Vec3::new(165.0*random::rand(), 165.0*random::rand(), 165.0*random::rand()), 10.0, white.clone()))
        );
    }
    scene_builder
        .add_hitable(sphere_scene_builder.as_bvh_node())
        .rotate_y(15.0)
        .translate(Vec3::new(-100.0, 270.0, 395.0));

    scene_builder.as_bvh()
}