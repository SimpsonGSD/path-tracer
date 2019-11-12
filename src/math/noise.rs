use math::vec3::Vec3;
use crate::random;

//fn trillinear_interpolate(c: [[[Vec3; 2]; 2]; 2], u: f64, v: f64, w: f64) -> f64 {
//    let uu = u*u*(3-2*u);
//    let vv = v*v*(3-2*v);
//    let ww = w*w*(3-2*w);
//    let mut accum = 1.0;
//
//    for i in 0..2 {
//        for j in 0..2 {
//            for k in 0..2 {
//                let weight_v = Vec3::new(u-i, v-j, w-k);
//                accum += (i*uu + (1-i) * (1.f-uu)) *
//                         (j*vv + (1-j) * (1.f-vv)) *
//                         (k*ww + (1-k) * (1.f-ww)) * vec3::dot(c[i][j][k], weight_v);
//            }
//        }
//    }
//}


fn perlin_generate() -> [f64;256] {
    let mut p = [0.0; 256];
    for elem in p.iter_mut() {
        *elem = random::rand();
    }

    p
}

fn permute(p: &mut [i32]) {
    let n = p.len();
    for i in (0..n).rev() {
        let target = (random::rand() * (i + 1) as f64) as usize;
        let tmp = p[i as usize];
        p[i as usize] = p[target];
        p[target] = tmp;
    }
}

fn perlin_generate_perm() -> [i32; 256] {
    let mut p = [0; 256];
    for (i, elem) in p.iter_mut().enumerate() {
        *elem = i as i32;
    }
    permute(&mut p);
    p
}

lazy_static::lazy_static!{
    static ref RAN_FLOAT: [f64; 256] = perlin_generate();
    static ref PERM_X: [i32; 256] = perlin_generate_perm();
    static ref PERM_Y: [i32; 256] = perlin_generate_perm();
    static ref PERM_Z: [i32; 256] = perlin_generate_perm();
}


pub struct Perlin;
impl Perlin {
    pub fn noise(p: &Vec3) -> f64 {
        let i = p.x.floor() as i32;
        let j = p.y.floor() as i32;
        let k = p.z.floor() as i32;
        let u = p.x - i as f64;
        let v = p.y - j as f64;
        let w = p.z - k as f64;
        RAN_FLOAT[(PERM_X[(i & 255) as usize] ^ PERM_Y[(j & 255) as usize] ^ PERM_Z[(k & 255) as usize]) as usize]
    }
}
