use math::vec3::Vec3;
use crate::random;

fn trillinear_interpolate(c: &[[[f64; 2]; 2]; 2], u: f64, v: f64, w: f64) -> f64 {
    let mut accum = 0.0;

    for i in 0..2 {
        for j in 0..2 {
            for k in 0..2 {
                let (i_f64, j_f64, k_f64) = (i as f64, j as f64, k as f64);
               // let weight_v = Vec3::new(u - i_f64, v - j_f64, w - k_f64);
                accum += (i_f64 * u + (1.0 - i_f64) * (1.0 - u)) *
                         (j_f64 * v + (1.0 - j_f64) * (1.0 - v)) *
                         (k_f64 * w + (1.0 - k_f64) * (1.0 - w)) * c[i][j][k];
            }
        }
    }

    accum
}


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

        let hermite_cubic = |x: f64| -> f64 {
            x*x*(3.0 - 2.0*x)
        };

        let u = hermite_cubic(u);
        let v = hermite_cubic(v);
        let w = hermite_cubic(w);

        let mut c = [[[0.0; 2]; 2]; 2];
        for di in 0..2 {
            for dj in 0..2 {
                for dk in 0..2 {
                    let di_i32 = di as i32;
                    let dj_i32 = dj as i32;
                    let dk_i32 = dk as i32;
                    c[di][dj][dk] = RAN_FLOAT[
                        (PERM_X[(i+di_i32 & 255) as usize] ^ 
                         PERM_Y[(j+dj_i32 & 255) as usize] ^ 
                         PERM_Z[(k+dk_i32 & 255) as usize]) as usize
                    ]
                }
            }
        }
        trillinear_interpolate(&c, u, v, w)
    }
}
