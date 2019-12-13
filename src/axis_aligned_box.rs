use math::*;
use material::Material;
use hitable::*;
use std::sync::Arc;
use rect::*;


pub struct AxisAlignedBox {
    pub pmin: Vec3,
    pub pmax: Vec3,
    pub list: HitableList,
}

impl AxisAlignedBox {
    pub fn new(pmin: Vec3, pmax: Vec3, material: Arc<dyn Material + Send + Sync>) -> Self {
        let mut list: Vec<Arc<dyn Hitable + Send + Sync + 'static>> = vec![];
        list.push(Arc::new(AxisAlignedRect::new(pmin.x, pmax.x, pmin.y, pmax.y, pmax.z, AxisAlignedRectAxis::Z, material.clone())));
        list.push(Arc::new(FlipNormals::new(Arc::new(AxisAlignedRect::new(pmin.x, pmax.x, pmin.y, pmax.y, pmin.z, AxisAlignedRectAxis::Z, material.clone())))));
        list.push(Arc::new(AxisAlignedRect::new(pmin.x, pmax.x, pmin.z, pmax.z, pmax.y, AxisAlignedRectAxis::Y, material.clone())));
        list.push(Arc::new(FlipNormals::new(Arc::new(AxisAlignedRect::new(pmin.x, pmax.x, pmin.z, pmax.z, pmin.y, AxisAlignedRectAxis::Y, material.clone())))));
        list.push(Arc::new(AxisAlignedRect::new(pmin.y, pmax.y, pmin.z, pmax.z, pmax.x, AxisAlignedRectAxis::X, material.clone())));
        list.push(Arc::new(FlipNormals::new(Arc::new(AxisAlignedRect::new(pmin.y, pmax.y, pmin.z, pmax.z, pmin.x, AxisAlignedRectAxis::X, material.clone())))));
        let list = HitableList::new(list);
        Self {
            pmin,
            pmax,
            list,
        }
    }
}

impl Hitable for AxisAlignedBox {
    fn hit(&self, ray: &Ray, t_min: f64, t_max: f64) -> Option<HitRecord> {
        self.list.hit(ray, t_min, t_max)
    }

    fn bounding_box(&self, _t0: f64, _t1: f64) -> AABB {
        AABB::new(self.pmin, self.pmax)
    }

}