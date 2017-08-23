use math::*;

/// Axis-Aligned Bounding Box
#[derive(Copy, Clone, Debug, Serialize, Deserialize)]
pub struct AABB {
	pub min: Vec3,
	pub max: Vec3,
}

impl AABB {
	#[inline(always)]
	pub fn intersect_fast(&self, ray: Ray, inv_dir: Vec3) -> (f32, f32) {
		let t_min = (self.min - ray.origin) * inv_dir;
		let t_max = (self.max - ray.origin) * inv_dir;
		let t1 = Vec3::min(t_min, t_max);
		let t2 = Vec3::max(t_min, t_max);
		let t_near = t1.x.max(t1.y).max(t1.z);
		let t_far  = t2.x.min(t2.y).min(t2.z);
		if t_near > t_far {
			return (-1.0, -1.0);
		}
		(t_near, t_far)
		//if t_near < 0.0 { t_far } else { t_near }
	}

	pub fn intersect(&self, ray: Ray) -> (f32, f32) {
		let inv_dir = 1.0 / ray.direction;
		self.intersect_fast(ray, inv_dir)
	}

	pub fn zero() -> AABB {
		AABB { min: Vec3::zero(), max: Vec3::zero() }
	}

	pub fn empty() -> AABB {
		AABB { min: Vec3::thrice(INFINITY), max: Vec3::thrice(NEG_INFINITY) }
	}

	pub fn from_point(p: Vec3) -> AABB {
		AABB { min: p, max: p }
	}

	pub fn extend_point(&mut self, p: Vec3) {
		self.min = Vec3::min(self.min, p);
		self.max = Vec3::max(self.max, p);
	}

	pub fn union(&self, b: &AABB) -> AABB {
		AABB {
			min: Vec3::min(self.min, b.min),
			max: Vec3::max(self.max, b.max),
		}
	}

	pub fn longuest_axis(&self) -> Axis {
		let dx = self.max.x - self.min.x;
		let dy = self.max.y - self.min.y;
		let dz = self.max.z - self.min.z;

		if dx > dy {
			if dx > dz { Axis::X } else { Axis::Z }
		} else {
			if dy > dz { Axis::Y } else { Axis::Z }
		}
	}

	pub fn center(&self) -> Vec3 {
		(self.min + self.max) * 0.5
	}

	/*
	fn normal_at(&self, p: Vec3) -> Vec3 {
        if p.x < self.min.x + EPSILON {
			Vec3::new(-1.0, 0.0, 0.0)
		} else if p.x > self.max.x - EPSILON {
			Vec3::new(1.0, 0.0, 0.0)
		} else if p.y < self.min.y + EPSILON {
			Vec3::new(0.0, -1.0, 0.0)
   		} else if p.y > self.max.y - EPSILON {
			Vec3::new(0.0, 1.0, 0.0)
   		} else if p.z < self.min.z + EPSILON {
			Vec3::new(0.0, 0.0, -1.0)
   		} else {
			Vec3::new(0.0, 0.0, 1.0)
		}
	}
	*/

	pub fn surface_area(&self) -> f32 {
		let d = self.max - self.min;
		2.0 * (d.x * d.y + d.x * d.z + d.y * d.z)
	}
}
