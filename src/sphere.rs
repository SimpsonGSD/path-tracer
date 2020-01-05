use math::*;
use material::Material;
use hitable::*;
use std::sync::Arc;
use std::f64::consts::{PI, FRAC_PI_2};
use onb::ONB;

fn get_sphere_uv(point: &Vec3) -> (f64, f64) {
    let phi = point.z.atan2(point.x);
    let theta = point.y.asin();
    let u = 1.0 - (phi + PI) / (PI * 2.0); // convert from [-pi, pi] to [1, 0]
    let v = (theta + FRAC_PI_2) / PI; // convert from [-pi/2, pi/2] tp [0, 1]
    (u, v)
}

fn random_to_sphere(radius: f64, distance_squared: f64) -> Vec3 {
    let r1 = random::rand();
    let r2 = random::rand();
    let z = 1.0 + r2*((1.0-radius*radius/distance_squared).sqrt()-1.0);
    let phi = 2.0*PI*r1;
    let c = (1.0-z*z).sqrt();
    let x = phi.cos()*c;
    let y = phi.sin()*c;
    Vec3::new(x, y, z)
}

pub struct Sphere {
    center: Vec3,
    radius: f64,
    material: Arc<dyn Material + Send + Sync + 'static>,
}

impl Sphere {
    pub fn new(center: Vec3, radius: f64, material: Arc<dyn Material + Send + Sync + 'static>) -> Sphere {
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
                let (u, v) = get_sphere_uv(&((&self.center - &point)/self.radius));
                return Some(HitRecord::new(
                    temp,
                    u, v,
                    point.clone(),
                    (point - &self.center) / self.radius,
                    Arc::clone(&self.material))
                );
            }

            let temp = (-b + (b*b-a*c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                let point = ray.point_at_parameter(temp);
                let (u, v) = get_sphere_uv(&(&self.center - &point));
                return Some(HitRecord::new(
                    temp,
                    u, v,
                    point.clone(),
                    (point - &self.center) / self.radius,
                    Arc::clone(&self.material))
                );
            }
        } 

        None
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        AABB::new(&self.center - Vec3::from_float(self.radius), &self.center + Vec3::from_float(self.radius))
    }

    fn pdf_value(&self, origin: &Vec3, direction: &Vec3) -> f64 {
        if let Some(_) = self.hit(&Ray::new(*origin, *direction, 0.0), 0.001, std::f64::MAX) {
            let cos_theta_max = (1.0 - self.radius*self.radius/(self.center-origin).squared_length()).sqrt();
            let solid_angle = 2.0*PI*(1.0-cos_theta_max);
            1.0 / solid_angle
        } else {
            0.0
        }
    }
    fn random(&self, origin: &Vec3) -> Vec3 {
        let direction = self.center - origin;
        let distance_sq = direction.squared_length();
        let uvw = ONB::build_from_w(&direction);
        uvw.local(random_to_sphere(self.radius, distance_sq))
    }
}

pub struct MovingSphere {
    center0: Vec3,
    //center1: Vec3,
    center_range: Vec3,
    time0: f64,
    //time1: f64,
    time_range: f64,
    radius: f64,
    material: Arc<dyn Material + Send + Sync + 'static>
}

impl MovingSphere {
    pub fn new(center0: Vec3, center1: Vec3, time0: f64, time1: f64, radius: f64, material: Arc<dyn Material + Send + Sync + 'static>) -> MovingSphere {
        let center_range = &center1 - &center0;
        MovingSphere {
            center0,
            //center1,
            center_range,
            time0, 
            //time1,
            time_range: time1 - time0,
            radius,
            material,
        }
    }

    fn center(&self, time: f64) -> Vec3 {
        &self.center0 + ((time - self.time0) / self.time_range) * (&self.center_range)
    }
}


impl Hitable for MovingSphere {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        // SS: A lot of 2s cancelled out here
        let center = self.center(ray.time());
        let oc = ray.origin() - &center;
        let a = vec3::dot(&ray.direction(), &ray.direction());
        let b = vec3::dot(&oc, &ray.direction());
        let c = vec3::dot(&oc, &oc) - self.radius*self.radius;
        let discriminant = b*b - a*c;
        if discriminant > 0.0 {

            let temp = (-b - (b*b-a*c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                let point = ray.point_at_parameter(temp);
                let (u, v) = get_sphere_uv(&(&center - &point));
                return Some(HitRecord::new(
                    temp,
                    u, v,
                    point.clone(),
                    (point - &center) / self.radius,
                    Arc::clone(&self.material))
                );
            }

            let temp = (-b + (b*b-a*c).sqrt()) / a;
            if temp < t_max && temp > t_min {
                let point = ray.point_at_parameter(temp);
                let (u, v) = get_sphere_uv(&(&center - &point));
                return Some(HitRecord::new(
                    temp,
                    u, v,
                    point.clone(),
                    (point - &center) / self.radius,
                    Arc::clone(&self.material))
                );
            }
        } 

        None
    }

    fn bounding_box(&self, t0: f64, t1: f64) -> AABB {
        let center0 = self.center(t0);
        let center1 = self.center(t1);
        let box0 = AABB::new(&center0 - Vec3::from_float(self.radius), &center0 + Vec3::from_float(self.radius));
        let box1 = AABB::new(&center1 - Vec3::from_float(self.radius), &center1 + Vec3::from_float(self.radius));
        AABB::get_union(&box0, &box1)
    }
}

