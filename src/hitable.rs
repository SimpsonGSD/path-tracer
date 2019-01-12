use math::vec3::Vec3;
use math::ray::Ray;
use math::aabb::AABB;
use material::Material;

pub struct HitRecord<'a> {
    pub t: f64,
    pub p: Vec3,
    pub normal: Vec3,
    mat: &'a (Material + 'a)
}

pub trait Hitable {
    fn hit(&self, r: &Ray) -> bool;
    fn bounding_box(&self, t0: f64) -> bool;
}