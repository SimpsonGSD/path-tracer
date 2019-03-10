use math::*;
use material::Material;
use hitable::*;
use std::rc::Rc;

pub struct Sphere {
    center: Vec3,
    radius: f64,
    material: Rc<Material>,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f64, material: Rc<Material>) -> Sphere {
        Sphere {
            center,
            radius,
            material,
        }
    }
}

impl Hitable for Sphere {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        // SS: A lot of 2s cancelled out here
        let oc = ray.origin() - &self.center;
        let a = vec3::dot(&ray.direction(), &ray.direction());
        let b = vec3::dot(&oc, &ray.direction());
        let c = vec3::dot(&oc, &oc) - self.radius*self.radius;
        let discriminant = b*b - a*c;
        if discriminant > 0.0 {

            let temp = (-b - (b*b-a*c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                let point = ray.point_at_parameter(temp);
                return Some(HitRecord::new(
                    temp,
                    point.clone(),
                    (point - &self.center) / self.radius,
                    Rc::clone(&self.material))
                );
            }

            let temp = (-b + (b*b-a*c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                let point = ray.point_at_parameter(temp);
                return Some(HitRecord::new(
                    temp,
                    point.clone(),
                    (point - &self.center) / self.radius,
                    Rc::clone(&self.material))
                );
            }
        } 

        None
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        AABB::new(&self.center - Vec3::from_float(self.radius), &self.center + Vec3::from_float(self.radius))
    }
}