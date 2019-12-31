use std::f64::consts::PI;
use math::*;
use crate::input;

#[allow(dead_code)]
pub struct Camera {
    origin: Vec3,
    look_at: Vec3,
    lower_left_corner: Vec3,
    horizontal: Vec3,
    vertical: Vec3,
    u: Vec3,
    v: Vec3, 
    w: Vec3,
    time0: f64,
    time1: f64,
    lens_radius: f64,
    half_width: f64,
    half_height: f64, 
    focus_dist: f64,
    world_up: Vec3
}

impl Camera {
    pub fn new(origin: Vec3, look_at: Vec3, vup: Vec3, vfov: f64, aspect: f64, 
               aperture: f64, focus_dist: f64, time0: f64, time1: f64) -> Camera {
        
        let theta = vfov * PI / 180.0;
        let half_height = (theta/2.0).tan();
        let half_width = aspect * half_height;
        let w = Vec3::new_unit_vector(&(origin - look_at)); // TODO(SS): This produces negative forward vector..
        let u = Vec3::new_unit_vector(&vec3::cross(&vup, &w));
        let v = vec3::cross(&w,&u);
        Camera {
            origin: origin.clone(),
            look_at: look_at.clone(),
            lower_left_corner: origin - (u*half_width*focus_dist) - (v*half_height*focus_dist) - (w*focus_dist),
            horizontal: u*2.0*half_width*focus_dist,
            vertical: v*2.0*half_height*focus_dist,
            u,
            v,
            w,
            time0,
            time1,
            lens_radius: aperture / 2.0,
            half_width,
            half_height, 
            focus_dist,
            world_up: vup
        }
    }

    pub fn get_ray(&self, s: f64, t: f64) -> Ray {
        let rd = random_in_unit_disk()*self.lens_radius;
        let offset = &self.u*rd.x + &self.v*rd.y;
        let time = self.time0 + random::rand()*(self.time1 - self.time0);
        let direction = self.lower_left_corner + self.horizontal*s + self.vertical*t - self.origin - offset;
        Ray::new(&self.origin + &offset, direction, time)
    }

    pub fn get_forward(&self) -> Vec3 {
        -self.w.clone()
    }

    pub fn get_up(&self) -> Vec3 {
        self.v.clone()
    }

    pub fn get_right(&self) -> Vec3 {
        self.u.clone()
    }

    pub fn get_origin(&self) -> Vec3 {
        self.origin.clone()
    }

    pub fn set_origin(&mut self, origin: Vec3, update_look_at: bool) {
        if update_look_at {
            self.look_at += &origin - &self.origin;
        }
        self.origin = origin;
        self.lower_left_corner = &self.origin - &(&self.u*self.half_width*self.focus_dist) - &(&self.v*self.half_height*self.focus_dist) - &(&self.w*self.focus_dist)
    }

    pub fn get_look_at(&self) -> Vec3 {
        self.look_at.clone()
    }

    pub fn update(&mut self) {
        self.w = Vec3::new_unit_vector(&(&self.origin - &self.look_at));
        self.u = Vec3::new_unit_vector(&vec3::cross(&self.world_up, &self.w));
        self.v = vec3::cross(&self.w, &self.u);
        self.lower_left_corner = &self.origin - &(&self.u*self.half_width*self.focus_dist) - &(&self.v*self.half_height*self.focus_dist) - &(&self.w*self.focus_dist);
        self.horizontal = &self.u*2.0*self.half_width*self.focus_dist;
        self.vertical = &self.v*2.0*self.half_height*self.focus_dist;
    }

    pub fn set_look_at(&mut self, look_at: Vec3, maintain_distance: bool) {
        let mut look_at = look_at;
        if maintain_distance {
             // minimise drift - ensure new look is same distance apart
            let look_at_dist = (&self.origin - &self.look_at).length();
            let new_look_at_dist = (&self.origin - &look_at).length();
            look_at *= look_at_dist / new_look_at_dist;
        }
        self.look_at = look_at;
    }
    
