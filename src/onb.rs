use crate::math::vec3::*;

pub struct ONB {
    pub u: Vec3,
    pub v: Vec3, 
    pub w: Vec3,
}

impl ONB {
    pub fn new() -> Self {
        Self {
            u: Vec3::new(1.0, 0.0, 0.0),
            v: Vec3::new(0.0, 1.0, 0.0),
            w: Vec3::new(0.0, 0.0, 1.0),
        }
    }

    pub fn build_from_w(n: &Vec3) -> Self {
        let w = Vec3::new_unit_vector(n);
        let a;
        if w.x.abs() > 0.9 {
            a = Vec3::new(0.0, 1.0, 0.0);
        } else {
            a = Vec3::new(1.0, 0.0, 0.0);
        }
        let v = Vec3::new_unit_vector(&cross(&w, &a));
        let u = cross(&w, &v);
        Self {
            u,
            v,
            w
        }
    }

    pub fn local(&self, a: Vec3) -> Vec3 {
        self.u*a.x + self.v*a.y + self.w*a.z
    }

}

