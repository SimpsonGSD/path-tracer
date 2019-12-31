use math::vec3::Vec3;

pub struct Ray {
    pub origin: Vec3,
    pub direction: Vec3, // not guaranteed to be normalized, must normalize if being used in angles
    pub time: f64
}

impl Ray {
    pub fn new(origin: Vec3, direction: Vec3, time: f64) -> Ray {
        Ray {
            origin,
            direction,
            time
        }
    }

    pub fn origin(&self) -> &Vec3 {
        &self.origin
    }

    pub fn direction(&self) -> &Vec3 {
        &self.direction
    }

    pub fn time(&self) -> f64 {
        self.time
    }

    pub fn point_at_parameter(&self, t: f64) -> Vec3 {
        &self.origin + &(&self.direction * t)
    }
}