
use math::*;
use material::Material;
use hitable::*;
use std::sync::Arc;

pub enum AxisAlignedRectAxis {
    X,
    Y,
    Z,
}

pub struct AxisAlignedRect {
    material: Arc<dyn Material + Send + Sync + 'static>,
    amin: f64,
    amax: f64,
    bmin: f64,
    bmax: f64,
    c: f64, // corresponds to the axis denoted by plane_axis
    a_size: f64,
    b_size: f64,
    plane_axis: AxisAlignedRectAxis,
}

const FLT_TOLERANCE: f64 = 0.0000001;

impl AxisAlignedRect {
    pub fn new(
        amin: f64,
        amax: f64,
        bmin: f64,
        bmax: f64,
        c: f64,
        plane_axis: AxisAlignedRectAxis,
        material: Arc<dyn Material + Send + Sync + 'static>) 
    -> Self {

        if (amin - amax).abs() < FLT_TOLERANCE {
            panic!("amin != amax, no axis-aligned plane supplied. amin = {}, amax = {}", amin, amax);
        }
        if (bmin - bmax).abs() < FLT_TOLERANCE {
            panic!("bmin != bmax, no axis-aligned plane supplied, bmin = {}, bmax = {}", bmin, bmax);
        }

        Self {
            material,
            amin,
            amax, 
            bmin,
            bmax,
            c,
            a_size: amax - amin,
            b_size: bmax - bmin,
            plane_axis,
        }
    }

    pub fn get_plane_intersection(&self, ray: &Ray) -> f64 {
        match self.plane_axis {
            AxisAlignedRectAxis::X => (self.c - ray.origin().x) / ray.direction().x,
            AxisAlignedRectAxis::Y => (self.c - ray.origin().y) / ray.direction().y,
            AxisAlignedRectAxis::Z => (self.c - ray.origin().z) / ray.direction().z,
            _ => unimplemented!()
        }
    }

    pub fn get_ab_intersection(&self, ray: &Ray, t: f64) -> (f64, f64) {
        let a = match self.plane_axis {
            AxisAlignedRectAxis::X => ray.origin().y + t*ray.direction().y,
            AxisAlignedRectAxis::Y => ray.origin().x + t*ray.direction().x,
            AxisAlignedRectAxis::Z => ray.origin().x + t*ray.direction().x,
            _ => unimplemented!()
        };

        let b = match self.plane_axis {
            AxisAlignedRectAxis::X => ray.origin().z + t*ray.direction().z,
            AxisAlignedRectAxis::Y => ray.origin().z + t*ray.direction().z,
            AxisAlignedRectAxis::Z => ray.origin().y + t*ray.direction().y,
            _ => unimplemented!()
        };

        (a, b)
    }

    pub fn get_plane_normal(&self) -> Vec3 {
        match self.plane_axis {
            AxisAlignedRectAxis::X => Vec3::new(1.0,0.0,0.0),
            AxisAlignedRectAxis::Y => Vec3::new(0.0,1.0,0.0),
            AxisAlignedRectAxis::Z => Vec3::new(0.0,0.0,1.0),
            _ => unimplemented!()
        }
    }
}

impl Hitable for AxisAlignedRect {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let t = self.get_plane_intersection(ray);
        if t < t_min || t > t_max {
            return None;
        }
        let (a, b) = self.get_ab_intersection(ray, t);
        if a < self.amin || a > self.amax || b < self.bmin || b > self.bmax {
            return None;
        }
        Some(HitRecord::new(
            t, 
            (a - self.amin) / self.a_size,
            (b - self.bmin) / self.b_size,
            ray.point_at_parameter(t),
            self.get_plane_normal(),
            self.material.clone()
        ))
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        match self.plane_axis {
            AxisAlignedRectAxis::X => AABB::new(Vec3::new(self.c-0.0001, self.amin, self.bmin), Vec3::new(self.c+0.0001, self.amax, self.bmax)),
            AxisAlignedRectAxis::Y => AABB::new(Vec3::new( self.amin, self.c-0.0001, self.bmin), Vec3::new(self.amax, self.c+0.0001, self.bmax)),
            AxisAlignedRectAxis::Z => AABB::new(Vec3::new(self.amin, self.bmin, self.c-0.0001), Vec3::new(self.amax, self.bmax, self.c + 0.0001)),
            _ => unimplemented!()
        }
    }
}