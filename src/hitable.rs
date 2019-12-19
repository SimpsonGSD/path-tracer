use math::*;
use material::Material;
use std::sync::Arc;

pub struct HitRecord {
    pub t: f64,
    pub u: f64,
    pub v: f64,
    pub p: Vec3,
    pub normal: Vec3,
    pub mat: Arc<dyn Material + Send + Sync + 'static>
}

impl HitRecord {
    pub fn new(t: f64, u: f64, v: f64, p: Vec3, normal: Vec3, mat: Arc<dyn Material + Send + Sync + 'static>) -> HitRecord {
        HitRecord {
            t,
            u,
            v,
            p,
            normal, 
            mat,
        }
    }
}

pub trait Hitable {
    fn hit(&self, r: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord>;
    fn bounding_box(&self, t0: f64, t1: f64) -> AABB;
}

pub type ThreadsafeHitable = dyn Hitable + Send + Sync;

pub struct HitableList {
    list: Vec<Arc<dyn Hitable + Send + Sync + 'static>>
}

impl HitableList {
    pub fn new(list: Vec<Arc<dyn Hitable + Send + Sync + 'static>>) -> HitableList {
        HitableList { list }
    }
}

impl Hitable for HitableList {
    fn hit(&self, r: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let mut closest_so_far = t_max;
        let mut hitrecord = None;
        for object in &self.list {
            if let Some(hit_record) = object.hit(r, t_min, closest_so_far) {
                closest_so_far = hit_record.t;
                hitrecord = Some(hit_record);
            }
        }

        hitrecord
    }
    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        unreachable!(); 
    }
}

pub struct FlipNormals {
    child: Arc<dyn Hitable + Send + Sync + 'static>
}

impl FlipNormals {
    pub fn new(child: Arc<dyn Hitable + Send + Sync + 'static>) -> Self {
        Self {
            child
        }
    }
}

impl Hitable for FlipNormals {
    fn hit(&self, r: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        if let Some(mut hit_record) = self.child.hit(r, t_min, t_max) {
            hit_record.normal = -hit_record.normal;
            return Some(hit_record);
        }

        None
    }

    fn bounding_box(&self, t0: f64, t1: f64) -> AABB {
        self.child.bounding_box(t0, t1)
    }
}

pub struct Translate {
    translation: Vec3,
    hittable: Arc<dyn Hitable + Send + Sync>,
}

impl Translate {
    pub fn new( hittable: Arc<dyn Hitable + Send + Sync>, translation: Vec3) -> Self {
        Self {
            translation,
            hittable,
        }
    }
}

impl Hitable for Translate {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        // translate incoming ray by the inverse of our translation node
        let translated_ray = Ray::new(ray.origin - self.translation, ray.direction, ray.time);
        if let Some(mut hit_record) = self.hittable.hit(&translated_ray, t_min, t_max) {
            hit_record.p += self.translation;
            return Some(hit_record);
        }

        None
    }

    fn bounding_box(&self, t0: f64, t1: f64) -> AABB {
        let mut bounding_box = self.hittable.bounding_box(t0, t1);
        bounding_box.add_translation(self.translation);
        bounding_box
    }
}

pub struct RotateY {
    hittable: Arc<ThreadsafeHitable>,
    sin_theta: f64,
    cos_theta: f64,
    bounding_box: AABB,
}

impl RotateY {
    pub fn new( hittable: Arc<ThreadsafeHitable>, angle: f64) -> Self {
        let radians = angle.to_radians();
        let sin_theta = radians.sin();
        let cos_theta = radians.cos();
        let bounding_box = hittable.bounding_box(0.0, 1.0);
        let mut min = Vec3::from_float(std::f64::MAX);
        let mut max = Vec3::from_float(-std::f64::MAX);

        for i in 0..2 {
            for j in 0..2 {
                for k in 0..2 {
                    let (i_f64, j_f64, k_f64) = (i as f64, j as f64, k as f64);
                    let x = i_f64 * bounding_box.max().x + (1.0 - i_f64) * bounding_box.min().x;
                    let y = j_f64 * bounding_box.max().y + (1.0 - j_f64) * bounding_box.min().y;
                    let z = k_f64 * bounding_box.max().z + (1.0 - k_f64) * bounding_box.min().z;
                    let new_x =  cos_theta * x + sin_theta * z;
                    let new_z = -sin_theta * x + cos_theta * z;
                    let new_axis = Vec3::new(new_x, y, new_z);
                    min = vec3::min(&new_axis, &min);
                    max = vec3::max(&new_axis, &max);
                }
            }
        }

        let bounding_box = AABB::new(min, max);

        Self {
            hittable,
            sin_theta,
            cos_theta,
            bounding_box,
        }
    }

    pub fn unrotate_vector(&self, v: &Vec3) -> Vec3 {
        let mut rotated_vec = v.clone();
        rotated_vec.x = self.cos_theta * v.x - self.sin_theta * v.z;
        rotated_vec.z = self.sin_theta * v.x + self.cos_theta * v.z;
        rotated_vec
    }

    pub fn rotate_vector(&self, v: &Vec3) -> Vec3 {
        let mut rotated_vec = v.clone();
        rotated_vec.x = self.cos_theta * v.x + self.sin_theta * v.z;
        rotated_vec.z = -self.sin_theta * v.x + self.cos_theta * v.z;
        rotated_vec
    }
}


impl Hitable for RotateY {
    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        self.bounding_box.clone()
    }

    fn hit(&self, r: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        let origin = self.unrotate_vector(&r.origin);
        let direction = self.unrotate_vector(&r.direction);
        let ray = Ray::new(origin, direction, r.time);
        match self.hittable.hit(&ray, t_min, t_max) {
            Some(mut hit_record) => {
                hit_record.p = self.rotate_vector(&hit_record.p);
                hit_record.normal = self.rotate_vector(&hit_record.normal);
                Some(hit_record)
            },
            None => None
        }
    }
}