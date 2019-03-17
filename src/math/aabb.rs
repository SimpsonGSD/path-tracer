use super::vec3::Vec3;
use super::ray::Ray;
use std::mem;

#[derive(Clone)]
pub struct AABB {
    min: Vec3,
    max: Vec3
}

impl AABB {
    pub fn new(min: Vec3, max: Vec3) -> AABB {
        AABB {
            min,
            max
        }
    }

    pub fn min(&self) -> &Vec3 {
        &self.min
    }

    pub fn max(&self) -> &Vec3 {
        &self.max
    }

    pub fn hit(&self, r: &Ray, tmin: f64, tmax: f64) -> bool {

        let mut tmin = tmin;
        let mut tmax = tmax;

        for i in 0..3 {
            let inv_d = 1.0 / r.direction()[i];
            let mut t0 = (self.min()[i] - r.origin()[i]) * inv_d;
            let mut t1 = (self.max()[i] - r.origin()[i]) * inv_d;
            if inv_d < 0.0 {
                mem::swap(&mut t0, &mut t1);
            }
            tmin = if t0 > tmin {t0} else {tmin};
            tmax = if t1 < tmax {t1} else {tmax};
            if tmax <= tmin {
                return false;
            }
        }

        true
    }

    pub fn get_union(box0: &AABB, box1: &AABB) -> AABB {
        AABB::new( Vec3::new(   ffmin(box0.min().x, box1.min().x),
                                ffmin(box0.min().y, box1.min().y),
                                ffmin(box0.min().z, box1.min().z)),
                    Vec3::new(  ffmax(box0.max().x, box1.max().x),
                                ffmax(box0.max().y, box1.max().y),
                                ffmax(box0.max().z, box1.max().z)))             
    }
}



fn ffmax(a: f64, b :f64) -> f64 {
    if a > b {a} else {b}
}

fn ffmin(a: f64, b :f64) -> f64 {
    if a < b {a} else {b}
}