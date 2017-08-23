use geometry::*;
use math::*;
use mat4::*;
use warp::*;
use light::*;

#[derive(Copy, Clone)]
pub struct Sphere {
	pub radius: f32,
	pub position: Vec3,
}

impl Sphere {
	pub fn new(radius: f32, position: Vec3) -> Sphere {
		Sphere { radius: radius, position: position }
	}
}

impl Surface for Sphere {
	// returns distance, infinity if no hit.
	fn intersect(&self, ray: Ray) -> Option<Intersection> {
		let to_sphere = ray.origin - self.position;
		let b = 2.0 * Vec3::dot(to_sphere, ray.direction);
		let c = Vec3::dot(to_sphere, to_sphere) - self.radius * self.radius;
		let discriminant = b * b - 4.0 * c;
		if discriminant > 0.0 {
			let t = {
				let s = discriminant.sqrt();
				let t1 = (-b - s) / 2.0;
				if t1 > 0.0 { t1 } else { (-b + s) / 2.0 }
			};
			if t > 0.0 {
				let normal = (ray.point_at(t) - self.position) / self.radius;
				let phi = normal.z.atan2(normal.x);
				let theta = normal.y.acos();
				let u = phi * INV_2_PI;
				let v = theta * INV_PI;
				return Some(Intersection {
					distance: t,
					normal: normal,
					// TODO: floor not necessary?
					uv: (u - u.floor(), v - v.floor()),
				});
			}
		}
		return None;
	}

	fn aabb(&self) -> AABB {
		AABB {
			min: self.position + Vec3::thrice(-self.radius),
			max: self.position + Vec3::thrice( self.radius),
		}
	}
}

pub struct Parallelogram {
	position: Vec3,
	edge1: Vec3,
	edge2: Vec3,
	// cache
	normal: Vec3,
	l1: f32,
	l2: f32,
	area: f32,
}

impl Parallelogram {
	/// Create a parallelogram by transforming the unit parallelogram
	///
	/// The unit parallelogram is a square of width 1 centered in the plane XZ
	pub fn unit_transform(transform: &Mat4) -> Parallelogram {
		let edge1 = transform.transform_vector(Vec3::new(0.0, 0.0, 1.0));
		let edge2 = transform.transform_vector(Vec3::new(1.0, 0.0, 0.0));
		let position = transform.transform_point(Vec3::new(-0.5, 0.0, -0.5));

		Parallelogram::new(position, edge1, edge2)
	}

	pub fn new(position: Vec3, edge1: Vec3, edge2: Vec3) -> Parallelogram {
		let l1 = Vec3::dot(edge1, edge1);
		let l2 = Vec3::dot(edge2, edge2);
		Parallelogram {
			position: position,
			edge1: edge1,
			edge2: edge2,
			normal: Vec3::cross(edge2, edge1).normalized(),
			l1: l1,
			l2: l2,
			area: (l1 * l2).sqrt(),
		}
	}

	pub fn from_square(center: Vec3, normal: Vec3, side_length: f32) -> Parallelogram {
		let a = Vec3::new(normal.y, normal.z, -normal.x).normalized();
		let b = Vec3::cross(a, normal);
		let d1 = a * (side_length * 0.5);
		let d2 = b * (side_length * 0.5);
		Parallelogram::new(center - d1 - d2, a * side_length, b * side_length)
	}
}

impl Surface for Parallelogram {
	fn intersect(&self, ray: Ray) -> Option<Intersection> {
		let nd = Vec3::dot(self.normal, ray.direction);

		let t = (Vec3::dot(self.normal, self.position) - Vec3::dot(self.normal, ray.origin)) / nd;
		if t < 0.0 {
			return None;
		}

		let p = ray.point_at(t) - self.position;
		let u = Vec3::dot(self.edge1, p);
		let v = Vec3::dot(self.edge2, p);

		if !(0.0 <= u && u <= self.l1 && 0.0 <= v && v <= self.l2) {
			return None;
		}

		// normalize uv
		let u = u / self.l1;
		let v = v / self.l2;

		Some(Intersection {
			distance: t,
			normal: self.normal,
			uv: (u - u.floor(), v - v.floor()),
		})
	}

	fn aabb(&self) -> AABB {
		let mut aabb = AABB::from_point(self.position);
		aabb.extend_point(self.position + self.edge1);
		aabb.extend_point(self.position + self.edge2);
		aabb.extend_point(self.position + self.edge1 + self.edge2);
		return aabb;
	}
}

impl SampleDirectSurface for Parallelogram {
	fn sample_direct(&self, p: Vec3, (u, v): (f32, f32)) -> DirectSample {
		//if Vec3::dot(self.normal, p - self.position) <= 0.0 {
		//	return DirectSample {
		//		dir: Vec3::zero(),
		//		dist: 0.0,
		//		pdf: 0.0,
		//	}
		//}

		let q = self.position + self.edge1 * u + self.edge2 * v;
		let (dir, dist) = Vec3::dir_and_dist(p, q);
		//let cos_theta = -Vec3::dot(self.normal, dir);
		let cos_theta = Vec3::dot(self.normal, dir).abs();
		let pdf = dist * dist / (cos_theta * self.area);

		DirectSample { dir, dist, pdf }
	}

	fn pdf_direct(&self, dir: Vec3, dist: f32) -> f32 {
		//let cos_theta = -Vec3::dot(self.normal, dir);
		//if cos_theta <= 0.0 {
		//	0.0
		//} else {
		//	dist * dist / (self.area * cos_theta)
		//}

		let cos_theta = Vec3::dot(self.normal, dir).abs();
		dist * dist / (self.area * cos_theta)
	}
}

pub struct Disk {
	center: Vec3,
	normal: Vec3,
	radius: f32,
	// cache
	u_axis: Vec3,
	v_axis: Vec3,
}

impl Disk {
	pub fn new(center: Vec3, normal: Vec3, radius: f32) -> Disk {
		let u_axis = Vec3::new(normal.y, normal.z, -normal.x).normalized();
		let v_axis = Vec3::cross(u_axis, normal);

		Disk {
			center: center,
			normal: normal,
			radius: radius,
			u_axis: u_axis,
			v_axis: v_axis,
		}
	}
}

impl Surface for Disk {
	fn intersect(&self, ray: Ray) -> Option<Intersection> {
		let nd = Vec3::dot(self.normal, ray.direction);

		let t = (Vec3::dot(self.normal, self.center) - Vec3::dot(self.normal, ray.origin)) / nd;
		if t < 0.0 {
			return None;
		}

		let p = ray.point_at(t);
		let u = Vec3::dot(self.u_axis, p);
		let v = Vec3::dot(self.v_axis, p);

		if u * u + v * v > self.radius * self.radius {
			return None;
		}

		Some(Intersection {
			distance: t,
			normal: self.normal,
			uv: (u - u.floor(), v - v.floor()),
		})
	}

	fn aabb(&self) -> AABB {
		unimplemented!();
	}
}
