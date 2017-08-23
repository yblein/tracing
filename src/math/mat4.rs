use std::ops::{Index, IndexMut, Mul};
use math::{Vec3, PI};

/// row-major 4x4 matrix
#[derive(Debug, PartialEq, Clone, Copy)]
// TODO: remove pub
pub struct Mat4(pub(crate) [f32; 16]);

impl Mat4 {
	pub fn zero() -> Mat4 {
		Mat4([0f32; 16])
	}

	pub fn identity() -> Mat4 {
		Mat4([
			1.0, 0.0, 0.0, 0.0,
			0.0, 1.0, 0.0, 0.0,
			0.0, 0.0, 1.0, 0.0,
			0.0, 0.0, 0.0, 1.0,
		])
	}

	pub fn inverse(&self) -> Mat4 {
		// Code derived from MESA, see https://stackoverflow.com/a/1148405
		let a = &self.0;
		let mut inv = [0.0; 16];

		inv[ 0] =  a[5]*a[10]*a[15] - a[5]*a[11]*a[14] - a[9]*a[6]*a[15] + a[9]*a[7]*a[14] + a[13]*a[6]*a[11] - a[13]*a[7]*a[10];
		inv[ 1] = -a[1]*a[10]*a[15] + a[1]*a[11]*a[14] + a[9]*a[2]*a[15] - a[9]*a[3]*a[14] - a[13]*a[2]*a[11] + a[13]*a[3]*a[10];
		inv[ 2] =  a[1]*a[ 6]*a[15] - a[1]*a[ 7]*a[14] - a[5]*a[2]*a[15] + a[5]*a[3]*a[14] + a[13]*a[2]*a[ 7] - a[13]*a[3]*a[ 6];
		inv[ 3] = -a[1]*a[ 6]*a[11] + a[1]*a[ 7]*a[10] + a[5]*a[2]*a[11] - a[5]*a[3]*a[10] - a[ 9]*a[2]*a[ 7] + a[ 9]*a[3]*a[ 6];
		inv[ 4] = -a[4]*a[10]*a[15] + a[4]*a[11]*a[14] + a[8]*a[6]*a[15] - a[8]*a[7]*a[14] - a[12]*a[6]*a[11] + a[12]*a[7]*a[10];
		inv[ 5] =  a[0]*a[10]*a[15] - a[0]*a[11]*a[14] - a[8]*a[2]*a[15] + a[8]*a[3]*a[14] + a[12]*a[2]*a[11] - a[12]*a[3]*a[10];
		inv[ 6] = -a[0]*a[ 6]*a[15] + a[0]*a[ 7]*a[14] + a[4]*a[2]*a[15] - a[4]*a[3]*a[14] - a[12]*a[2]*a[ 7] + a[12]*a[3]*a[ 6];
		inv[ 8] =  a[4]*a[ 9]*a[15] - a[4]*a[11]*a[13] - a[8]*a[5]*a[15] + a[8]*a[7]*a[13] + a[12]*a[5]*a[11] - a[12]*a[7]*a[ 9];
		inv[ 7] =  a[0]*a[ 6]*a[11] - a[0]*a[ 7]*a[10] - a[4]*a[2]*a[11] + a[4]*a[3]*a[10] + a[ 8]*a[2]*a[ 7] - a[ 8]*a[3]*a[ 6];
		inv[ 9] = -a[0]*a[ 9]*a[15] + a[0]*a[11]*a[13] + a[8]*a[1]*a[15] - a[8]*a[3]*a[13] - a[12]*a[1]*a[11] + a[12]*a[3]*a[ 9];
		inv[10] =  a[0]*a[ 5]*a[15] - a[0]*a[ 7]*a[13] - a[4]*a[1]*a[15] + a[4]*a[3]*a[13] + a[12]*a[1]*a[ 7] - a[12]*a[3]*a[ 5];
		inv[11] = -a[0]*a[ 5]*a[11] + a[0]*a[ 7]*a[ 9] + a[4]*a[1]*a[11] - a[4]*a[3]*a[ 9] - a[ 8]*a[1]*a[ 7] + a[ 8]*a[3]*a[ 5];
		inv[12] = -a[4]*a[ 9]*a[14] + a[4]*a[10]*a[13] + a[8]*a[5]*a[14] - a[8]*a[6]*a[13] - a[12]*a[5]*a[10] + a[12]*a[6]*a[ 9];
		inv[13] =  a[0]*a[ 9]*a[14] - a[0]*a[10]*a[13] - a[8]*a[1]*a[14] + a[8]*a[2]*a[13] + a[12]*a[1]*a[10] - a[12]*a[2]*a[ 9];
		inv[14] = -a[0]*a[ 5]*a[14] + a[0]*a[ 6]*a[13] + a[4]*a[1]*a[14] - a[4]*a[2]*a[13] - a[12]*a[1]*a[ 6] + a[12]*a[2]*a[ 5];
		inv[15] =  a[0]*a[ 5]*a[10] - a[0]*a[ 6]*a[ 9] - a[4]*a[1]*a[10] + a[4]*a[2]*a[ 9] + a[ 8]*a[1]*a[ 6] - a[ 8]*a[2]*a[ 5];

		let det = a[0] * inv[0] + a[1] * inv[4] + a[2] * inv[8] + a[3] * inv[12];
		debug_assert!(det != 0.0);
		let inv_det = 1.0 / det;

		for x in inv.iter_mut() {
			*x *= inv_det;
		}

		Mat4(inv)
	}

