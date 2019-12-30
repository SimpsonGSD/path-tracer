#![allow(dead_code)]

use crate::hitable::*;
use std::sync::Arc;
use crate::bvh::BvhNode;
use crate::math::vec3::*;

pub struct SceneBuilder {
    scene: Vec<Arc<dyn Hitable + Send + Sync + 'static>>,
}

impl SceneBuilder {
    pub fn new() -> Self {
        Self {
            scene: vec![],
        }
    }

    pub fn as_bvh(self) -> Box<dyn Hitable + Send + Sync + 'static> {
        Box::new(BvhNode::from_list(self.scene.clone(), 0.0, 1.0))
    }

    pub fn as_bvh_node(self) -> Arc<dyn Hitable + Send + Sync + 'static> {
        Arc::new(BvhNode::from_list(self.scene.clone(), 0.0, 1.0))
    }

    pub fn as_hitable_list(self) -> HitableList {
        HitableList::new(self.scene.clone())
    }

    pub fn as_hitable(mut self) -> Arc<ThreadsafeHitable> {
        let last_hitable = self.scene.pop();
        last_hitable.unwrap()
    }

    pub fn add_hitable(&mut self, hitable: Arc<dyn Hitable + Send + Sync>) -> &mut Self {
        self.scene.push(hitable);
        self
    }

    pub fn flip_normals(&mut self) -> &mut Self {
        let last_hitable = self.scene.pop();
        if let Some(hitable) = last_hitable {
            self.scene.push(Arc::new(FlipNormals::new(hitable)));
        }
        self
    }

    pub fn translate(&mut self, translation: Vec3) -> &mut Self {
        let last_hitable = self.scene.pop();
        if let Some(hitable) = last_hitable {
            self.scene.push(Arc::new(Translate::new(hitable, translation)));
        }
        self
    }

    pub fn rotate_y(&mut self, angle: f64) -> &mut Self {
        let last_hitable = self.scene.pop();
        if let Some(hitable) = last_hitable {
            self.scene.push(Arc::new(RotateY::new(hitable, angle)));
        }
        self
    }
}

