pub mod vec3;
pub mod mat4;
pub mod frame;
pub mod ray;
pub mod aabb;

pub use vec3::Vec3;
pub use mat4::Mat4;
pub use frame::Frame;
pub use ray::Ray;
pub use aabb::AABB;
pub use std::f32::{INFINITY, NEG_INFINITY};
pub use std::f32::consts::*;

pub const EPSILON: f32 = 1e-5;
pub const INV_PI: f32 = FRAC_1_PI;
pub const INV_2_PI: f32 = 0.5 * FRAC_1_PI;
pub const INV_4_PI: f32 = 0.25 * FRAC_1_PI;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Axis { X, Y, Z }

pub fn lerp(a: Vec3, b: Vec3, t: f32) -> Vec3 {
	(1.0 - t) * a + t * b
}

pub fn bilerp(x00: Vec3, x01: Vec3, x10: Vec3, x11: Vec3, (u, v): (f32, f32)) -> Vec3 {
	lerp(lerp(x00, x01, u), lerp(x10, x11, u), v)
}
