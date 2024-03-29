#![allow(dead_code)]
use math::*;
use hitable::HitRecord;
use texture::{Texture, ConstantTexture, ThreadsafeTexture};
use std::sync::Arc;
use std::f64::consts::{FRAC_1_PI, PI};
use crate::onb::ONB;
use hitable::ThreadsafeHitable;

fn random_cosine_direction() -> Vec3 {
    let r1 = random::rand();
    let r2 = random::rand();
    let z = (1.0 - r2).sqrt();
    let phi = 2.0 * PI * r1;
    let r2_sqrt = r2.sqrt();
    let x = phi.cos() * r2_sqrt;
    let y = phi.sin() * r2_sqrt;
    Vec3::new(x, y, z)
}

fn random_in_unit_sphere() -> Vec3 {
    let mut p: Vec3;
    loop  {
        p = 2.0 * Vec3::new(random::rand(), random::rand(), random::rand()) - Vec3::new(1.0, 1.0, 1.0);
        if p.squared_length() < 1.0 {
            break
        }
    }

    p
}

fn random_on_unit_sphere() -> Vec3 {
    let mut p: Vec3;
    loop  {
        p = 2.0 * Vec3::new(random::rand(), random::rand(), random::rand()) - Vec3::new(1.0, 1.0, 1.0);
        if p.squared_length() < 1.0 {
            break
        }
    }

    p.normalise();
    p
}

fn unit_sphere_pdf() -> f64{
    1.0 / (4.0 * std::f64::consts::PI)
}

fn reflect(v: &Vec3, n: &Vec3) -> Vec3 {
    v - &(2.0*vec3::dot(v, n)*n)
}

fn refract(v: &Vec3, n: &Vec3, ni_over_nt: f64, refracted: &mut Vec3) -> bool {
    let uv = Vec3::new_unit_vector(&v);
    let dt = vec3::dot(&uv, &n);
    let discriminant = 1.0 - ni_over_nt*ni_over_nt*(1.0-dt*dt);
    if discriminant > 0.0 {
        *refracted = ni_over_nt*(uv - n*dt) - n*discriminant.sqrt();
        return true;
    } else {
        return false;
    }
}

fn schlick(cosine: f64, ref_idx: f64) -> f64 {
    let mut r0 = (1.0 - ref_idx) / (1.0 + ref_idx);
    r0 = r0*r0;
    r0 + (1.0-r0)*(1.0-cosine).powf(5.0)
}

pub struct MaterialBuilder {
    texture: Arc<dyn Texture + Send + Sync + 'static>,
    albedo: Vec3,
    emissive: f64,
    fuzz: f64,
    refraction_index: f64
}

impl MaterialBuilder {
    pub fn new() -> Self {
        Self {
            texture: Arc::new(ConstantTexture::new(Vec3::from_float(0.0))),
            emissive: 0.0,
            albedo: Vec3::from_float(0.0),
            fuzz: 0.0,
            refraction_index: 1.0,
        }
    }

    pub fn with_texture<'a>(&'a mut self, texture: Arc<dyn Texture + Send + Sync + 'static>) -> &'a mut MaterialBuilder {
        self.texture = texture;
        self
    }

    pub fn set_emissive<'a>(&'a mut self, emissive: f64) -> &'a mut MaterialBuilder {
        self.emissive = emissive;
        self
    }

    pub fn set_albedo<'a>(&'a mut self, albedo: Vec3) -> &'a mut MaterialBuilder {
        self.albedo = albedo;
        self
    }

    pub fn set_fuzz<'a>(&'a mut self, fuzz: f64) -> &'a mut MaterialBuilder {
        self.fuzz = fuzz;
        self
    }

    pub fn set_refraction_index<'a>(&'a mut self, refraction_index: f64) -> &'a mut MaterialBuilder {
        self.refraction_index = refraction_index;
        self
    }

    pub fn lambertian(&self) -> Arc<dyn Material + Send + Sync + 'static> {
        Arc::new(Lambertian::new(self.texture.clone(), self.emissive))
    }

    pub fn diffuse_light(&self) -> Arc<dyn Material + Send + Sync + 'static> {
        Arc::new(DiffuseLight::new(self.texture.clone()))
    }

    pub fn metal(&self) -> Arc<dyn Material + Send + Sync + 'static> {
        Arc::new(Metal::new(self.albedo, self.fuzz))
    }

    pub fn dielectric(&self) -> Arc<dyn Material + Send + Sync + 'static> {
        Arc::new(Dielectric::new(self.refraction_index))
    }
}  

pub struct ScatterResult {
    pub specular_ray: Ray,
    pub is_specular: bool,
    pub albedo: Vec3,
    pub pdf: Arc<dyn PDF>,
}

