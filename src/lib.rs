use std::fs::File;
use std::io::Write;

mod math;
mod hitable;
mod material;
mod texture;
mod camera;

use math::vec3::Vec3;
use math::ray::Ray;
use math::random;
use hitable::Hitable;
use camera::Camera;
use texture::ConstantTexture;

pub fn run() {
    
    let mut output_image = File::create("output.ppm").expect("Could not open file for write");

    let nx = 600;
    let ny = 300;
    let ns = 10; // number of samples

    // write image header
    write!(output_image, "P3\n{} {}\n255\n", nx, ny);

    println!("Starting.. image size ({} x {})", nx, ny);

    let r = Ray::new(Vec3::new(1.0,0.0,0.0), Vec3::new(1.0, 0.0, 0.0), 2.0);


    let lookfrom = Vec3::new(13.0,2.0,3.0);
    let lookat = Vec3::new(0.0,0.0,0.0);
    let dist_to_focus = 10.0;
    let aperture = 0.1;
    let aspect: f64 = (nx as f64)/(ny as f64);
    let cam = Camera::new(lookfrom, lookat, Vec3::new(0.0 ,1.0,0.0), 20.0, aspect, aperture, dist_to_focus, 0.0, 1.0);


    for j in (0..ny).rev() {
        for i in 0..nx {

            let mut col = Vec3::new_zero_vector();
            for s in 0..ns {
                let random: f64 = random::rand();
                let u: f64 = ((i as f64) + random) / (nx as f64);
                let random: f64 = random::rand();
                let v: f64 = ((j as f64) + random) / (ny as f64);

                let r = cam.get_ray(u, v);

                col += Vec3::new(u, v, 0.0);
            }

            col = col / ns as f64;
            col = Vec3::new(col.x().sqrt(), col.y().sqrt(), col.z().sqrt()); // Gamma correct 1/2.0

            let ir = (255.99*col.r()) as i32;
            let ig = (255.99*col.g()) as i32;
            let ib = (255.99*col.b()) as i32;

           if ( i + (ny - j) * nx) % 120 == 0 {
               print!("\rProgress: {} {}%", i + (ny - j) * nx, 100.0 * ((i+1) + ((ny - (j+1)) * nx)) as f64 / ((ny*nx) as f64));
           }

            write!(output_image, "{} {} {}\n", ir, ig, ib).unwrap();
        }
    }

    println!("Done..");
}

fn two_spheres() -> Vec<Box<Hitable>> {
    let red_texture = ConstantTexture::new(Vec3::new(1.0, 0.0, 0.0));
    let blue_texture = ConstantTexture::new(Vec3::new(0.0, 0.0, 1.0));

    let n = 50;

    let list: Vec<Box<Hitable>> = Vec::with_capacity(n);
    //list.push(

    list
}

fn color(r : &Ray, world: impl Hitable, depth: i32) -> Vec3 {

    world.hit(r);

    Vec3::new(0.0,0.0,0.0)

//
//   hit_record rec;
//   if(world->hit(r, 0.001, MAXFLOAT, rec))
//   {
//       ray scattered;
//       vec3 attenuation;
//       if(depth < 50 && rec.mat_ptr->scatter(r, rec, attenuation, scattered))
//       {
//           return attenuation*color(scattered, world, depth+1);
//       }
//       else
//       {
//           return vec3(0,0,0);
//       }
//   }
//   else
//   {
//       vec3 unit_direction = unit_vector(r.direction());
//       float t = 0.5*(unit_direction.y() + 1.0);
//       return (1.0-t)*vec3(1.0, 1.0, 1.0) + t*vec3(0.5, 0.7, 1.0);
}
