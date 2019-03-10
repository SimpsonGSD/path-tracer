use std::fs::File;
use std::io::Write;
use std::f64;
use std::time::{Instant, Duration};
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

mod math;
mod hitable;
mod material;
mod texture;
mod camera;
mod sphere;
mod bvh;

use math::*;
use math::vec3::Vec3;
use math::ray::Ray;
use math::random;
use hitable::*;
use camera::Camera;
use texture::ConstantTexture;
use material::*;
use sphere::Sphere;
use std::rc::Rc;
use bvh::BvhNode;

#[cfg(target_os = "windows")]
extern crate winapi;

extern crate winit;
use winit::{ControlFlow, Event, WindowBuilder, WindowEvent};
use winit::dpi::LogicalSize;

//extern crate rayon;
//use rayon::prelude::*;

type LockableImageBuffer = Arc<Mutex<Vec<u8>>>;
type ThreadsafeCounter = Arc<AtomicUsize>;

pub fn run() {

    let nx: u32 = 1600;
    let ny: u32 = 900;
    let ns: u32 = 50; // number of samples
    let image_size = (nx,ny);

    let window_width = nx as f64;
    let window_height = ny as f64;

    let mut events_loop = winit::EventsLoop::new();
    let builder = WindowBuilder::new();
    let window = Arc::new(builder.with_dimensions(LogicalSize{width: window_width, height: window_height}).build(&events_loop).unwrap());
    window.set_title("Ray Tracer");

    let start_timer = Instant::now();
    
    println!("Starting.. image size ({} x {})", nx, ny);

    //let world = two_spheres();

    //let lookfrom = Vec3::new(13.0,2.0,3.0);
    //let lookat = Vec3::new(0.0,0.0,0.0);
    let lookfrom = Vec3::new(-2.0,2.0,1.0);
    let lookat = Vec3::new(0.0,0.0,-1.0);
    let dist_to_focus = 10.0;
    let aperture = 0.1;
    let aspect: f64 = (nx as f64)/(ny as f64);
    let cam = Arc::new(Camera::new(lookfrom, lookat, Vec3::new(0.0,1.0,0.0), 90.0, aspect, aperture, dist_to_focus, 0.0, 1.0));

    let buffer_size_bytes = (nx*ny*3) as usize;
    let bgr_image_buffer = Arc::new(Mutex::new(vec![0_u8; buffer_size_bytes]));
    update_window_framebuffer(&window, &mut bgr_image_buffer.lock().unwrap(), image_size);

    let num_tasks_xy = (4, 3);
    let task_dim_xy = (nx / num_tasks_xy.0, ny / num_tasks_xy.1);
    let window_lock = Arc::new(AtomicBool::new(false));
    let remaining_tasks = Arc::new(AtomicUsize::new((num_tasks_xy.0*num_tasks_xy.1) as usize));

    let run_single_threaded = false;

    if !run_single_threaded {
        let mut handles = vec![];
        // TODO(SS): bottom is at top and vice versa
        for task_y in 0..num_tasks_xy.1 {
            for task_x in 0..num_tasks_xy.0 {
                let window_lock = Arc::clone(&window_lock);
                let cam = Arc::clone(&cam);
                let window = Arc::clone(&window);
                let image_buffer = Arc::clone(&bgr_image_buffer);
                let remaining_tasks = Arc::clone(&remaining_tasks);

                let handle = thread::spawn( move || {
                    let world = four_spheres();
                    let start_xy = (task_dim_xy.0 * task_x, task_dim_xy.1 * task_y);
                    let end_xy = (start_xy.0 + task_dim_xy.0, start_xy.1 + task_dim_xy.1);
                    trace_scene_mt(&cam, &world, ns, start_xy, end_xy, image_buffer, image_size, remaining_tasks, window_lock, &window);
                });

                handles.push(handle);
            }
        }

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
            // TODO(SS): Use condvars instead to wait on event?
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
        let image_buffer = Arc::clone(&bgr_image_buffer);
        trace_scene_mt(&cam, &world, ns, start_xy, end_xy, image_buffer, image_size, remaining_tasks, window_lock, &window);
    }

    // write image 
    let image_file_name = "output.ppm";
    save_bgr_buffer_as_ppm(image_file_name, &bgr_image_buffer.lock().unwrap(), image_size);
    println!("image saved: {}", image_file_name);
    
    // stats
    let duration = start_timer.elapsed();
    let duration_in_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    println!("");
    println!("Done.. in {} s", duration_in_secs);
    window.set_title(&format!("Ray Tracer: Done.. in {}s", duration_in_secs));

    // don't need this across threads now.
    let mut bgr_image_buffer = bgr_image_buffer.lock().unwrap();

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
                    update_window_framebuffer(&window, &mut bgr_image_buffer, image_size);
                    ControlFlow::Continue
                },
                _ => ControlFlow::Continue,
            },
             _ => ControlFlow::Continue,
        }
    });
}

