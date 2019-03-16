// std library imports
use std::fs::File;
use std::io::Write;
use std::f64;
use std::time::{Instant, Duration};
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

// 3rd party crate imports
#[cfg(target_os = "windows")]
extern crate winapi;

extern crate winit;
use winit::{ControlFlow, Event, WindowBuilder, WindowEvent};
use winit::dpi::LogicalSize;
use winit_utils::*;

extern crate num_cpus;
extern crate lazy_static;

//extern crate rayon;
//use rayon::prelude::*;

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

use math::*;
use hitable::*;
use camera::Camera;
use texture::ConstantTexture;
use material::*;
use sphere::Sphere;
use bvh::BvhNode;
use trace::*;
use jobs::{Jobs, JobTask, JobDescriptor};

// For tracking multithreading bugs
const RUN_SINGLE_THREADED: bool = false;

pub fn run() {

    let nx: u32 = 1280;
    let ny: u32 = 720;
    let ns: u32 = 100; // number of samples
    let image_size = (nx,ny);

    let window_width = nx as f64;
    let window_height = ny as f64;

    let mut events_loop = winit::EventsLoop::new();
    let builder = WindowBuilder::new();
    let window = Arc::new(builder.with_dimensions(LogicalSize{width: window_width, height: window_height}).build(&events_loop).unwrap());
    window.set_title("Path Tracer");

    let start_timer = Instant::now();
    
    update_window_title_status(&window, &format!("Starting.. image size ({} x {})", nx, ny));

    //let world = two_spheres();
    let world = four_spheres();

    //let lookfrom = Vec3::new(13.0,2.0,3.0);
    //let lookat = Vec3::new(0.0,0.0,0.0);
    let lookfrom = Vec3::new(-2.0,2.0,1.0);
    let lookat = Vec3::new(0.0,0.0,-1.0);
    let dist_to_focus = 10.0;
    let aperture = 0.1;
    let aspect: f64 = (nx as f64)/(ny as f64);
    let cam = Arc::new(Camera::new(lookfrom, lookat, Vec3::new(0.0,1.0,0.0), 90.0, aspect, aperture, dist_to_focus, 0.0, 1.0));

    let buffer_size_bytes = (nx*ny*3) as usize;
    let bgr_texture = Arc::new(Mutex::new(vec![0_u8; buffer_size_bytes]));
    update_window_framebuffer(&window, &mut bgr_texture.lock().unwrap(), image_size);

    let num_cores = num_cpus::get();
    println!("Running on {} cores", num_cores);

    let num_tasks_xy = (8, 6);
    // sanitize so num tasks divides exactly into image
    let num_tasks_xy = (round_down_to_closest_factor(num_tasks_xy.0, nx), round_down_to_closest_factor(num_tasks_xy.1, ny));
    let task_dim_xy = (nx / num_tasks_xy.0, ny / num_tasks_xy.1);
    let window_lock = Arc::new(AtomicBool::new(false));
    let remaining_tasks = Arc::new(AtomicUsize::new((num_tasks_xy.0*num_tasks_xy.1) as usize));

    update_window_title_status(&window, &format!("Tracing... {} tasks", num_tasks_xy.0 * num_tasks_xy.1));

    let run_single_threaded = RUN_SINGLE_THREADED;
    if !run_single_threaded {
        let mut batches = vec![];
        for task_y in 0..num_tasks_xy.1 {
            for task_x in 0..num_tasks_xy.0 {
                let window_lock = Arc::clone(&window_lock);
                let cam = cam.clone();
                let world = world.clone();
                let window = window.clone();
                let image_buffer = bgr_texture.clone();
                let remaining_tasks = remaining_tasks.clone();

                let start_xy = (task_dim_xy.0 * task_x, task_dim_xy.1 * task_y);
                let end_xy = (start_xy.0 + task_dim_xy.0, start_xy.1 + task_dim_xy.1);
                let batch = TraceSceneBatchJob::new(cam, 
                                                    world, 
                                                    ns, 
                                                    start_xy, end_xy, 
                                                    image_buffer, image_size, 
                                                    remaining_tasks, 
                                                    window_lock, window);
                batches.push(JobDescriptor::new(Box::new(batch)));
            }
        }

        Jobs::dispatch_jobs(batches);

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
            if remaining_tasks.compare_and_swap(0, 1, Ordering::Acquire) == 0 {
                break;
            }

            // yield thread
            thread::sleep(Duration::from_secs(1));
        }
    } else {
        let world = four_spheres();
        let start_xy = (0, 0);
        let end_xy = image_size;
        let image_buffer = Arc::clone(&bgr_texture);
        let cam = cam.clone();
        let world = world.clone();
        let window = window.clone();
        let remaining_tasks = remaining_tasks.clone();
        let batch = TraceSceneBatchJob::new(cam, 
                                    world, 
                                    ns, 
                                    start_xy, end_xy, 
                                    image_buffer, image_size, 
                                    remaining_tasks, 
                                    window_lock, window);
        batch.run();
        //trace_scene_mt(&cam, &world, ns, start_xy, end_xy, image_buffer, image_size, remaining_tasks, window_lock, &window);
    }

    // stats
    let duration = start_timer.elapsed();
    let duration_in_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    update_window_title_status(&window, &format!("Done.. in {}s.", duration_in_secs));
    
    // don't need this across threads now, keep unlocked
    let mut bgr_texture = bgr_texture.lock().unwrap();

    // write image 
    let image_file_name = "output.ppm";
    save_bgr_texture_as_ppm(image_file_name, &bgr_texture, image_size);

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
                    update_window_framebuffer(&window, &mut bgr_texture, image_size);
                    ControlFlow::Continue
                },
                _ => ControlFlow::Continue,
            },
             _ => ControlFlow::Continue,
        }
    });
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
fn two_spheres() -> Box<Hitable + Send + Sync + 'static> {
    let red_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(1.0, 0.0, 0.0)))));
    let blue_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.0, 0.0, 1.0)))));

    let list: Vec<Arc<Hitable + Send + Sync + 'static>> = vec![
        Arc::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Arc::new(Sphere::new(Vec3::new(0.0,  10.0, 0.0), 10.0, blue_material)),
    ];

    Box::new(HitableList::new(list))
}

