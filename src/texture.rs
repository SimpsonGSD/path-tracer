use math::vec3::Vec3;

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
    fn value(&self, u: f64, v: f64, point: &Vec3) -> &Vec3 {
        &self.colour
    }
}