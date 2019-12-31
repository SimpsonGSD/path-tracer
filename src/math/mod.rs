#![allow(dead_code)]

pub mod vec3;
pub mod ray;
pub mod random;
pub mod aabb;
pub mod noise;
extern crate rand;

pub use self::vec3::*;
pub use self::ray::*;
pub use self::random::*;
pub use self::aabb::*;

pub fn lerp<T>(a: &T, b: &T, t: f64) -> T
where for<'a> &'a T: std::ops::Mul<f64, Output = T>,
      for<'a> T: std::ops::Add<T, Output = T>,
{
    a*(1.0-t) + b*t
}

pub fn clamp<T: PartialOrd<T> + Clone>(a: &T, minimum: &T, maximum: &T) -> T {
    let b = if a > maximum {maximum} else {a};
    let b = if b < minimum {minimum} else {b};
    (*b).clone()
}

pub fn round_down_to_closest_factor (factor_to_round: u32, factor_of: u32) -> u32 {
    let factor = factor_of as f64 / factor_to_round as f64;
    factor_of / factor.floor() as u32
}

pub fn round_up_to_closest_factor (factor_to_round: u32, factor_of: u32) -> u32 {
    let factor = factor_of as f64 / factor_to_round as f64;
    factor_of / factor.round() as u32
}

#[cfg(test)]
mod tests {
    // TODO(SS): Added unit tests for round_to_closest_factor functions
}