	pub fn scale(v: Vec3) -> Mat4 {
		Mat4([
			v.x, 0.0, 0.0, 0.0,
			0.0, v.y, 0.0, 0.0,
			0.0, 0.0, v.z, 0.0,
			0.0, 0.0, 0.0, 1.0,
		])
	}

	pub fn translate(v: Vec3) -> Mat4 {
		Mat4([
			1.0, 0.0, 0.0, v.x,
			0.0, 1.0, 0.0, v.y,
			0.0, 0.0, 1.0, v.z,
			0.0, 0.0, 0.0, 1.0,
		])
	}

	pub fn rot_yxz(v: Vec3) -> Mat4 {
		let r = v * (PI / 180.0);
		let c = [f32::cos(r.x), f32::cos(r.y), f32::cos(r.z)];
		let s = [f32::sin(r.x), f32::sin(r.y), f32::sin(r.z)];

		Mat4([
			c[1]*c[2] - s[1]*s[0]*s[2], -c[1]*s[2] - s[1]*s[0]*c[2], -s[1]*c[0], 0.0,
			                 c[0]*s[2],                   c[0]*c[2],      -s[0], 0.0,
			s[1]*c[2] + c[1]*s[0]*s[2], -s[1]*s[2] + c[1]*s[0]*c[2],  c[1]*c[0], 0.0,
			                       0.0,                         0.0,        0.0, 1.0
		])
	}

	pub fn transform_point(&self, p: Vec3) -> Vec3 {
		let a = &self;
		Vec3 {
			x: a[(0,0)] * p.x + a[(0,1)] * p.y + a[(0,2)] * p.z + a[(0,3)],
			y: a[(1,0)] * p.x + a[(1,1)] * p.y + a[(1,2)] * p.z + a[(1,3)],
			z: a[(2,0)] * p.x + a[(2,1)] * p.y + a[(2,2)] * p.z + a[(2,3)],
		}
	}

	pub fn transform_vector(&self, p: Vec3) -> Vec3 {
		let a = &self;
		Vec3 {
			x: a[(0,0)] * p.x + a[(0,1)] * p.y + a[(0,2)] * p.z,
			y: a[(1,0)] * p.x + a[(1,1)] * p.y + a[(1,2)] * p.z,
			z: a[(2,0)] * p.x + a[(2,1)] * p.y + a[(2,2)] * p.z,
		}
	}

	pub fn look_at(pos: Vec3, look_at: Vec3, up: Vec3) -> Mat4 {
		let f = (look_at - pos).normalized();
		let r = Vec3::cross(f, up).normalized();
		let u = Vec3::cross(r, f).normalized();

		Mat4([
			r.x, u.x, f.x, pos.x,
			r.y, u.y, f.y, pos.y,
			r.z, u.z, f.z, pos.z,
			0.0, 0.0, 0.0, 1.0
		])
	}
}

impl Mul for Mat4 {
	type Output = Mat4;
	fn mul(self, rhs: Mat4) -> Mat4 {
		let a = &self.0;
		let b = &rhs.0;
		let mut result = [0.0; 16];

		for i in 0..4 {
			for t in 0..4 {
				result[i*4 + t] =
					a[i*4 + 0]*b[0*4 + t] +
					a[i*4 + 1]*b[1*4 + t] +
					a[i*4 + 2]*b[2*4 + t] +
					a[i*4 + 3]*b[3*4 + t];
			}
		}

		Mat4(result)
	}
}

impl Index<(usize, usize)> for Mat4 {
	type Output = f32;

	fn index<'a>(&'a self, coord: (usize, usize)) -> &'a f32 {
		&self.0[4 * coord.0 + coord.1]
	}
}

impl IndexMut<(usize, usize)> for Mat4 {
	fn index_mut<'a>(&'a mut self, coord: (usize, usize)) -> &'a mut f32 {
		&mut self.0[4 * coord.0 + coord.1]
	}
}

#[test]
fn test_inverse_identity() {
	assert!(Mat4::identity().inverse() == Mat4::identity());
}
