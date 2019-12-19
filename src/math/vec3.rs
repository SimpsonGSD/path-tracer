use std::ops;

#[derive(Debug, Clone, Copy)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64
}

impl Vec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3 {
            x,
            y,
            z,
        }
    }

    pub fn from_float(f: f64) -> Vec3 {
        Vec3 {
            x: f,
            y: f,
            z: f,
        }
    }

    pub fn new_zero_vector() -> Vec3 {
        Vec3::new(0.0,0.0,0.0)
    }

    pub fn new_unit_vector(v: &Vec3) -> Vec3 {
        v.div_float(v.length())
    }

    pub fn r(&self) -> f64 {
        self.x
    }

    pub fn g(&self) -> f64 {
        self.y
    }

    pub fn b(&self) -> f64 {
        self.z
    }

    pub fn length(&self) -> f64 {
        self.squared_length().sqrt()
    }

    pub fn squared_length(&self) -> f64 {
        self.x*self.x + self.y*self.y + self.z*self.z
    }

    pub fn make_unit_vector(&mut self) {
        let length = self.length();
        self.x /= length;
        self.y /= length;
        self.z /= length;
    }

    pub fn equal(&self, rhs: &Vec3) -> bool {
        self.x == rhs.x && self.y == rhs.y && self.z == rhs.z
    }

    fn add_vec(&self, rhs: &Vec3) -> Vec3 {
         Vec3 {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }

    fn add_float(&self, rhs: f64) -> Vec3 {
        Vec3 {
            x: self.x + rhs,
            y: self.y + rhs,
            z: self.z + rhs,
        }
    }

    fn sub_vec(&self, rhs: &Vec3) -> Vec3 {
         Vec3 {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }

    fn sub_float(&self, rhs: f64) -> Vec3 {
        Vec3 {
            x: self.x - rhs,
            y: self.y - rhs,
            z: self.z - rhs,
        }
    }

    fn mul_vec(&self, rhs: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x * rhs.x,
            y: self.y * rhs.y,
            z: self.z * rhs.z,
        }
    }

    fn mul_float(&self, rhs: f64) -> Vec3 {
        Vec3 {
            x: self.x * rhs,
            y: self.y * rhs,
            z: self.z * rhs,
        }
    }

    fn div_vec(&self, rhs: &Vec3) -> Vec3 {
        Vec3 {
            x: self.x / rhs.x,
            y: self.y / rhs.y,
            z: self.z / rhs.z,
        }
    }

    fn div_float(&self, rhs: f64) -> Vec3 {
        Vec3 {
            x: self.x / rhs,
            y: self.y / rhs,
            z: self.z / rhs,
        }
    }
}

impl std::fmt::Display for Vec3 {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {}, {})", self.x, self.y, self.z)
    }
}

pub fn cross(v1: &Vec3, v2: &Vec3) -> Vec3 {
    Vec3::new(
            v1.y*v2.z - v1.z*v2.y,
          -(v1.x*v2.z - v1.z*v2.x),
            v1.x*v2.y - v1.y*v2.x
    )
}

pub fn dot(v1: &Vec3, v2: &Vec3) -> f64 {
    v1.x*v2.x + v1.y*v2.y + v1.z*v2.z
}

pub fn min(v1: &Vec3, v2: &Vec3) -> Vec3 {
    Vec3::new(v1.x.min(v2.x), v1.y.min(v2.y), v1.z.min(v2.z))
}

pub fn max(v1: &Vec3, v2: &Vec3) -> Vec3 {
    Vec3::new(v1.x.max(v2.x), v1.y.max(v2.y), v1.z.max(v2.z))
}

impl ops::Index<usize> for Vec3 {
    type Output = f64;
    fn index<'a>(&'a self, i: usize) -> &'a f64 {
        match i {
            0 => &self.x,
            1 => &self.y,
            2 => &self.z,
            _ => unreachable!()
        }
    }
}

impl ops::IndexMut<usize> for Vec3 {
    fn index_mut<'a>(&'a mut self, i: usize) -> &'a mut f64 {
        match i {
            0 => &mut self.x,
            1 => &mut self.y,
            2 => &mut self.z,
            _ => unreachable!()
        }
    }
}

