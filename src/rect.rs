
use math::*;
use material::Material;
use hitable::*;
use std::sync::Arc;
use std::f64::consts::{FRAC_2_PI, FRAC_PI_2, FRAC_1_PI};

pub struct RectXY {
    material: Arc<dyn Material + Send + Sync + 'static>,
    x0: f64,
    x1: f64,
    y0: f64,
    y1: f64,
    k: f64,
    x_size: f64,
    y_size: f64
}

impl RectXY {
    pub fn new(
        x0: f64, x1: f64, y0: f64, y1: f64, k: f64,
        material: Arc<dyn Material + Send + Sync + 'static>) 
    -> Self {
        Self {
            material,
            x0,
            x1, 
            y0,
            y1,
            k,
            x_size: x1 - x0,
            y_size: y1 - y0,
        }
    }
}

impl Hitable for RectXY {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let t = (self.k - ray.origin().z) / ray.direction().z;
        if t < t_min || t > t_max {
            return None;
        }
        let x = ray.origin().x + t*ray.direction().x;
        let y = ray.origin().y + t*ray.direction().y;
        if x < self.x0 || x > self.x1 || y < self.y0 || y > self.y1 {
            return None;
        }
        Some(HitRecord::new(
            t, 
            (x - self.x0) / self.x_size,
            (y - self.y0) / self.y_size,
            ray.point_at_parameter(t),
            Vec3::new(0.0, 0.0, 1.0),
            self.material.clone()
        ))
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        AABB::new(Vec3::new(self.x0, self.y0, self.k-0.0001), Vec3::new(self.x1, self.y1, self.k + 0.0001))
    }
}