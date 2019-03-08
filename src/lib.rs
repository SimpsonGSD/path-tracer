use std::fs::File;
use std::io::Write;
use std::f64;
use std::time::{Instant};

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

pub fn get_physical_window_size(window: &winit::Window) -> (f64, f64) {
    let dpi_factor = window.get_current_monitor().get_hidpi_factor();
    let window_size = window.get_inner_size().unwrap().to_physical(dpi_factor);
    (window_size.width, window_size.height)
}

#[cfg(target_os = "windows")]
pub fn update_window_framebuffer(window: &winit::Window, buffer: &mut Vec<u8>, buffer_width_height: (u32, u32)) {
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
            biWidth: buffer_width_height.0 as i32,
            biHeight: buffer_width_height.1 as i32,
            biPlanes: 1,
            biBitCount: 24,
            biCompression:  BI_RGB,
            biSizeImage: buffer_width_height.1 * buffer_width_height.0 * 3,
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
                      buffer_width_height.0 as i32,
                      buffer_width_height.1 as i32, 
                      buffer.as_mut_ptr() as *mut c_void,
                      &bitmap_info,
                      DIB_RGB_COLORS,
                      SRCCOPY);
        assert_ne!(result, 0);
    };

}

pub fn run() {

    let nx: u32 = 1600;
    let ny: u32 = 900;
    let ns: u32 = 50; // number of samples

    let window_width = 1920.0;
    let window_height = 1080.0;

    let mut events_loop = winit::EventsLoop::new();
    let builder = WindowBuilder::new();
    let window = builder.with_dimensions(LogicalSize{width: window_width, height: window_height}).build(&events_loop).unwrap();
    window.set_title("Ray Tracer");

    let start_timer = Instant::now();
    
    println!("Starting.. image size ({} x {})", nx, ny);

    //let world = two_spheres();
    let world = four_spheres();

    //let lookfrom = Vec3::new(13.0,2.0,3.0);
    //let lookat = Vec3::new(0.0,0.0,0.0);
    let lookfrom = Vec3::new(-2.0,2.0,1.0);
    let lookat = Vec3::new(0.0,0.0,-1.0);
    let dist_to_focus = 10.0;
    let aperture = 0.1;
    let aspect: f64 = (nx as f64)/(ny as f64);
    //let cam = Camera::new(lookfrom, lookat, Vec3::new(0.0 ,1.0,0.0), 20.0, aspect, aperture, dist_to_focus, 0.0, 1.0);
    let cam = Camera::new(lookfrom, lookat, Vec3::new(0.0,1.0,0.0), 90.0, aspect, aperture, dist_to_focus, 0.0, 1.0);

    let mut bgr_image_buffer: Vec<u8> = vec![0; (nx*ny*3) as usize];
    let mut rgb_image_buffer: Vec<u8> = vec![0; (nx*ny*3) as usize];
    update_window_framebuffer(&window, &mut bgr_image_buffer, (nx, ny));

    for j in (0..ny).rev() {
        for i in 0..nx {

            let mut col = Vec3::new_zero_vector();
            for _ in 0..ns {
                let random = random::rand();
                let u: f64 = ((i as f64) + random) / (nx as f64);
                let random = random::rand();
                let v: f64 = ((j as f64) + random) / (ny as f64);

                let r = cam.get_ray(u, v);
                col += color(&r, &world, 0);

                // SS: Debug uv image
                // col += Vec3::new(u, v, 0.0);
            }

            col = col / ns as f64;
            col = Vec3::new(col.x().sqrt(), col.y().sqrt(), col.z().sqrt()); // Gamma correct 1/2.0

            let ir = (255.99*col.r()) as u8;
            let ig = (255.99*col.g()) as u8;
            let ib = (255.99*col.b()) as u8;
            
            let offset = (j * nx * 3 + i * 3) as usize;
            bgr_image_buffer[offset] = ib;
            bgr_image_buffer[offset+1] = ig;
            bgr_image_buffer[offset+2] = ir;
            rgb_image_buffer[offset] = ir;
            rgb_image_buffer[offset+1] = ig;
            rgb_image_buffer[offset+2] = ib;
        }

        if j % 20 == 0 {
            
            // Poll message loop while we trace
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

            // Update frame buffer to show progress
            update_window_framebuffer(&window, &mut bgr_image_buffer, (nx, ny));
            
            let progress = 100.0 * ((ny - (j+1)) * nx) as f64 / ((ny*nx) as f64);
            let progress_string = format!("Ray Tracer: Progress {} {}%",  (ny - j) * nx, progress);
            window.set_title(&progress_string);
            print!("\r{}", &progress_string);
        }
    }

    // write image 
    let mut output_image = File::create("output.ppm").expect("Could not open file for write");
    let header = format!("P6 {} {} 255\n", nx, ny);
    output_image.write(header.as_bytes()).expect("failed to write to image file");
    output_image.write(&rgb_image_buffer).expect("failed to write to image");

    let duration = start_timer.elapsed();
    let duration_in_secs = duration.as_secs() as f64 + duration.subsec_nanos() as f64 * 1e-9;
    println!("");
    println!("Done.. in {} s", duration_in_secs);
    window.set_title(&format!("Ray Tracer: Done.. in {}s", duration_in_secs));

    update_window_framebuffer(&window, &mut bgr_image_buffer, (nx, ny));

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
                    update_window_framebuffer(&window, &mut bgr_image_buffer, (nx, ny));
                    ControlFlow::Continue
                },
                _ => ControlFlow::Continue,
            },
             _ => ControlFlow::Continue,
        }
    });
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

    let list: Vec<Rc<dyn Hitable>> = vec![
        Rc::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Rc::new(Sphere::new(Vec3::new(0.0,  -100.5, -1.0), 100.0, blue_material)),
        Rc::new(Sphere::new(Vec3::new(1.0,  0.0, -1.0), 0.5, green_material)),
        Rc::new(Sphere::new(Vec3::new(-1.0,  0.0, -1.0), 0.5, yellow_material)),
        Rc::new(Sphere::new(Vec3::new(-2.0,  0.0, -1.0), 0.5, dielectric_material)),
    ];

    Box::new(BvhNode::from_list(list, 0.0, 1.0))
    //Box::new(HitableList::new(list))
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
