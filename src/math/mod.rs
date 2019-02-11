pub mod vec3;
pub mod ray;
pub mod random;
pub mod aabb;
extern crate rand;

pub fn lerp<T>(a: &T, b: &T, t: f64) -> T
where for<'a> &'a T: std::ops::Mul<f64, Output = T>,
      for<'a> T: std::ops::Add<T, Output = T>,
{
    a*(1.0-t) + b*t
}

#[cfg(test)]
mod tests {
}