use math::*;
use material::{ThreadsafeMaterial, Isotropic};
use hitable::*;
use texture::ThreadsafeTexture;
use std::sync::Arc;

pub struct ConstantMedium {
    boundary: Arc<ThreadsafeHitable>,
    density: f64, 
    phase_function: Arc<ThreadsafeMaterial>,
}

impl ConstantMedium {
    pub fn new(boundary: Arc<ThreadsafeHitable>, density: f64, texture: Arc<ThreadsafeTexture>) -> Self {
        let phase_function = Arc::new(Isotropic::new(texture));
        Self {
            boundary, 
            density,
            phase_function
        }
    }
}

impl Hitable for ConstantMedium {
    fn hit(&self, r: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>{
        // print occasional samples when debugging. To enable set enable_debug true.
        const ENABLE_DEBUG: bool = false;
        let debugging: bool = ENABLE_DEBUG && (random::rand() < 0.00001);

        if let Some(mut rec1) = self.boundary.hit(r, -std::f64::MAX, std::f64::MAX) {
            if let Some(mut rec2) = self.boundary.hit(r, rec1.t+0.0001, std::f64::MAX) {
                if debugging {
                    println!("t0 {} t1 {}", rec1.t, rec2.t);
                }
                rec1.t = rec1.t.max(t_min);
                rec2.t = rec2.t.min(t_max);
                if rec1.t >= rec2.t {
                    return None;
                }
                let ray_length = r.direction.length();
                let distance_inside_boundary = (rec2.t - rec1.t) * ray_length;
                let hit_distance = -(1.0 / self.density) * random::rand().ln();
                if hit_distance < distance_inside_boundary {
                    let time = rec1.t + hit_distance / ray_length;
                    let point = r.point_at_parameter(time);
                    if debugging {
                        println!("hit_distance = {}", hit_distance);
                        println!("time = {}", time);
                        println!("point = {}", point);
                    }
                    let normal = Vec3::new(1.0, 0.0, 0.0); // arbitary
                    return Some(HitRecord::new(
                        time, 
                        0.0, // u - no surface uvs for a volume, we could project on to boundary if required or support uvw for volumetric coords
                        0.0, // v 
                        point, 
                        normal, 
                        self.phase_function.clone(),
                    ));
                }
            }
        }

        None
    }
    fn bounding_box(&self, t0: f64, t1: f64) -> AABB {
        self.boundary.bounding_box(t0, t1)
    }
}