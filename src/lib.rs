use std::fs::File;
use std::io::Write;
use std::f64;

mod math;
mod hitable;
mod material;
mod texture;
mod camera;
mod sphere;

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

pub fn run() {
    
    let mut output_image = File::create("output.ppm").expect("Could not open file for write");

    let nx = 600;
    let ny = 300;
    let ns = 50; // number of samples

    // write image header
    write!(output_image, "P3\n{} {}\n255\n", nx, ny).expect("failed to write to image file");

    println!("Starting.. image size ({} x {})", nx, ny);

    let world = two_spheres();
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

    for j in (0..ny).rev() {
        for i in 0..nx {

            let mut col = Vec3::new_zero_vector();
            for s in 0..ns {
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

            let ir = (255.99*col.r()) as i32;
            let ig = (255.99*col.g()) as i32;
            let ib = (255.99*col.b()) as i32;

           if ( i + (ny - j) * nx) % 400 == 0 {
               print!("\rProgress: {} {}%", i + (ny - j) * nx, 100.0 * ((i+1) + ((ny - (j+1)) * nx)) as f64 / ((ny*nx) as f64));
           }

           // TODO(SS): Fix this and write to buffer as this is slow to do every loop.
           write!(output_image, "{} {} {}\n", ir, ig, ib).expect("failed to write image file");
        }
    }

    println!("");
    println!("Done..");
}

fn two_spheres() -> Box<Hitable> {
    let red_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(1.0, 0.0, 0.0)))));
    let blue_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(0.0, 0.0, 1.0)))));

    let n = 50;

    let list: Vec<Box<dyn Hitable>> = vec![
        Box::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Box::new(Sphere::new(Vec3::new(0.0,  10.0, 0.0), 10.0, blue_material)),
    ];

    Box::new(HitableList::new(list))
}

fn four_spheres() -> Box<Hitable> {
    let red_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(1.0, 0.0, 0.0)))));
    let blue_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(0.0, 0.0, 1.0)))));
    let green_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(0.0, 1.0, 0.0)))));
    let yellow_material = Rc::new(Lambertian::new(Rc::new(ConstantTexture::new(Vec3::new(1.0, 1.0, 0.0)))));

    let list: Vec<Box<dyn Hitable>> = vec![
        Box::new(Sphere::new(Vec3::new(0.0, 0.0, -1.0), 0.5, red_material)),
        Box::new(Sphere::new(Vec3::new(0.0,  -100.5, -1.0), 100.0, blue_material)),
        Box::new(Sphere::new(Vec3::new(1.0,  0.0, -1.0), 0.5, green_material)),
        Box::new(Sphere::new(Vec3::new(-1.0,  0.0, -1.0), 0.5, yellow_material)),
    ];

    Box::new(HitableList::new(list))
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

fn sky_color(r : &Ray, world: &Box<Hitable>, depth: i32) -> Vec3 {

    let unit_dir = Vec3::new_unit_vector(&r.direction());
    let t = 0.5*(unit_dir.y() + 1.0);
    let white = Vec3::from_float(1.0);
    let sky = Vec3::new(0.5, 0.7, 1.0);
    lerp(&white, &sky, t)  
}