impl PartialEq<Vec3> for Vec3 {
    fn eq(&self, other: &Vec3) -> bool {
        self.equal(other)
    }
}

impl ops::Add<Vec3> for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Vec3) -> Vec3 {
        self.add_vec(&rhs)
    }
}

impl ops::Add<f64> for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: f64) -> Vec3 {
        self.add_float(rhs)
    }
}

impl<'a> ops::Add<Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Vec3) -> Vec3 {
        self.add_vec(&rhs)
    }
}

impl<'a> ops::Add<&'a Vec3> for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: &'a Vec3) -> Vec3 {
        self.add_vec(rhs)
    }
}

impl<'a> ops::Add<&'a Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn add(self, rhs: &'a Vec3) -> Vec3 {
        self.add_vec(rhs)
    }
}

impl<'a> ops::Add<f64> for &'a Vec3 {
    type Output = Vec3;

    fn add(self, rhs: f64) -> Vec3 {
        self.add_float(rhs)
    }
}

impl<> ops::AddAssign for Vec3 {
    fn add_assign(&mut self, rhs: Vec3) {
        self.x += rhs.x;
        self.y += rhs.y;
        self.z += rhs.z;
    }
}

impl<> ops::AddAssign<f64> for Vec3 {
    fn add_assign(&mut self, rhs: f64) {
        self.x += rhs;
        self.y += rhs;
        self.z += rhs;
    }
}

impl ops::Sub<Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Vec3 {
        self.sub_vec(&rhs)
    }
}

impl ops::Sub<f64> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: f64) -> Vec3 {
        self.sub_float(rhs)
    }
}

impl<'a> ops::Sub<Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Vec3 {
        self.sub_vec(&rhs)
    }
}

impl<'a> ops::Sub<&'a Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: &'a Vec3) -> Vec3 {
        self.sub_vec(rhs)
    }
}

impl<'a> ops::Sub<&'a Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: &'a Vec3) -> Vec3 {
        self.sub_vec(rhs)
    }
}

impl<'a> ops::Sub<f64> for &'a Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: f64) -> Vec3 {
        self.sub_float(rhs)
    }
}

impl<> ops::SubAssign for Vec3 {
    fn sub_assign(&mut self, rhs: Vec3) {
        self.x -= rhs.x;
        self.y -= rhs.y;
        self.z -= rhs.z;
    }
}

impl<> ops::SubAssign<f64> for Vec3 {
    fn sub_assign(&mut self, rhs: f64) {
        self.x -= rhs;
        self.y -= rhs;
        self.z -= rhs;
    }
}

impl ops::Mul<Vec3> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        self.mul_vec(&rhs)
    }
}

impl ops::Mul<f64> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f64) -> Vec3 {
        self.mul_float(rhs)
    }
}

impl ops::Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Vec3 {
        self.mul_float(rhs as f64)
    }
}

impl ops::Mul<Vec3> for f64 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        rhs * self
    }
}

impl ops::Mul<Vec3> for f32 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        rhs * self as f64
    }
}

impl<'a> ops::Mul<Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: Vec3) -> Vec3 {
        self.mul_vec(&rhs)
    }
}

impl<'a> ops::Mul<&'a Vec3> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: &'a Vec3) -> Vec3 {
        self.mul_vec(rhs)
    }
}

impl<'a> ops::Mul<&'a Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: &'a Vec3) -> Vec3 {
        self.mul_vec(rhs)
    }
}

impl<'a> ops::Mul<f64> for &'a Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f64) -> Vec3 {
        self.mul_float(rhs)
    }
}

impl<'a> ops::Mul<&'a Vec3> for f64 {
    type Output = Vec3;

    fn mul(self, rhs: &'a Vec3) -> Vec3 {
        rhs.mul_float(self)
    }
}

impl<> ops::MulAssign for Vec3 {
    fn mul_assign(&mut self, rhs: Vec3) {
        self.x *= rhs.x;
        self.y *= rhs.y;
        self.z *= rhs.z;
    }
}

