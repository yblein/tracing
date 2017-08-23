use std::ops::{Add, AddAssign, Sub, Mul, MulAssign, Div, Neg, Index, IndexMut};
use math::Axis;

#[derive(Copy, Clone, PartialEq, Debug, Default, Serialize, Deserialize)]
pub struct Vec3 {
	pub x: f32,
	pub y: f32,
	pub z: f32,
}

impl Vec3 {
	#[inline(always)]
	pub fn new(x: f32, y: f32, z: f32) -> Vec3 {
		Vec3 { x, y, z }
	}

	#[inline(always)]
	pub fn zero() -> Vec3 {
		Vec3 { x: 0.0, y: 0.0, z: 0.0 }
	}

	#[inline(always)]
	pub fn thrice(v: f32) -> Vec3 {
		Vec3 { x: v, y: v, z: v }
	}

	#[inline(always)]
	pub fn dot(lhs: Vec3, rhs: Vec3) -> f32 {
		(lhs * rhs).sum()
	}

	#[inline(always)]
	pub fn cross(lhs: Vec3, rhs: Vec3) -> Vec3 {
		Vec3 {
			x: lhs.y * rhs.z - lhs.z * rhs.y,
			y: lhs.z * rhs.x - lhs.x * rhs.z,
			z: lhs.x * rhs.y - lhs.y * rhs.x
		}
	}

	#[inline(always)]
	pub fn length(self) -> f32 {
		Vec3::dot(self, self).sqrt()
	}

	#[inline(always)]
	pub fn normalized(self) -> Vec3 {
		self / self.length()
	}

	#[inline(always)]
	pub fn dir_and_dist(p1: Vec3, p2: Vec3) -> (Vec3, f32) {
		let d = p2 - p1;
		let l = d.length();
		(d / l, l)
	}

	#[inline(always)]
	pub fn min(lhs: Vec3, rhs: Vec3) -> Vec3 {
		Vec3 {
			x: lhs.x.min(rhs.x),
			y: lhs.y.min(rhs.y),
			z: lhs.z.min(rhs.z),
		}
	}

	#[inline(always)]
	pub fn max(lhs: Vec3, rhs: Vec3) -> Vec3 {
		Vec3 {
			x: lhs.x.max(rhs.x),
			y: lhs.y.max(rhs.y),
			z: lhs.z.max(rhs.z),
		}
	}

	#[inline(always)]
	pub fn sum(self) -> f32 {
		self.x + self.y + self.z
	}

	#[inline(always)]
	pub fn avg(self) -> f32 {
		self.sum() / 3.0
	}

	#[inline(always)]
	pub fn min_elem(self) -> f32 {
		self.x.min(self.y).min(self.z)
	}

	#[inline(always)]
	pub fn max_elem(self) -> f32 {
		self.x.max(self.y).max(self.z)
	}

	#[inline(always)]
	pub fn has_nan(self) -> bool {
		self.x.is_nan() || self.y.is_nan() || self.z.is_nan()
	}

	#[inline(always)]
	pub fn all_finite(self) -> bool {
		self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
	}

	#[inline(always)]
	pub fn map<F>(self, f: F) -> Vec3
		where F : Fn(f32) -> f32
	{
		Vec3 {
			x: f(self.x),
			y: f(self.y),
			z: f(self.z),
		}
	}
}

impl Add for Vec3 {
	type Output = Vec3;
	#[inline(always)]
	fn add(self, rhs: Vec3) -> Vec3 {
		Vec3 { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
	}
}

impl AddAssign for Vec3 {
	#[inline(always)]
	fn add_assign(&mut self, rhs: Vec3) {
		*self = *self + rhs;
	}
}

impl Sub for Vec3 {
	type Output = Vec3;
	#[inline(always)]
	fn sub(self, rhs: Vec3) -> Vec3 {
		Vec3 { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
	}
}

impl Mul for Vec3 {
	type Output = Vec3;
	#[inline(always)]
	fn mul(self, rhs: Vec3) -> Vec3 {
		Vec3 { x: self.x * rhs.x, y: self.y * rhs.y, z: self.z * rhs.z }
	}
}

impl Mul<f32> for Vec3 {
	type Output = Vec3;
	#[inline(always)]
	fn mul(self, rhs: f32) -> Vec3 {
		Vec3 { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
	}
}

impl Mul<Vec3> for f32 {
	type Output = Vec3;
	#[inline(always)]
	fn mul(self, rhs: Vec3) -> Vec3 {
		Vec3 { x: self * rhs.x, y: self * rhs.y, z: self * rhs.z }
	}
}

impl MulAssign for Vec3 {
	#[inline(always)]
	fn mul_assign(&mut self, rhs: Vec3) {
		*self = *self * rhs;
	}
}

impl Div for Vec3 {
	type Output = Vec3;
	#[inline(always)]
	fn div(self, rhs: Vec3) -> Vec3 {
		Vec3 { x: self.x / rhs.x, y: self.y / rhs.y, z: self.z / rhs.z }
	}
}

impl Div<f32> for Vec3 {
	type Output = Vec3;
	#[inline(always)]
	fn div(self, rhs: f32) -> Vec3 {
		let s = 1.0 / rhs;
		self * s
	}
}

impl Div<Vec3> for f32 {
	type Output = Vec3;
	#[inline(always)]
	fn div(self, rhs: Vec3) -> Vec3 {
		Vec3 { x: self / rhs.x, y: self / rhs.y, z: self / rhs.z }
	}
}

impl Neg for Vec3 {
	type Output = Vec3;
	#[inline(always)]
	fn neg(self) -> Vec3 {
		Vec3 { x: -self.x, y: -self.y, z: -self.z }
	}
}

impl Index<Axis> for Vec3 {
	type Output = f32;
	#[inline(always)]
	fn index(&self, index: Axis) -> &f32 {
		match index {
			Axis::X => &self.x,
			Axis::Y => &self.y,
			Axis::Z => &self.z,
		}
	}
}

impl IndexMut<Axis> for Vec3 {
	#[inline(always)]
	fn index_mut(&mut self, index: Axis) -> &mut f32 {
		match index {
			Axis::X => &mut self.x,
			Axis::Y => &mut self.y,
			Axis::Z => &mut self.z,
		}
	}
}
