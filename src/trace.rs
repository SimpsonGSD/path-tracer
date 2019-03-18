use std::f64;
use std::sync::{Mutex, Arc, RwLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::rc::Rc;

use math::*;
use hitable::*;
use camera::Camera;
use winit_utils::*;

use jobs::JobTask;

type LockableImageBuffer = Arc<Mutex<Vec<u8>>>;
type ThreadsafeCounter = Arc<AtomicUsize>;

// Number of lines to wait before updating the backbuffer. Smaller the number worse the performance.
const RENDER_UPDATE_LATENCY: u32 = 20; 
pub const REALTIME: bool = true;
const ENABLE_RENDER: bool = true && !REALTIME;

pub struct SharedSceneWriteState {
    pub buffer: Mutex<Vec<u8>>,
    pub window_lock: AtomicBool, 
    pub remaining_tasks: AtomicUsize,
}

impl SharedSceneWriteState {
    pub fn new(buffer: Mutex<Vec<u8>>, remaining_tasks: AtomicUsize, window_lock: AtomicBool) -> SharedSceneWriteState {
            
        SharedSceneWriteState {
            buffer,
            window_lock,
            remaining_tasks,
        }
    }
}

pub struct SharedSceneReadState {
    pub cam: Camera,
    pub world: Box<Hitable + Send + Sync + 'static>,
    pub window: winit::Window,
}

impl SharedSceneReadState {
    pub fn new(cam: Camera, world: Box<Hitable + Send + Sync + 'static>, window: winit::Window) -> SharedSceneReadState {
            
        SharedSceneReadState {
            cam,
            world,
            window,
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
    local_buffer_f32: Vec<f32>,
    shared_scene_read_state: Arc<RwLock<SharedSceneReadState>>,
    shared_scene_write_state: Arc<SharedSceneWriteState>,
}

impl TraceSceneBatchJob {
    pub fn new(num_samples: u32, start_xy: (u32, u32), end_xy: (u32, u32), 
               image_size: (u32, u32), shared_scene_read_state: Arc<RwLock<SharedSceneReadState>>, shared_scene_write_state: Arc<SharedSceneWriteState>) -> TraceSceneBatchJob {
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
            local_buffer_f32: if REALTIME {vec![0.0; (num_pixels_xy.0*num_pixels_xy.1*3) as usize]} else {vec![]},
            shared_scene_read_state,
            shared_scene_write_state
        }
    }

    pub fn clear_buffer(&mut self) {
        if REALTIME {
            self.local_buffer_f32 = vec![0.0; (self.num_pixels_xy.0*self.num_pixels_xy.1*3) as usize]
        } else {
            self.local_buffer_u8 = vec![0; (self.num_pixels_xy.0*self.num_pixels_xy.1*3) as usize]
        }
    }

    fn trace(&mut self) {

        let update_window_and_release_lock = |buffer: &mut Vec<u8>, window: &winit::Window, image_start_xy: (u32,u32), num_pixels_xy: (u32,u32), window_lock: &AtomicBool| {
            // TODO(SS): Optimise so we are only copying the changed buffer parts
            update_window_framebuffer_rect(&window, buffer, image_start_xy, num_pixels_xy);
            window_lock.store(false, Ordering::Release);
        };

        let read_state = self.shared_scene_read_state.read().unwrap();

        for j in (self.start_xy.1..self.end_xy.1).rev() {

            let stride = (self.num_pixels_xy.0 * 3) as usize;
            let row_offset = stride * (j - self.start_xy.1) as usize;
            let mut buffer_offset = row_offset;

            for i in self.start_xy.0..self.end_xy.0 {
                let mut col = Vec3::new_zero_vector();
                for _ in 0..self.num_samples {
                    let random = random::rand();
                    let u: f64 = ((i as f64) + random) / (self.image_size.0 as f64);
                    let random = random::rand();
                    let v: f64 = ((j as f64) + random) / (self.image_size.1 as f64);

                    let r = read_state.cam.get_ray(u, v);
                    col += color(&r, &read_state.world, 0);

                    // SS: Debug uv image
                    // col += Vec3::new(u, v, 0.0);
                }

                col = col / self.num_samples as f64;

                const WEIGHT: f32 = 1.0;
                const ONE_MINUS_WEIGHT: f32 = 1.0 - WEIGHT;

                if REALTIME {
                    self.local_buffer_f32[buffer_offset]  = (col.z as f32) * WEIGHT + self.local_buffer_f32[buffer_offset] * ONE_MINUS_WEIGHT;
                    buffer_offset += 1;
                    self.local_buffer_f32[buffer_offset]  = (col.y as f32) * WEIGHT + self.local_buffer_f32[buffer_offset] * ONE_MINUS_WEIGHT;
                    buffer_offset += 1;
                    self.local_buffer_f32[buffer_offset]  = (col.x as f32) * WEIGHT + self.local_buffer_f32[buffer_offset] * ONE_MINUS_WEIGHT;
                    buffer_offset += 1;
                } else {
                    col = Vec3::new(col.x.sqrt(), col.y.sqrt(), col.z.sqrt()); // Gamma correct 1/2.0

                    let ir = (255.99*col.r()) as u8;
                    let ig = (255.99*col.g()) as u8;
                    let ib = (255.99*col.b()) as u8;
                    
                    self.local_buffer_u8[buffer_offset]  = ib;
                    buffer_offset += 1;
                    self.local_buffer_u8[buffer_offset]  = ig;
                    buffer_offset += 1;
                    self.local_buffer_u8[buffer_offset]  = ir;
                    buffer_offset += 1;
                }
            }

            // copy 1 row of our local buffer into correct slice of image buffer.
            {
                let u8_buffer: Vec<u8>;
                let src_buffer;
                if REALTIME { 
                    // for now gamma correct and convert to u8.
                    // TODO(SS): This will be done in shader
                    u8_buffer = self.local_buffer_f32[row_offset..row_offset + stride].iter().map(|x| {
                        (255.99*x.sqrt()) as u8
                    }).collect();
                   src_buffer = &u8_buffer[..];
                } else {
                    src_buffer = &self.local_buffer_u8[row_offset..row_offset + stride]
                }
                let mut buffer_mutex = self.shared_scene_write_state.buffer.lock().unwrap();
                let start = (self.start_xy.0 * 3 + j * self.image_size.0 * 3) as usize;
                let dest_buffer = &mut buffer_mutex[start..start + stride];
                dest_buffer.copy_from_slice(src_buffer);
            } // buffer_mutex is released here 

            if ENABLE_RENDER && j % RENDER_UPDATE_LATENCY == 0 && self.shared_scene_write_state.window_lock.compare_and_swap(false, true, Ordering::Acquire)  {
                // Update frame buffer to show progress
                update_window_and_release_lock(&mut self.local_buffer_u8, &read_state.window, self.image_start_xy, self.num_pixels_xy, &self.shared_scene_write_state.window_lock);
            }
        }

        if ENABLE_RENDER {
            while self.shared_scene_write_state.window_lock.compare_and_swap(false, true, Ordering::Acquire)  {
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

fn color(r : &Ray, world: &Box<Hitable + Send + Sync + 'static>, depth: i32) -> Vec3 {
    if let Some(hit_record) = world.hit(r, 0.001, f64::MAX) {
        if depth < 50 {
            if  let Some((scattered, attenuation)) =  hit_record.mat.scatter(r, &hit_record) {
                return attenuation * color(&scattered, world, depth+1);
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