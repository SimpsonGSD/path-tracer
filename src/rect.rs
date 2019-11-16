
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
    a0: f64,
    a1: f64,
    b0: f64,
    b1: f64,
    c: f64, // corresponds to the axis denoted by plane_axis
    a_size: f64,
    b_size: f64,
    plane_axis: AxisAlignedRectAxis,
}

const FLT_TOLERANCE: f64 = 0.0000001;

impl AxisAlignedRect {
    pub fn new(
        min: &Vec3,
        max: &Vec3, 
        plane_axis: AxisAlignedRectAxis,
        material: Arc<dyn Material + Send + Sync + 'static>) 
    -> Self {

        let (c, a0, b0, a1, b1) = match plane_axis {
            AxisAlignedRectAxis::X => {
                if (min.x - max.x).abs() > FLT_TOLERANCE {
                    panic!("min.x != max.x, no axis-aligned plane supplied");
                }
                (min.x, min.y, min.z, max.y, max.z)
            },
            AxisAlignedRectAxis::Y => {
                if (min.y - max.y).abs() > FLT_TOLERANCE {
                    panic!("min.y != max.y, no axis-aligned plane supplied");
                }
                (min.y, min.x, min.z, max.x, max.z)
            },
            AxisAlignedRectAxis::Z => {
                if (min.z - max.z).abs() > FLT_TOLERANCE {
                    panic!("min.z != max.z, no axis-aligned plane supplied");
                }
                (min.z, min.x, min.y, max.x, max.y)
            },
        };

        Self {
            material,
            a0,
            a1, 
            b0,
            b1,
            c,
            a_size: a1 - a0,
            b_size: b1 - b0,
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
        if a < self.a0 || a > self.a1 || b < self.b0 || b > self.b1 {
            return None;
        }
        Some(HitRecord::new(
            t, 
            (a - self.a0) / self.a_size,
            (b - self.b0) / self.b_size,
            ray.point_at_parameter(t),
            self.get_plane_normal(),
            self.material.clone()
        ))
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        match self.plane_axis {
            AxisAlignedRectAxis::X => AABB::new(Vec3::new(self.c-0.0001, self.a0, self.b0), Vec3::new(self.c+0.0001, self.a1, self.b1)),
            AxisAlignedRectAxis::Y => AABB::new(Vec3::new( self.a0, self.c-0.0001, self.b0), Vec3::new(self.a1, self.c+0.0001, self.b1)),
            AxisAlignedRectAxis::Z => AABB::new(Vec3::new(self.a0, self.b0, self.c-0.0001), Vec3::new(self.a1, self.b1, self.c + 0.0001)),
            _ => unimplemented!()
        }
    }
}