impl<> ops::MulAssign<f64> for Vec3 {
    fn mul_assign(&mut self, rhs: f64) {
        self.x *= rhs;
        self.y *= rhs;
        self.z *= rhs;
    }
}

impl ops::Div<Vec3> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: Vec3) -> Vec3 {
        self.div_vec(&rhs)
    }
}

impl ops::Div<f64> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: f64) -> Vec3 {
        self.div_float(rhs)
    }
}

impl<'a> ops::Div<Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn div(self, rhs: Vec3) -> Vec3 {
        self.div_vec(&rhs)
    }
}

impl<'a> ops::Div<&'a Vec3> for Vec3 {
    type Output = Vec3;

    fn div(self, rhs: &'a Vec3) -> Vec3 {
        self.div_vec(rhs)
    }
}

impl<'a> ops::Div<&'a Vec3> for &'a Vec3 {
    type Output = Vec3;

    fn div(self, rhs: &'a Vec3) -> Vec3 {
        self.div_vec(rhs)
    }
}

impl<'a> ops::Div<f64> for &'a Vec3 {
    type Output = Vec3;

    fn div(self, rhs: f64) -> Vec3 {
        self.div_float(rhs)
    }
}

impl<> ops::DivAssign for Vec3 {
    fn div_assign(&mut self, rhs: Vec3) {
        self.x /= rhs.x;
        self.y /= rhs.y;
        self.z /= rhs.z;
    }
}

impl<> ops::DivAssign<f64> for Vec3 {
    fn div_assign(&mut self, rhs: f64) {
        self.x /= rhs;
        self.y /= rhs;
        self.z /= rhs;
    }
}

impl ops::Neg for Vec3 {
    type Output = Vec3;

    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}   

