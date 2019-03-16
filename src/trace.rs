use std::f64;
use std::sync::{Mutex, Arc};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use math::*;
use hitable::*;
use camera::Camera;
use winit_utils::*;

use jobs::JobTask;

type LockableImageBuffer = Arc<Mutex<Vec<u8>>>;
type ThreadsafeCounter = Arc<AtomicUsize>;

// Number of lines to wait before updating the backbuffer. Smaller the number worse the performance.
const RENDER_UPDATE_LATENCY: u32 = 20; 

pub struct TraceSceneBatchJob {
    cam: Arc<Camera>,
    world: Arc<Hitable + Send + Sync + 'static>,
    num_samples: u32,
    start_xy: (u32, u32),
    end_xy: (u32, u32),
    buffer: LockableImageBuffer,
    image_size: (u32, u32),
    remaining_tasks: ThreadsafeCounter,
    window_lock: Arc<AtomicBool>, 
    window: Arc<winit::Window>,
}

impl TraceSceneBatchJob {
    pub fn new(cam: Arc<Camera>, world: Arc<Hitable + Send + Sync + 'static>, num_samples: u32, start_xy: (u32, u32), end_xy: (u32, u32), 
               buffer: LockableImageBuffer, image_size: (u32, u32), remaining_tasks: ThreadsafeCounter,
               window_lock: Arc<AtomicBool>, window: Arc<winit::Window>) -> TraceSceneBatchJob {
        TraceSceneBatchJob {
            cam,
            world,
            num_samples,
            start_xy,
            end_xy,
            buffer,
            image_size,
            remaining_tasks,
            window_lock,
            window
        }
    }

    fn trace(&self) {
        let num_pixels_xy = (self.end_xy.0 - self.start_xy.0, self.end_xy.1 - self.start_xy.1);
        let mut local_buffer = vec![0; (num_pixels_xy.0*num_pixels_xy.1*3) as usize];

        // the window and image buffer start with 0 at the top not the bottom so we must convert here.
        let image_start_xy = (self.start_xy.0, self.image_size.1 - self.start_xy.1 - num_pixels_xy.1);

        let update_window_and_release_lock = |buffer: &mut Vec<u8>| {
            // TODO(SS): Optimise so we are only copying the changed buffer parts
            update_window_framebuffer_rect(&self.window, buffer, image_start_xy, num_pixels_xy);
            self.window_lock.store(false, Ordering::Release);
        };

        for j in (self.start_xy.1..self.end_xy.1).rev() {

            let stride = (num_pixels_xy.0 * 3) as usize;
            let row_offset = stride * (j - self.start_xy.1) as usize;
            let mut buffer_offset = row_offset;

            for i in self.start_xy.0..self.end_xy.0 {
                let mut col = Vec3::new_zero_vector();
                for _ in 0..self.num_samples {
                    let random = random::rand();
                    let u: f64 = ((i as f64) + random) / (self.image_size.0 as f64);
                    let random = random::rand();
                    let v: f64 = ((j as f64) + random) / (self.image_size.1 as f64);

                    let r = self.cam.get_ray(u, v);
                    col += color(&r, &self.world, 0);

                    // SS: Debug uv image
                    // col += Vec3::new(u, v, 0.0);
                }

                col = col / self.num_samples as f64;
                col = Vec3::new(col.x().sqrt(), col.y().sqrt(), col.z().sqrt()); // Gamma correct 1/2.0

                let ir = (255.99*col.r()) as u8;
                let ig = (255.99*col.g()) as u8;
                let ib = (255.99*col.b()) as u8;
                
                local_buffer[buffer_offset]   = ib;
                buffer_offset += 1;
                local_buffer[buffer_offset]  = ig;
                buffer_offset += 1;
                local_buffer[buffer_offset]  = ir;
                buffer_offset += 1;
            }

            // copy 1 row of our local buffer into correct slice of image buffer.
            {
                let src_buffer = &local_buffer[row_offset..row_offset + stride];
                let mut buffer_mutex = self.buffer.lock().unwrap();
                let start = (self.start_xy.0 * 3 + j * self.image_size.0 * 3) as usize;
                let dest_buffer = &mut buffer_mutex[start..start + stride];
                dest_buffer.copy_from_slice(src_buffer);
            } // buffer_mutex is released here 

            if j % RENDER_UPDATE_LATENCY == 0 && self.window_lock.compare_and_swap(false, true, Ordering::Acquire)  {
                // Update frame buffer to show progress
                update_window_and_release_lock(&mut local_buffer);
            }
        }

        while self.window_lock.compare_and_swap(false, true, Ordering::Acquire)  {
            // Update frame buffer to show progress
            update_window_and_release_lock(&mut local_buffer);
        }

        // notify completion by decrementing task counter
        self.remaining_tasks.fetch_sub(1, Ordering::SeqCst);
    }
}

impl JobTask for TraceSceneBatchJob {
    fn run(&self) {
        self.trace();
    }
}

fn color(r : &Ray, world: &Arc<Hitable + Send + Sync + 'static>, depth: i32) -> Vec3 {
    if let Some(hit_record) = world.hit(r, 0.001, f64::MAX) {
        if depth < 50 {
            if  let Some((scattered, attenuation)) =  hit_record.mat.scatter(r, &hit_record) {
                return attenuation * color(&scattered, world, depth+1);
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