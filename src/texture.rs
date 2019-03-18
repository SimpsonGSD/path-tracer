use math::*;
use std::sync::Arc;

pub trait Texture {
    fn value(&self, u: f64, v: f64, point: &Vec3) -> &Vec3;
}

pub struct ConstantTexture {
    colour: Vec3,
}

impl ConstantTexture {
    pub fn new(colour: Vec3) -> ConstantTexture {
        ConstantTexture {
            colour
        }
    }   
}

impl Texture for ConstantTexture {
    fn value(&self, _u: f64, _v: f64, _point: &Vec3) -> &Vec3 {
        &self.colour
    }
}

pub struct CheckerTexture {
    even: Arc<Texture + Send + Sync + 'static>,
    odd: Arc<Texture + Send + Sync + 'static>
}

impl CheckerTexture {
    pub fn new(even: Arc<Texture + Send + Sync + 'static>, odd: Arc<Texture + Send + Sync + 'static>) -> CheckerTexture {
        CheckerTexture {
            even,
            odd
        }
    }
}

impl Texture for CheckerTexture {
    fn value(&self, u: f64, v: f64, point: &Vec3) -> &Vec3 {
        let sines = (10.0*point.x).sin() * (10.0*point.y).sin() * (10.0*point.z).sin();
        if sines < 0.0 {
            self.odd.value(u,v,&point)
        } else {
            self.even.value(u,v,&point)
        }
    }

}