fn save_bgr_buffer_as_ppm(filename: &str, bgr_buffer: &Vec<u8>, buffer_size: (u32,u32)) {
    // convert to rgb buffer
    let buffer_length = bgr_buffer.len();
    let mut rgb_buffer = vec![0; buffer_length];
    for i in (0..buffer_length).step_by(3) {
        rgb_buffer[i]   = bgr_buffer[i+2];
        rgb_buffer[i+1] = bgr_buffer[i+1];
        rgb_buffer[i+2] = bgr_buffer[i];
    }
    
    let mut output_image = File::create(filename).expect("Could not open file for write");
    let header = format!("P6 {} {} 255\n", buffer_size.0, buffer_size.1);
    output_image.write(header.as_bytes()).expect("failed to write to image file");
    output_image.write(&rgb_buffer).expect("failed to write to image");
}


fn trace_scene_mt(cam: &Camera, world: &Box<Hitable>, num_samples: u32, start_xy: (u32, u32), end_xy: (u32, u32), 
                  buffer: LockableImageBuffer, image_size: (u32, u32), remaining_tasks: ThreadsafeCounter,
                  window_lock: Arc<AtomicBool>, window: &winit::Window) {
    
    let num_pixels_xy = (end_xy.0 - start_xy.0, end_xy.1 - start_xy.1);
    let mut local_buffer = vec![0; (num_pixels_xy.0*num_pixels_xy.1*3) as usize];

    for j in (start_xy.1..end_xy.1).rev() {
        for i in start_xy.0..end_xy.0 {

            let mut col = Vec3::new_zero_vector();
            for _ in 0..num_samples {
                let random = random::rand();
                let u: f64 = ((i as f64) + random) / (image_size.0 as f64);
                let random = random::rand();
                let v: f64 = ((j as f64) + random) / (image_size.1 as f64);

                let r = cam.get_ray(u, v);
                col += color(&r, &world, 0);

                // SS: Debug uv image
                // col += Vec3::new(u, v, 0.0);
            }

            col = col / num_samples as f64;
            col = Vec3::new(col.x().sqrt(), col.y().sqrt(), col.z().sqrt()); // Gamma correct 1/2.0

            let ir = (255.99*col.r()) as u8;
            let ig = (255.99*col.g()) as u8;
            let ib = (255.99*col.b()) as u8;
            
            let local_i = i - start_xy.0;
            let local_j = j - start_xy.1;
            let offset = (local_j * num_pixels_xy.0 * 3 + local_i * 3) as usize;
            local_buffer[offset]    = ib;
            local_buffer[offset+1]  = ig;
            local_buffer[offset+2]  = ir;
        }

        if j % 20 == 0 && window_lock.compare_and_swap(false, true, Ordering::Acquire)  {
            // Update frame buffer to show progress
            update_window_framebuffer_rect(&window, &mut local_buffer, start_xy, num_pixels_xy);
            window_lock.store(false, Ordering::Release);
        }
    }

    while window_lock.compare_and_swap(false, true, Ordering::Acquire)  {
        // Update frame buffer to show progress
        update_window_framebuffer_rect(&window, &mut local_buffer, start_xy, num_pixels_xy);
        window_lock.store(false, Ordering::Release);
    }

    // copy all of our local buffer into correct slice of image buffer
    // TODO(SS): This doesn't quite work, result is squashed.
    let buffer_offset_start = (start_xy.0 * 3 + start_xy.1 * image_size.0) as usize;
    let buffer_offset_end = buffer_offset_start + local_buffer.len();
    let mut buffer_mutex = buffer.lock().unwrap();
    buffer_mutex[buffer_offset_start..buffer_offset_end].copy_from_slice(&local_buffer);

    // notify completion by decrementing task counter
    remaining_tasks.fetch_sub(1, Ordering::SeqCst);
}

fn color(r : &Ray, world: &Box<Hitable>, depth: i32) -> Vec3 {
    if let Some(hit_record) = world.hit(r, 0.001, f64::MAX) {
        if depth < 50 {
            if  let Some((scattered, attenuation)) =  hit_record.mat.scatter(r, &hit_record) {
                return attenuation * color(&scattered, &world, depth+1);
            }
        }
        return Vec3::new_zero_vector();
    } else {
        let unit_dir = Vec3::new_unit_vector(&r.direction());
        let t = 0.5*(unit_dir.y() + 1.0);
        let white = Vec3::from_float(1.0);
        let sky = Vec3::new(0.5, 0.7, 1.0);
        return lerp(&white, &sky, t);
    }
}

pub fn get_physical_window_size(window: &winit::Window) -> (f64, f64) {
    let dpi_factor = window.get_current_monitor().get_hidpi_factor();
    let window_size = window.get_inner_size().unwrap().to_physical(dpi_factor);
    (window_size.width, window_size.height)
}

