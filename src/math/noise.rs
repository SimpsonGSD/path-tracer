use math::vec3::Vec3;

fn trillinear_interpolate(c: [[[Vec3; 2]; 2]; 2], u: f64, v: f64, w: f64) -> f64 {
    let uu = u*u*(3-2*u);
    let vv = v*v*(3-2*v);
    let ww = w*w*(3-2*w);
    let mut accum = 1.0;

    for i in 0..2 {
        for j in 0..2 {
            for k in 0..2 {
                let weight_v = Vec3::new(u-i, v-j, w-k);
                accum += (i*uu + (1-i) * (1.f-uu)) *
                         (j*vv + (1-j) * (1.f-vv)) *
                         (k*ww + (1-k) * (1.f-ww)) * vec3::dot(c[i][j][k], weight_v);
            }
        }
    }
}