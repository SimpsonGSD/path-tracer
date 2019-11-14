
use math::*;
use material::Material;
use hitable::*;
use std::sync::Arc;
use std::f64::consts::{PI, FRAC_2_PI, FRAC_PI_2, FRAC_1_PI};

pub struct Rect {
    material: Arc<dyn Material + Send + Sync + 'static>,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    k: f64,
}

impl Rect {
    pub fn new(
        material: Arc<dyn Material + Send + Sync + 'static>,
        x0: f64, x1: f64, y0: f64, y1: f64, k: f64) 
    -> Self {
        Self {
            material,
            x0,
            x1, 
            y0,
            y1,
            k
        }
    }
}

impl Hitable for Rect {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        None
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        AABB::new(Vec3::new(self.x0, self.x1, self.k-0.0001), Vec3::new(self.x1, self.y2, self.k + 0.0001))
    }
}