use math::*;
use std::sync::Arc;
use crate::noise;
use crate::math;

pub trait Texture {
    fn value(&self, u: f64, v: f64, point: &Vec3) -> Vec3;
}

pub type ThreadsafeTexture = dyn Texture + Send + Sync;

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
    fn value(&self, _u: f64, _v: f64, _point: &Vec3) -> Vec3 {
        self.colour.clone()
    }
}

pub struct CheckerTexture {
    even: Arc<dyn Texture + Send + Sync + 'static>,
    odd: Arc<dyn Texture + Send + Sync + 'static>
}

impl CheckerTexture {
    pub fn new(
        even: Arc<dyn Texture + Send + Sync + 'static>, 
        odd: Arc<dyn Texture + Send + Sync + 'static>) 
    -> CheckerTexture {
        CheckerTexture {
            even,
            odd
        }
    }
}

impl Texture for CheckerTexture {
    fn value(&self, u: f64, v: f64, point: &Vec3) -> Vec3 {
        let sines = (10.0*point.x).sin() * (10.0*point.y).sin() * (10.0*point.z).sin();
        if sines < 0.0 {
            self.odd.value(u,v,&point)
        } else {
            self.even.value(u,v,&point)
        }
    }
}

pub struct NoiseTexture {
    pub scale: f64,
}

impl NoiseTexture {
    pub fn new(scale: f64) -> Self {
        Self {
            scale
        }
    }
}

impl Texture for NoiseTexture {
    fn value(&self, _u: f64, _v: f64, point: &Vec3) -> Vec3 {
        let noise = self.scale * point.z + 10.0 * noise::Perlin::turb(point, 7);
        Vec3::from_float(1.0) * 0.5 * (1.0 + noise.sin())
    }
}

pub struct ImageTexture {
    width: u32,
    height: u32,
    data: Vec<u8>
}

impl ImageTexture {
    pub fn new(image_bytes: &[u8]) -> Self {

        let image = image::load_from_memory(image_bytes)
                        .expect("Binary corrupted!")
                        .to_rgb();
        let height = image.height();
        let width = image.width();
        let data = image.into_vec();

        Self {
            width,
            height,
            data,
        }
    }
}

impl Texture for ImageTexture {
    fn value(&self, u: f64, v: f64, _point: &Vec3) -> Vec3 {
        let (width_f64, height_f64) = (self.width as f64, self.height as f64);
        let i = u * width_f64;
        let j = v * height_f64 - 0.001;
        let i = math::clamp(&i, &0.0, &(width_f64 - 1.0)) as usize;
        let j = math::clamp(&j, &0.0, &(height_f64 - 1.0)) as usize;
        let pixel_offset = 3 * i + 3 * self.width as usize * j;
        let r = self.data[pixel_offset] as f64 / 255.0;
        let g = self.data[pixel_offset + 1] as f64 / 255.0;
        let b = self.data[pixel_offset + 2] as f64 / 255.0;
        Vec3::new(r, g, b)
    }
}