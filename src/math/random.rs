use super::rand::prelude::*;

pub fn rand() -> f64 {
    let mut rng = rand::thread_rng();
    rng.gen()
}