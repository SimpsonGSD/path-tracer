use math::*;
use material::Material;
use hitable::*;
use std::sync::Arc;
use rect::AxisAlignedRect;

pub struct Box {
    pub pmin: Vec3,
    pub pmax: Vec3,
    pub list: Vec<Arc<Hitable + Send + Sync>>,
}

impl Box {
    pub fn new(pmin: Vec3, pmax: Vec3) -> Self {


        Self {
            pmin,
            pmax,
            list: vec![]
        }
    }
}