    pub fn update_from_input(
        &mut self, 
        user_input: &input::UserInput, 
        frame_time: f64) 
    -> bool {
        use winit::event::*;

        const CAM_SPEED: f64 = 40.0;
        const MOUSE_LOOK_SPEED: f64 = 1.0;

        let mut camera_moved = false;

        if user_input.keys_pressed.contains(&VirtualKeyCode::W) {
            let cam_origin = self.get_origin();
            let cam_forward = self.get_forward();
            let diff = cam_forward * CAM_SPEED * frame_time;
            self.set_origin(cam_origin + &diff, true);
            camera_moved = true;
        } 

        if user_input.keys_pressed.contains(&VirtualKeyCode::S) {
            let cam_origin = self.get_origin();
            let cam_forward = self.get_forward();
            let diff = -cam_forward * CAM_SPEED * frame_time;
            self.set_origin(cam_origin + &diff, true);
            camera_moved = true;
        }

        if user_input.keys_pressed.contains(&VirtualKeyCode::D) {
            let cam_origin = self.get_origin();
            let cam_right = self.get_right();
            let diff = cam_right * CAM_SPEED * frame_time;
            self.set_origin(cam_origin + &diff, true);
            camera_moved = true;
            
        }

        if user_input.keys_pressed.contains(&VirtualKeyCode::A) {
            let cam_origin = self.get_origin();
            let cam_right = self.get_right();
            let diff = -cam_right * CAM_SPEED * frame_time;
            self.set_origin(cam_origin + &diff, true);
            camera_moved = true;
        }

        if user_input.keys_pressed.contains(&VirtualKeyCode::E) {
            let cam_origin = self.get_origin();
            let cam_up = self.get_up();
            let diff = cam_up * CAM_SPEED * frame_time;
            self.set_origin(cam_origin + &diff, true);
            camera_moved = true;
        }

        if user_input.keys_pressed.contains(&VirtualKeyCode::Q) {
            let cam_origin = self.get_origin();
            let cam_up = self.get_up();
            let diff = -cam_up * CAM_SPEED * frame_time;
            self.set_origin(cam_origin + &diff, true);
            camera_moved = true;
        }
        
        if user_input.keys_held.contains(&VirtualKeyCode::Right) {
            let cam_look_at = self.get_look_at();
            let cam_right = self.get_right();
            self.set_look_at(cam_look_at + cam_right * CAM_SPEED * frame_time, true);
            camera_moved = true;
        }
        if user_input.keys_held.contains(&VirtualKeyCode::Left) {
            let cam_look_at = self.get_look_at();
            let cam_right = self.get_right();
            self.set_look_at(cam_look_at + -cam_right * CAM_SPEED * frame_time, true);
            camera_moved = true;
        }
        if user_input.keys_held.contains(&VirtualKeyCode::Up) {
            let cam_look_at = self.get_look_at();
            let cam_up = self.get_up();
            self.set_look_at(cam_look_at + cam_up * CAM_SPEED * frame_time, true);
            camera_moved = true;
        }
        if user_input.keys_held.contains(&VirtualKeyCode::Down) {
            let cam_look_at = self.get_look_at();
            let cam_up = self.get_up();
            self.set_look_at(cam_look_at + -cam_up * CAM_SPEED * frame_time, true);
            camera_moved = true;
        }
        if user_input.mouse_delta != (0.0,0.0) {
            let mouse_x_delta = user_input.mouse_delta.0;
            let mouse_y_delta = user_input.mouse_delta.1;
            if mouse_x_delta != 0.0 || mouse_y_delta != 0.0
            { 
                let mut cam_look_at = self.get_look_at();
                let cam_right = self.get_right();
                let cam_up = self.get_up();
                if mouse_x_delta != 0.0 {
                    cam_look_at += cam_right * MOUSE_LOOK_SPEED * frame_time * mouse_x_delta
                }
                if mouse_y_delta != 0.0 {
                    cam_look_at += cam_up * MOUSE_LOOK_SPEED * frame_time * mouse_y_delta;
                }

                self.set_look_at(cam_look_at, true);
                camera_moved = true;
            }
        }

        camera_moved
    }

    //pub fn get_look
}


fn random_in_unit_disk() -> Vec3 {
    let mut new_vector = Vec3::new(random::rand(), random::rand(), 0.0)*2.0 - Vec3::new(1.0,1.0,0.0);
    while vec3::dot(&new_vector,&new_vector) >= 1.0 {
        new_vector = Vec3::new(random::rand(), random::rand(), 0.0)*2.0 - Vec3::new(1.0,1.0,0.0);
    } 

    new_vector
}