fn four_spheres() -> Arc<Hitable + Send + Sync + 'static> {
    let red_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.9, 0.0, 0.0)))));
    let blue_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.0, 0.1, 0.8)))));
    let green_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.0, 0.9, 0.0)))));
    let yellow_material = Arc::new(Lambertian::new(Arc::new(ConstantTexture::new(Vec3::new(0.9, 0.9, 0.0)))));

    let dielectric_material = Arc::new(Dielectric::new(1.2));
    let metal_material = Arc::new(Metal::new(Vec3::from_float(0.9), 0.5));

    let list: Vec<Arc<Hitable + Send + Sync + 'static>> = vec![
        Arc::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Arc::new(Sphere::new(Vec3::new(0.0,  -100.5, -1.0), 100.0, blue_material)),
        Arc::new(Sphere::new(Vec3::new(1.0,  0.0, -1.0), 0.5, green_material)),
        Arc::new(Sphere::new(Vec3::new(-1.0,  0.0, -1.0), 0.5, yellow_material)),
        Arc::new(Sphere::new(Vec3::new(-2.0,  0.0, -1.0), 0.5, dielectric_material)),
        Arc::new(Sphere::new(Vec3::new(2.0,  0.0, -1.0), 0.5, metal_material)),
    ];

    Arc::new(BvhNode::from_list(list, 0.0, 1.0))
    //Box::new(HitableList::new(list))
}

//#[allow(dead_code)]
//pub fn trace_scene(cam: &Camera, world: &Box<Hitable>, num_samples: u32, start_xy: (u32, u32), end_xy: (u32, u32), 
//                   buffer_width_height: (u32, u32), draw_lock: Arc<AtomicBool>, window: &winit::Window) 
//{
    //for j in (0..ny).rev() {
    //    for i in 0..nx {
//
    //        let mut col = Vec3::new_zero_vector();
    //        for _ in 0..ns {
    //            let random = random::rand();
    //            let u: f64 = ((i as f64) + random) / (nx as f64);
    //            let random = random::rand();
    //            let v: f64 = ((j as f64) + random) / (ny as f64);
//
    //            let r = cam.get_ray(u, v);
    //            col += color(&r, &world, 0);
//
    //            // SS: Debug uv image
    //            // col += Vec3::new(u, v, 0.0);
    //        }
//
    //        col = col / ns as f64;
    //        col = Vec3::new(col.x().sqrt(), col.y().sqrt(), col.z().sqrt()); // Gamma correct 1/2.0
//
    //        let ir = (255.99*col.r()) as u8;
    //        let ig = (255.99*col.g()) as u8;
    //        let ib = (255.99*col.b()) as u8;
    //        
    //        let offset = (j * nx * 3 + i * 3) as usize;
    //        bgr_image_buffer[offset] = ib;
    //        bgr_image_buffer[offset+1] = ig;
    //        bgr_image_buffer[offset+2] = ir;
    //        rgb_image_buffer[offset] = ir;
    //        rgb_image_buffer[offset+1] = ig;
    //        rgb_image_buffer[offset+2] = ib;
    //    }
//
    //    if j % 20 == 0 {
    //        
    //        // Poll message loop while we trace
    //        events_loop.poll_events(|event| {
    //            use winit::VirtualKeyCode;
    //            match event {
    //                Event::WindowEvent { event, .. } => match event {
    //                    WindowEvent::KeyboardInput { input, .. } => {
    //                        if let Some(VirtualKeyCode::Escape) = input.virtual_keycode {
    //                            std::process::exit(0);
    //                        }
    //                    }
    //                    WindowEvent::CloseRequested => std::process::exit(0),
    //                    _ => {},
    //                },
    //                _ => {},
    //            }
    //        });
//
    //        // Update frame buffer to show progress
    //        update_window_framebuffer(&window, &mut bgr_image_buffer, (nx, ny));
    //        
    //        let progress = 100.0 * ((ny - (j+1)) * nx) as f64 / ((ny*nx) as f64);
    //        let progress_string = format!("Ray Tracer: Progress {} {}%",  (ny - j) * nx, progress);
    //        window.set_title(&progress_string);
    //        print!("\r{}", &progress_string);
    //    }
    //}
//}