impl<'a> ops::Neg for &'a Vec3 {
    type Output = Vec3;

    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
} 

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test() {

        // compare
        assert_eq!(Vec3::new(1.0, 2.0, 4.0), Vec3::new(1.0, 2.0, 4.0));

        // add vec
        assert_eq!(Vec3::new(2.0, 3.0, 4.0), Vec3::new(1.0, 1.0, 1.0).add_vec(&Vec3::new(1.0, 2.0, 3.0))); 
        assert_eq!(Vec3::new(2.0, 3.0, 4.0), &Vec3::new(1.0, 1.0, 1.0) + &Vec3::new(1.0, 2.0, 3.0)); 
        assert_eq!(Vec3::new(2.0, 3.0, 4.0), Vec3::new(1.0, 1.0, 1.0) + Vec3::new(1.0, 2.0, 3.0)); 
        assert!(Vec3::new(2.0, 3.0, 4.0) != Vec3::new(1.0, 1.0, 1.0).add_vec(&Vec3::new(1.0, 1.0, 1.0)));

        // add assign
        let mut v1 = Vec3::new(0.0, 0.0, 1.0);
        let v2 = Vec3::new(1.0, 0.0, 0.0);
        v1 += v2;
        assert_eq!(v1, Vec3::new(1.0, 0.0, 1.0));

        // add float
        assert_eq!(Vec3::new(0.0, 1.0, 2.0).add_float(1.0), Vec3::new(1.0, 2.0, 3.0));
        assert!(Vec3::new(0.0, 1.0, 2.0).add_float(1.0) !=  Vec3::new(0.0, 0.0, 0.0));

        // sub vec
        assert_eq!(Vec3::new(0.0, -1.0, -2.0), Vec3::new(1.0, 1.0, 1.0).sub_vec(&Vec3::new(1.0, 2.0, 3.0))); 
        assert_eq!(Vec3::new(0.0, -1.0, -2.0), &Vec3::new(1.0, 1.0, 1.0) - &Vec3::new(1.0, 2.0, 3.0)); 
        assert_eq!(Vec3::new(0.0, -1.0, -2.0), Vec3::new(1.0, 1.0, 1.0) - Vec3::new(1.0, 2.0, 3.0)); 
        assert!(Vec3::new(0.0, 0.0, 4.0) != Vec3::new(1.0, 1.0, 1.0).sub_vec(&Vec3::new(1.0, 1.0, 1.0)));

        // sub assign
        let mut v1 = Vec3::new(0.0, 0.0, 1.0);
        let v2 = Vec3::new(1.0, 0.0, 0.0);
        v1 -= v2;
        assert_eq!(v1, Vec3::new(-1.0, 0.0, 1.0));

        // sub float
        assert_eq!(Vec3::new(0.0, 1.0, 2.0).sub_float(1.0), Vec3::new(-1.0, 0.0, 1.0));
        assert!(Vec3::new(0.0, 1.0, 2.0).sub_float(1.0) !=  Vec3::new(0.0, 0.0, 0.0));

        // mul vec
        assert_eq!(Vec3::new(2.0, 4.0, 6.0), Vec3::new(2.0, 2.0, 2.0).mul_vec(&Vec3::new(1.0, 2.0, 3.0))); 
        assert_eq!(Vec3::new(2.0, 4.0, 6.0), &Vec3::new(2.0, 2.0, 2.0) * &Vec3::new(1.0, 2.0, 3.0)); 
        assert_eq!(Vec3::new(2.0, 4.0, 6.0), Vec3::new(2.0, 2.0, 2.0) * Vec3::new(1.0, 2.0, 3.0)); 
        assert!(Vec3::new(2.0, 3.0, 4.0) != Vec3::new(2.0, 3.0, 2.0).mul_vec(&Vec3::new(1.0, 1.0, 1.0)));

        // mul assign
        let mut v1 = Vec3::new(3.0, 0.0, 3.0);
        let v2 = Vec3::new(2.0, 1.0, 3.0);
        v1 *= v2;
        assert_eq!(v1, Vec3::new(6.0, 0.0, 9.0));

        // mul float
        assert_eq!(Vec3::new(0.0, 1.0, 2.0).mul_float(2.0), Vec3::new(0.0, 2.0, 4.0));
        assert!(Vec3::new(0.0, 1.0, 2.0).mul_float(2.0) !=  Vec3::new(4.0, 2.0, 1.0));

        // div vec
        assert_eq!(Vec3::new(2.0, 1.0, 0.5), Vec3::new(2.0, 2.0, 2.0).div_vec(&Vec3::new(1.0, 2.0, 4.0))); 
        assert_eq!(Vec3::new(2.0, 1.0, 0.5), &Vec3::new(2.0, 2.0, 2.0) / &Vec3::new(1.0, 2.0, 4.0)); 
        assert_eq!(Vec3::new(2.0, 1.0, 0.5), Vec3::new(2.0, 2.0, 2.0) / Vec3::new(1.0, 2.0, 4.0)); 
        assert!(Vec3::new(2.0, 3.0, 4.0) != Vec3::new(2.0, 3.0, 2.0).div_vec(&Vec3::new(1.0, 1.0, 1.0)));

        // div assign
        let mut v1 = Vec3::new(4.0, 0.0, 9.0);
        let v2 = Vec3::new(2.0, 1.0, 3.0);
        v1 /= v2;
        assert_eq!(v1, Vec3::new(2.0, 0.0, 3.0));

        // div float
        assert_eq!(Vec3::new(0.0, 0.5, 1.0), Vec3::new(0.0, 1.0, 2.0).div_float(2.0));
        assert!(Vec3::new(4.0, 2.0, 1.0) != Vec3::new(0.0, 1.0, 2.0).div_float(2.0));
        
        // squared length
        let squared_length = 12.0_f64;
        assert_eq!(Vec3::new(2.0, 2.0, 2.0).squared_length(), squared_length);

        // length
        assert_eq!(Vec3::new(2.0, 2.0, 2.0).length(), squared_length.sqrt());

        // unit vector
        let mut v = Vec3::new(1.0, 1.0, 1.0);
        let length = 3.0_f64.sqrt();
        v.make_unit_vector();
        assert_eq!(v, Vec3::new(1.0/length, 1.0/length, 1.0/length));
    }
}