#[cfg(target_os = "windows")]
pub fn update_window_framebuffer(window: &winit::Window, 
                                 buffer: &mut Vec<u8>, 
                                 buffer_size: (u32, u32)) {
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::GetDC;
    use winit::os::windows::WindowExt;
    use winapi::um::wingdi::{StretchDIBits, DIB_RGB_COLORS, SRCCOPY, BITMAPINFO, BI_RGB, RGBQUAD, BITMAPINFOHEADER};
    use winapi::ctypes::c_void;
    
    let hwnd = window.get_hwnd() as HWND;
    let window_size = get_physical_window_size(&window);

    unsafe {
        let hdc = GetDC(hwnd);
        let bmi_colors = [RGBQUAD {
            rgbBlue: 0, 
            rgbGreen: 0, 
            rgbRed: 0, 
            rgbReserved: 0 
        }];
        let bitmap_header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFO>() as u32,
            biWidth: buffer_size.0 as i32,
            biHeight: buffer_size.1 as i32,
            biPlanes: 1,
            biBitCount: 24,
            biCompression:  BI_RGB,
            biSizeImage: buffer_size.1 * buffer_size.0 * 3,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0
        };
        let bitmap_info = BITMAPINFO{
            bmiHeader: bitmap_header,
            bmiColors: bmi_colors
        };
        let result = StretchDIBits(hdc,
                      0,
                      0,
                      window_size.0 as i32,
                      window_size.1 as i32,
                      0, 
                      0,
                      buffer_size.0 as i32,
                      buffer_size.1 as i32, 
                      buffer.as_mut_ptr() as *mut c_void,
                      &bitmap_info,
                      DIB_RGB_COLORS,
                      SRCCOPY);
        assert_ne!(result, 0);
    };

}

fn update_window_framebuffer_rect(window: &winit::Window, 
                                  buffer: &mut Vec<u8>, 
                                  window_pos: (u32, u32), 
                                  buffer_size: (u32, u32)) {
    use winapi::shared::windef::HWND;
    use winapi::um::winuser::GetDC;
    use winit::os::windows::WindowExt;
    use winapi::um::wingdi::{StretchDIBits, DIB_RGB_COLORS, SRCCOPY, BITMAPINFO, BI_RGB, RGBQUAD, BITMAPINFOHEADER};
    use winapi::ctypes::c_void;
    
    let hwnd = window.get_hwnd() as HWND;

    unsafe {
        let hdc = GetDC(hwnd);
        let bmi_colors = [RGBQUAD {
            rgbBlue: 0, 
            rgbGreen: 0, 
            rgbRed: 0, 
            rgbReserved: 0 
        }];
        let bitmap_header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFO>() as u32,
            biWidth: buffer_size.0 as i32,
            biHeight: buffer_size.1 as i32,
            biPlanes: 1,
            biBitCount: 24,
            biCompression:  BI_RGB,
            biSizeImage: buffer_size.1 * buffer_size.0 * 3,
            biXPelsPerMeter: 0,
            biYPelsPerMeter: 0,
            biClrUsed: 0,
            biClrImportant: 0
        };
        let bitmap_info = BITMAPINFO{
            bmiHeader: bitmap_header,
            bmiColors: bmi_colors
        };
        let result = StretchDIBits(hdc,
                      window_pos.0 as i32, 
                      window_pos.1 as i32, 
                      buffer_size.0 as i32,
                      buffer_size.1 as i32,
                      0, 
                      0,
                      buffer_size.0 as i32,
                      buffer_size.1 as i32, 
                      buffer.as_mut_ptr() as *mut c_void,
                      &bitmap_info,
                      DIB_RGB_COLORS,
                      SRCCOPY);
        assert_ne!(result, 0);
    };

}

#[allow(dead_code)]
fn two_spheres() -> Box<Hitable> {
    let red_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(1.0, 0.0, 0.0)))));
    let blue_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(0.0, 0.0, 1.0)))));

    let list: Vec<Rc<dyn Hitable>> = vec![
        Rc::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Rc::new(Sphere::new(Vec3::new(0.0,  10.0, 0.0), 10.0, blue_material)),
    ];

    Box::new(HitableList::new(list))
}

fn four_spheres() -> Box<Hitable> {
    let red_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(1.0, 0.0, 0.0)))));
    let blue_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(0.0, 0.0, 1.0)))));
    let green_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(0.0, 1.0, 0.0)))));
    let yellow_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(1.0, 1.0, 0.0)))));

    let dielectric_material = Rc::new(Dielectric::new(1.2));

    let list: Vec<Rc<Hitable>> = vec![
        Rc::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Rc::new(Sphere::new(Vec3::new(0.0,  -100.5, -1.0), 100.0, blue_material)),
        Rc::new(Sphere::new(Vec3::new(1.0,  0.0, -1.0), 0.5, green_material)),
        Rc::new(Sphere::new(Vec3::new(-1.0,  0.0, -1.0), 0.5, yellow_material)),
        Rc::new(Sphere::new(Vec3::new(-2.0,  0.0, -1.0), 0.5, dielectric_material)),
    ];

    Box::new(BvhNode::from_list(list, 0.0, 1.0))
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