pub trait Material {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterResult>;
    fn scattering_pdf(&self, _r_in: &Ray, _rec: &HitRecord, _scattered: &Ray) -> f64 {
        0.0
    }
    fn emitted(&self, _ray: &Ray, _rec: &HitRecord, _u: f64, _v: f64, _point: &Vec3) -> Vec3 {
        Vec3::from_float(0.0)
    }
}

pub type ThreadsafeMaterial = dyn Material + Send + Sync;

pub struct  DummyMaterial;
impl DummyMaterial {
    pub fn new() -> Self {
        Self {}
    }
}
impl Material for DummyMaterial {
    fn scatter(&self, _r_in: &Ray, _rec: &HitRecord) -> Option<ScatterResult> {
        None
    }
}

pub struct Dielectric {
    ref_idx: f64
}

impl Dielectric {
    pub fn new(ri: f64) -> Dielectric {
        Dielectric {
            ref_idx: ri
        }
    }
}

impl Material for Dielectric {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterResult> {
        let outward_normal: Vec3;
        let reflected = reflect(&r_in.direction(), &rec.normal);
        let ni_over_nt: f64;
        let albedo = Vec3::new(1.0, 1.0, 1.0);
        let mut refracted = Vec3::new_zero_vector();
        let reflect_prob: f64;
        let cosine: f64;

        if vec3::dot(&r_in.direction(), &rec.normal) > 0.0 {
            outward_normal = -(rec.normal.clone());
            ni_over_nt = self.ref_idx;
            cosine = self.ref_idx * vec3::dot(&r_in.direction, &rec.normal) / r_in.direction().length();
        } else {
            outward_normal = rec.normal.clone();
            ni_over_nt = 1.0 / self.ref_idx;
            cosine = -vec3::dot(&r_in.direction, &rec.normal) / r_in.direction().length();
        }

        if refract(&r_in.direction(), &outward_normal, ni_over_nt, &mut refracted) {
            reflect_prob = schlick(cosine, self.ref_idx);
        } else {
             //  scattered = ray(rec.p, reflected);
             reflect_prob = 1.0;
        }

        let specular_ray;
        let is_specular = true;
        if random::rand() < reflect_prob {
            specular_ray = Ray::new(rec.p.clone(), reflected, r_in.time());
        } else {
            specular_ray = Ray::new(rec.p.clone(), refracted, r_in.time());
        }

        Some(ScatterResult{is_specular, specular_ray, albedo, pdf: Arc::new(DummyPDF{})})
    }
}

pub struct Metal {
    albedo: Vec3,
    fuzz: f64,
}

impl Metal {
    pub fn new(albedo: Vec3, fuzz: f64) -> Metal {
        Metal {
            albedo,
            fuzz: fuzz.min(1.0)
        }
    }
}

impl Material for Metal{
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterResult> {
        let reflected = reflect(&Vec3::new_unit_vector(&r_in.direction), &rec.normal);
        let outgoing_ray_dir = reflected + self.fuzz*random_in_unit_sphere();
        let specular_ray = Ray::new(rec.p.clone(), outgoing_ray_dir, r_in.time());

        Some(ScatterResult {
            specular_ray,
            is_specular: true,
            albedo: self.albedo,
            pdf: Arc::new(DummyPDF{})
        })

        // check to see if outgoing ray is reflect externally or not, otherwise it is absorbed
      //  if vec3::dot(&scattered.direction(), &rec.normal) > 0.0 {
       //     let albedo = self.albedo.clone();
        //    let pdf = 1.0;
       //     Some(ScatterResult{scattered, albedo, pdf})
      //  } else {
      //      None
      //  }
    }
}

pub struct Lambertian {
    albedo: Arc<dyn Texture + Send + Sync + 'static>,
    emissive: f64
}

impl Lambertian {
    pub fn new(albedo: Arc<dyn Texture + Send + Sync + 'static>, emissive: f64) -> Lambertian {
        Lambertian {
            albedo,
            emissive
        }
    }
}

impl Material for Lambertian {
    fn scattering_pdf(&self, _r_in: &Ray, rec: &HitRecord, scattered: &Ray) -> f64 {
        let cosine = vec3::dot(&rec.normal, &Vec3::new_unit_vector(&scattered.direction));
        if cosine < 0.0 {
            0.0
        } else {
            cosine * FRAC_1_PI // cosine / PI
        }
    }

    //fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterResult> {
    //    let target = rec.p + rec.normal + random_in_unit_sphere();
    //    let scattered = Ray::new(rec.p, target - rec.p, r_in.time);
    //    let albedo = self.albedo.value(rec.u, rec.v, &rec.p);
    //    let pdf = vec3::dot(&rec.normal, &Vec3::new_unit_vector(&//scattered.direction)) * FRAC_1_PI;
    //    Some(ScatterResult {
    //        scattered, 
    //        albedo, 
    //        pdf
    //    })
    //}

