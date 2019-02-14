use math::vec3::Vec3;
use math::ray::Ray;
use math::aabb::AABB;
use material::Material;
use std::rc::Rc;

pub struct HitRecord {
    pub t: f64,
    pub p: Vec3,
    pub normal: Vec3,
    pub mat: Rc<Material>
}

impl HitRecord {
    pub fn new(t: f64, p: Vec3, normal: Vec3, mat: Rc<Material>) -> HitRecord {
        HitRecord {
            t,
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

pub struct HitableList {
    list: Vec<Rc<Hitable>>
}

impl HitableList {
    pub fn new(list: Vec<Rc<Hitable>>) -> HitableList {
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
    fn bounding_box(&self, t0: f64, t1: f64) -> AABB {
        unreachable!(); 
    }
}