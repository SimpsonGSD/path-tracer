use std::f64;
use std::sync::Arc;
use parking_lot::{RwLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use math::*;
use hitable::*;
use camera::Camera;
use winit_utils::*;

use jobs::JobTask;
use jobs::MultiSliceReadWriteLock;

// Number of lines to wait before updating the backbuffer. Smaller the number worse the performance.
const RENDER_UPDATE_LATENCY: u32 = 20; 
pub const REALTIME: bool = true;
const ENABLE_RENDER: bool = true && !REALTIME;

pub struct SceneOutput {
    pub buffer: MultiSliceReadWriteLock<Vec<f32>>,
    pub window_lock: AtomicBool, 
    pub remaining_tasks: AtomicUsize,
}

impl SceneOutput {
    pub fn new(buffer: MultiSliceReadWriteLock<Vec<f32>>, remaining_tasks: AtomicUsize, window_lock: AtomicBool) -> SceneOutput {
            
        SceneOutput {
            buffer,
            window_lock,
            remaining_tasks,
        }
    }
}

pub struct SceneState {
    pub cam: Camera,
    pub world: Box<Hitable + Send + Sync + 'static>,
    pub window: winit::Window,
    pub time0: f64,
    pub time1: f64,
}

impl SceneState {
    pub fn new(cam: Camera, world: Box<Hitable + Send + Sync + 'static>, window: winit::Window, time0: f64, time1: f64) -> SceneState {
            
        SceneState {
            cam,
            world,
            window,
            time0,
            time1
        }
    }
}

pub struct TraceSceneBatchJob {
    num_samples: u32,
    start_xy: (u32, u32),
    end_xy: (u32, u32),
    image_size: (u32, u32),
    num_pixels_xy: (u32, u32),
    image_start_xy: (u32, u32),
    local_buffer_u8: Vec<u8>,
    shared_scene_read_state: Arc<RwLock<SceneState>>,
    shared_scene_write_state: Arc<SceneOutput>,
    num_frames: i32,
}

impl TraceSceneBatchJob {
    pub fn new(num_samples: u32, start_xy: (u32, u32), end_xy: (u32, u32), 
               image_size: (u32, u32), shared_scene_read_state: Arc<RwLock<SceneState>>, shared_scene_write_state: Arc<SceneOutput>) -> TraceSceneBatchJob {
        let num_pixels_xy = (end_xy.0 - start_xy.0, end_xy.1 - start_xy.1);
        // the window and image buffer start with 0 at the top not the bottom so we must convert here.
        let image_start_xy = (start_xy.0, image_size.1 - start_xy.1 - num_pixels_xy.1);
        TraceSceneBatchJob {
            num_samples,
            start_xy,
            end_xy,
            image_size,
            num_pixels_xy,
            image_start_xy,
            local_buffer_u8: if !REALTIME {vec![0; (num_pixels_xy.0*num_pixels_xy.1*3) as usize]} else {vec![]},
            shared_scene_read_state,
            shared_scene_write_state,
            num_frames: 0
        }
    }

    pub fn clear_buffer(&mut self) {
        self.num_frames = 0;
        if !REALTIME {
            self.local_buffer_u8 = vec![0; (self.num_pixels_xy.0*self.num_pixels_xy.1*3) as usize]
        } 
        //else {
       //     self.local_buffer_f32 = vec![0.0; (self.num_pixels_xy.0*self.num_pixels_xy.1*3) as usize]
       // }
    }

    fn trace(&mut self) {

        let update_window_and_release_lock = |buffer: &mut Vec<u8>, window: &winit::Window, image_start_xy: (u32,u32), num_pixels_xy: (u32,u32), window_lock: &AtomicBool| {
            // TODO(SS): Optimise so we are only copying the changed buffer parts
            update_window_framebuffer_rect(&window, buffer, image_start_xy, num_pixels_xy);
            window_lock.store(false, Ordering::Release);
        };

        self.num_frames += if self.num_frames == 500 {0} else {1};
        let read_state = self.shared_scene_read_state.read();

        for j in (self.start_xy.1..self.end_xy.1).rev() {

            let stride = (self.num_pixels_xy.0 * 3) as usize;

            let start = (self.start_xy.0 * 3 + j * self.image_size.0 * 3) as usize;
            let dest_buffer_row_slice = &mut self.shared_scene_write_state.buffer.write()[start..start + stride];

            let row_offset = stride * (j - self.start_xy.1) as usize;
            let mut buffer_offset = row_offset;

            for (idx, i) in (self.start_xy.0..self.end_xy.0).enumerate() {

                // TODO(SS): Use random sampling - Not working properly
                //if !REALTIME {
                //    if random::rand() > 0.5 {
                //        continue;
                //    }
                //}

                let mut col = Vec3::new_zero_vector();
                for _ in 0..self.num_samples {
                    let random = random::rand();
                    let u: f64 = ((i as f64) + random) / (self.image_size.0 as f64);
                    let random = random::rand();
                    let v: f64 = ((j as f64) + random) / (self.image_size.1 as f64);

                    let r = read_state.cam.get_ray(u, v);
                    col += color(&r, &read_state.world, 0, read_state.time0, read_state.time1);

                    // SS: Debug uv image
                    // col += Vec3::new(u, v, 0.0);
                }

                col = col / self.num_samples as f64;

                let weight = 1.0 / self.num_frames as f32;
                let one_minus_weight: f32 = 1.0 - weight;

                let index = idx*3 as usize;
                if REALTIME {
                    dest_buffer_row_slice[index]     = (col.z as f32) * weight + dest_buffer_row_slice[index    ] * one_minus_weight;
                    dest_buffer_row_slice[index + 1] = (col.y as f32) * weight + dest_buffer_row_slice[index + 1] * one_minus_weight;
                    dest_buffer_row_slice[index + 2] = (col.x as f32) * weight + dest_buffer_row_slice[index + 2] * one_minus_weight;
                } else {

                    // Gamma correct 1/2.0 and convert to u8
                    let ir = (255.99*col.x.sqrt()) as u8;
                    let ig = (255.99*col.y.sqrt()) as u8;
                    let ib = (255.99*col.z.sqrt()) as u8;
                    
                    self.local_buffer_u8[buffer_offset]  = ib;
                    buffer_offset += 1;
                    self.local_buffer_u8[buffer_offset]  = ig;
                    buffer_offset += 1;
                    self.local_buffer_u8[buffer_offset]  = ir;
                    buffer_offset += 1;

                    dest_buffer_row_slice[index] = col.b() as f32;
                    dest_buffer_row_slice[index + 1] = col.g() as f32;
                    dest_buffer_row_slice[index + 2] = col.r() as f32;
                }
            }

            if ENABLE_RENDER && j % RENDER_UPDATE_LATENCY == 0 && self.shared_scene_write_state.window_lock.compare_and_swap(false, true, Ordering::Acquire) {
                // Update frame buffer to show progress
                update_window_and_release_lock(&mut self.local_buffer_u8, &read_state.window, self.image_start_xy, self.num_pixels_xy, &self.shared_scene_write_state.window_lock);
            }
        }

        if ENABLE_RENDER {
            while self.shared_scene_write_state.window_lock.compare_and_swap(false, true, Ordering::Acquire) {
                // Update frame buffer to show progress
                update_window_and_release_lock(&mut self.local_buffer_u8, &read_state.window, self.image_start_xy, self.num_pixels_xy, &self.shared_scene_write_state.window_lock);
            }
        }

        // notify completion by decrementing task counter
        self.shared_scene_write_state.remaining_tasks.fetch_sub(1, Ordering::SeqCst);
    }
}

impl JobTask for TraceSceneBatchJob {
    fn run(&mut self) {
        self.trace();
    }
}

fn color(r : &Ray, world: &Box<Hitable + Send + Sync + 'static>, depth: i32, t_min: f64, t_max: f64) -> Vec3 {
    if let Some(hit_record) = world.hit(r, 0.001, f64::MAX) {
        if depth < 50 {
            if  let Some((scattered, attenuation)) =  hit_record.mat.scatter(r, &hit_record) {
                return attenuation * color(&scattered, world, depth+1, 0.001, f64::MAX);
            }
        }
        return Vec3::new_zero_vector();
    } else {
        let unit_dir = Vec3::new_unit_vector(&r.direction());
        let t = 0.5*(unit_dir.y + 1.0);
        let white = Vec3::from_float(1.0);
        let sky = Vec3::new(0.5, 0.7, 1.0);
        return lerp(&white, &sky, t);
    }
}