    fn scatter(&self, _r_in: &Ray, rec: &HitRecord) -> Option<ScatterResult> {
        //let uvw = ONB::build_from_w(&rec.normal);
        //let direction = uvw.local(random_cosine_direction());
        //let scattered = Ray::new(rec.p, Vec3::new_unit_vector(&direction), r_in.time);
        let albedo = self.albedo.value(rec.u, rec.v, &rec.p);
        //let pdf = vec3::dot(&uvw.w, &scattered.direction) * FRAC_1_PI;
        Some(ScatterResult {
            specular_ray: Ray::default(), 
            is_specular: false,
            albedo, 
            pdf: Arc::new(CosinePDF::new(&rec.normal)),
        })
    }

    fn emitted(&self, _ray: &Ray, _rec: &HitRecord, u: f64, v: f64, point: &Vec3) -> Vec3 {
        if self.emissive > 0.0 {self.albedo.value(u, v, point) * self.emissive} else {Vec3::from_float(0.0)}
    }
}

pub struct DiffuseLight {
    texture: Arc<dyn Texture + Send + Sync + 'static>
}

impl DiffuseLight {
    pub fn new(texture: Arc<dyn Texture + Send + Sync + 'static>) -> Self {
        Self {
            texture
        }
    }
}

impl Material for DiffuseLight {
    fn scatter(&self, _r_in: &Ray, _rec: &HitRecord) -> Option<ScatterResult> {
        None
    }

    fn emitted(&self, ray: &Ray, rec: &HitRecord, u: f64, v: f64, point: &Vec3) -> Vec3 {
        if dot(&rec.normal, &ray.direction) < 0.0 {
            self.texture.value(u, v, point)
        } else {
            Vec3::new_zero_vector()
        }
    }
}

pub struct Isotropic {
    albedo: Arc<ThreadsafeTexture>,
} 

impl Isotropic {
    pub fn new(albedo: Arc<ThreadsafeTexture>) -> Self {
        Self {
            albedo,
        }
    }
}

impl Material for Isotropic {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<ScatterResult>{
        let specular_ray = Ray::new(rec.p, random_in_unit_sphere(), r_in.time);
        let albedo = self.albedo.value(rec.u, rec.v, &rec.p);
        Some(ScatterResult{is_specular: false, specular_ray, albedo, pdf: Arc::new(DummyPDF{})})
    } 
}

pub trait PDF {
    fn value(&self, direction: &Vec3) -> f64;
    fn generate(&self) -> Vec3;
}

pub struct CosinePDF {
    uvw: ONB,
}

impl CosinePDF {
    pub fn new(w: &Vec3) -> Self {
        Self {
            uvw: ONB::build_from_w(w)
        }
    }
}

// cosine pdf that just computes the dot product (cosine) of the
// basis normal and incoming direction. Returns 0.0 for directions
// with negative result
impl PDF for CosinePDF {
    fn value(&self, direction: &Vec3) -> f64 {
        let cosine = dot(&Vec3::new_unit_vector(direction),&self.uvw.w);
        if cosine > 0.0 {
            cosine * std::f64::consts::FRAC_1_PI
        } else {
            0.0
        }
    }
    fn generate(&self) -> Vec3 {
        self.uvw.local(random_cosine_direction())
    }
}

pub struct HittablePDF {
    origin: Vec3,
    hittable: Arc<ThreadsafeHitable>,
}

impl HittablePDF {
    pub fn new(hittable: Arc<ThreadsafeHitable>, origin: Vec3) -> Self {
        Self {
            origin,
            hittable
        }
    }
}

impl PDF for HittablePDF {
    fn value(&self, direction: &Vec3) -> f64 {
        self.hittable.pdf_value(&self.origin, direction)
    }
    fn generate(&self) -> Vec3 {
        self.hittable.random(&self.origin)
    }
}

pub struct MixturePDF {
    pdfs: [Arc<dyn PDF>; 2]
}

impl MixturePDF {
    pub fn new(pdf0: Arc<dyn PDF>, pdf1: Arc<dyn PDF> ) -> Self {
        Self {
            pdfs: [pdf0, pdf1]
        }
    }
}

impl PDF for MixturePDF {
    fn value(&self,direction: &Vec3) -> f64 {
        0.5 * self.pdfs[0].value(direction) + 0.5*self.pdfs[1].value(direction)
    }
    fn generate(&self) -> Vec3 {
        if random::rand() < 0.5 {
            self.pdfs[0].generate()
        } else {
            self.pdfs[1].generate()
        }
    }
}

pub struct DummyPDF {

}
impl PDF for DummyPDF {
    fn value(&self, _direction: &Vec3) ->f64 {
        0.0
    }
    fn generate(&self) -> Vec3 {
        Vec3::new_zero_vector()
    }
}