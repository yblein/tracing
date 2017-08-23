use math::*;

#[derive(Copy, Clone)]
pub struct Intersection {
	pub distance: f32,
	pub normal: Vec3,
	pub uv: (f32, f32),
}

pub trait Surface {
	fn intersect(&self, ray: Ray) -> Option<Intersection>;
	fn aabb(&self) -> AABB;
}
