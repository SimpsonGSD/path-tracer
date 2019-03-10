use math::*;
use hitable::HitRecord;
use texture::Texture;
use std::rc::Rc;

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

pub trait Material {
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<(Ray, Vec3)>;
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
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<(Ray, Vec3)> {
        let outward_normal: Vec3;
        let reflected = reflect(&r_in.direction(), &rec.normal);
        let ni_over_nt: f64;
        let attenuation = Vec3::new(1.0, 1.0, 1.0);
        let mut refracted = Vec3::new_zero_vector();
        let reflect_prob: f64;
        let cosine: f64;

        if vec3::dot(&r_in.direction(), &rec.normal) > 0.0 {
            outward_normal = -(rec.normal.clone());
            ni_over_nt = self.ref_idx;
            cosine = self.ref_idx * vec3::dot(&r_in.direction(), &rec.normal) / r_in.direction().length();
        } else {
            outward_normal = rec.normal.clone();
            ni_over_nt = 1.0 / self.ref_idx;
            cosine = -vec3::dot(&r_in.direction(), &rec.normal) / r_in.direction().length();
        }

        if refract(&r_in.direction(), &outward_normal, ni_over_nt, &mut refracted) {
            reflect_prob = schlick(cosine, self.ref_idx);
        } else {
             //  scattered = ray(rec.p, reflected);
             reflect_prob = 1.0;
        }

        let scattered;
        if random::rand() < reflect_prob {
            scattered = Ray::new(rec.p.clone(), reflected, r_in.time());
        } else {
            scattered = Ray::new(rec.p.clone(), refracted, r_in.time());
        }

        Some((scattered, attenuation))
    }
}

pub struct Lambertian {
    albedo: Rc<Texture>
}

impl Lambertian {
    pub fn new(albedo: Rc<Texture>) -> Lambertian {
        Lambertian {
            albedo
        }
    }
}

impl Material for Lambertian{
    fn scatter(&self, r_in: &Ray, rec: &HitRecord) -> Option<(Ray, Vec3)> {
        let target = &rec.p + &rec.normal + random_in_unit_sphere();
        let scattered = Ray::new(rec.p.clone(), &target-&rec.p, r_in.time());
        let attenuation = self.albedo.value(0.0, 0.0, &rec.p);
        Some((scattered, attenuation.clone()))
    }
}