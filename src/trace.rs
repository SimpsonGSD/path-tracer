use std::f64;
use std::sync::Arc;
use parking_lot::{RwLock};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use math::*;
use hitable::*;
use camera::Camera;

use jobs::JobTask;
use jobs::MultiSliceReadWriteLock;
use super::Config;

// Number of lines to wait before updating the backbuffer. Smaller the number worse the performance.
const RENDER_UPDATE_LATENCY: u32 = 20; 
const ENABLE_RENDER: bool = true;
const CHANCE_TO_SKIP_TASK_PER_FRAME: f64 = 0.0;
const CHANCE_TO_SKIP_PIXEL_PER_FRAME: f64 = 0.8;

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

    pub fn notify_task_completion(&self) {
        self.remaining_tasks.fetch_sub(1, Ordering::SeqCst);
    }
}


pub struct SceneState {
    pub cam: Camera,
    pub world: Box<dyn Hitable + Send + Sync + 'static>,
    pub window: winit::window::Window,
    pub time0: f64,
    pub time1: f64,
    pub sky_brightness: f64,
    pub disable_emissive: bool,
    pub config: Config,
}

impl SceneState {
    pub fn new(cam: Camera, world: Box<dyn Hitable + Send + Sync + 'static>, window: winit::window::Window, time0: f64, time1: f64, 
               sky_brightness: f64, disable_emissive: bool, config: Config) -> SceneState {
            
        SceneState {
            cam,
            world,
            window,
            time0,
            time1,
            sky_brightness,
            disable_emissive,
            config
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
    shared_scene_read_state: Arc<RwLock<SceneState>>,
    shared_scene_write_state: Arc<SceneOutput>,
    num_frames: i32,
    num_frames_per_pixel: Vec<u32>,
    realtime: bool,
}

impl TraceSceneBatchJob {
    pub fn new(
        num_samples: u32, 
        start_xy: (u32, u32), 
        end_xy: (u32, u32), 
        image_size: (u32, u32), 
        shared_scene_read_state: Arc<RwLock<SceneState>>, 
        shared_scene_write_state: Arc<SceneOutput>,
        realtime: bool) -> TraceSceneBatchJob {

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
            shared_scene_read_state,
            shared_scene_write_state,
            num_frames: 0,
            num_frames_per_pixel: vec![0; (num_pixels_xy.0*num_pixels_xy.1) as usize],
            realtime,
        }
    }

    pub fn clear_buffer(&mut self) {
        self.num_frames = 0;
        self.num_frames_per_pixel = vec![0; (self.num_pixels_xy.0*self.num_pixels_xy.1) as usize];
    }

    fn trace(&mut self) {

        //self.num_frames += if self.num_frames == 500 {0} else {1};
        self.num_frames += 1;//if self.num_frames == 500 {0} else {1};
        let read_state = self.shared_scene_read_state.read();

        if read_state.config.realtime && random::rand() < CHANCE_TO_SKIP_TASK_PER_FRAME {
            self.shared_scene_write_state.notify_task_completion();
            return;
        }

        for (row_idx, j) in (self.start_xy.1..self.end_xy.1).rev().enumerate() {

            let stride = (self.num_pixels_xy.0 * 4) as usize;

            let start = (self.start_xy.0 * 4 + j * self.image_size.0 * 4) as usize;
            let dest_buffer_row_slice = &mut self.shared_scene_write_state.buffer.write()[start..start + stride];

            for (col_idx, i) in (self.start_xy.0..self.end_xy.0).enumerate() {

               if read_state.config.realtime && random::rand() < CHANCE_TO_SKIP_PIXEL_PER_FRAME {
                   continue;
               }

                let local_pixel_idx = row_idx * self.num_pixels_xy.0 as usize + col_idx;
                self.num_frames_per_pixel[local_pixel_idx] += if self.num_frames_per_pixel[local_pixel_idx] <= 1000 {1} else {0};

                let mut pixel_colour = Vec3::new_zero_vector();
                for _ in 0..self.num_samples {
                    let random = random::rand();
                    let u: f64 = ((i as f64) + random) / (self.image_size.0 as f64);
                    let random = random::rand();
                    let v: f64 = ((j as f64) + random) / (self.image_size.1 as f64);

                    let r = read_state.cam.get_ray(u, v);
                    pixel_colour += color(&r, &read_state.world, 0, read_state.time0, read_state.time1, read_state.sky_brightness, read_state.disable_emissive, read_state.config.max_depth);

                    // SS: Debug uv image
                    // col += Vec3::new(u, v, 0.0);
                }

                // PDF
                pixel_colour = pixel_colour / self.num_samples as f64;

                let index = col_idx*4 as usize;

                let num_frames = self.num_frames_per_pixel[local_pixel_idx];
                let weight = 1.0 / num_frames as f32;
                let one_minus_weight: f32 = 1.0 - weight;

                dest_buffer_row_slice[index]     = (pixel_colour.x as f32) * weight + dest_buffer_row_slice[index    ] * one_minus_weight;
                dest_buffer_row_slice[index + 1] = (pixel_colour.y as f32) * weight + dest_buffer_row_slice[index + 1] * one_minus_weight;
                dest_buffer_row_slice[index + 2] = (pixel_colour.z as f32) * weight + dest_buffer_row_slice[index + 2] * one_minus_weight;

            }
        }

        // notify completion by decrementing task counter
        self.shared_scene_write_state.notify_task_completion();
    }
}

impl JobTask for TraceSceneBatchJob {
    fn run(&mut self) {
        self.trace();
    }
}

fn color(r : &Ray, world: &Box<dyn Hitable + Send + Sync + 'static>, depth: i32, _t_min: f64, _t_max: f64, sky_brightness: f64, disable_emissive: bool, 
         max_depth: i32) -> Vec3 {
    if let Some(hit_record) = world.hit(r, 0.001, f64::MAX) {
        let mut colour = if !disable_emissive {hit_record.mat.emitted(hit_record.u, hit_record.v, &hit_record.p)} else {Vec3::from_float(0.0)};
        if depth < max_depth {
            if  let Some(scatter_result) =  hit_record.mat.scatter(r, &hit_record) {
                colour += scatter_result.attenuation * color(&scatter_result.scattered, world, depth+1, 0.001, f64::MAX, sky_brightness, disable_emissive, max_depth);
            }
        }
        return colour;
    } else {
        let unit_dir = Vec3::new_unit_vector(&r.direction());
        let t = 0.5*(unit_dir.y + 1.0);
        let white = Vec3::from_float(1.0);
        //let sky = Vec3::new(135.0/255.0, 206.0/255.0, 235.0/255.0);
        let sky = Vec3::new(0.5, 0.7, 1.0);
        return lerp(&white, &sky, t) * sky_brightness;
        //return Vec3::new(0.0, 0.0, 0.0);
    }
}

pub fn reinhard_tonemap(colour: &Vec3) -> Vec3 {
    let _luminance: Vec3 = Vec3::new(0.2126, 0.7152, 0.0722);
    static EXPOSURE: f64 = 1.5;
    let colour = colour * EXPOSURE;
    //&colour / (vec3::dot(&colour, &luminance) + 1.0)
    &colour / (&colour + 1.0)
}