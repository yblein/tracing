use math::Vec3;

/// Right-handed orthonormal basis -- Y is up
pub struct Frame(Vec3, Vec3, Vec3);

impl Frame {
	pub fn from_up(normal: Vec3) -> Frame {
		let tangent = if normal.x.abs() > normal.y.abs() {
			Vec3::new(normal.z, 0.0, -normal.x) / (normal.x * normal.x + normal.z * normal.z).sqrt()
		} else {
			Vec3::new(0.0, -normal.z, normal.y) / (normal.y * normal.y + normal.z * normal.z).sqrt()
		};
		let bitangent = Vec3::cross(normal, tangent);
		Frame(tangent, normal, bitangent)
	}

	#[inline(always)]
	pub fn to_world(&self, v: Vec3) -> Vec3 {
		self.0 * v.x + self.1 * v.y + self.2 * v.z
	}

	#[inline(always)]
	pub fn to_local(&self, v: Vec3) -> Vec3 {
		Vec3::new(Vec3::dot(v, self.0), Vec3::dot(v, self.1), Vec3::dot(v, self.2))
	}
}
