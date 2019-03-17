use std::f64::consts::PI;
use math::*;

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
}

impl Camera {
    pub fn new(origin: Vec3, look_at: Vec3, vup: Vec3, vfov: f64, aspect: f64, 
               aperture: f64, focus_dist: f64, time0: f64, time1: f64) -> Camera {
        
        let theta = vfov * PI / 180.0;
        let half_height = (theta/2.0).tan();
        let half_width = aspect * half_height;
        let w = Vec3::new_unit_vector(&(&origin - &look_at)); // TODO(SS): This produces negative forward vector..
        let u = Vec3::new_unit_vector(&vec3::cross(&vup, &w));
        let v = vec3::cross(&w,&u);
        Camera {
            origin: origin.clone(),
            look_at: look_at.clone(),
            lower_left_corner: &origin - &(&u*half_width*focus_dist) - &(&v*half_height*focus_dist) - &(&w*focus_dist),
            horizontal: &u*2.0*half_width*focus_dist,
            vertical: &v*2.0*half_height*focus_dist,
            u,
            v,
            w,
            time0,
            time1,
            lens_radius: aperture / 2.0,
            half_width,
            half_height, 
            focus_dist,
        }
    }

    pub fn get_ray(&self, s: f64, t: f64) -> Ray {
        let rd = random_in_unit_disk()*self.lens_radius;
        let offset = &self.u*rd.x + &self.v*rd.y;
        let time = self.time0 + random::rand()*(self.time1 - self.time0);
        Ray::new(&self.origin + &offset, &self.lower_left_corner + &self.horizontal*s + &self.vertical*t - &self.origin - offset, time)
    }

    fn calc_lower_left_corner(&self) -> Vec3 {
         &self.origin - &(&self.u*self.half_width*self.focus_dist) - &(&self.v*self.half_height*self.focus_dist) - &(&self.w*self.focus_dist)
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

    pub fn set_origin(&mut self, origin: Vec3) {
        self.origin = origin;
        self.lower_left_corner = self.calc_lower_left_corner();
    }

    pub fn rotate_right(&mut self, delta: f64) {
        //let radius = (&self.origin - &self.lookat).length();
        //self.lookat.x += delta.sin() * radius;
    }

    pub fn get_look_at(&self) -> Vec3 {
        self.look_at.clone()
    }

    pub fn set_look_at(&mut self, look_at: Vec3) {
        self.look_at = look_at;
        self.w = Vec3::new_unit_vector(&(&self.origin - &self.look_at));
        self.u = Vec3::new_unit_vector(&vec3::cross(&self.v, &self.w));
        self.v = vec3::cross(&self.w, &self.u);
        self.lower_left_corner = self.calc_lower_left_corner();
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
