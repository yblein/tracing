use math::*;

/// Warp a sample from [0:1[² on the unit hemisphere around the Y-axis uniformly
pub fn uniform_hemisphere((u, v): (f32, f32)) -> Vec3 {
	let r = (1.0 - u * u).sqrt();
	let phi = 2.0 * PI * v;
	let x = r * phi.cos();
	let z = r * phi.sin();
	Vec3::new(x, u, z)
}

pub fn uniform_hemisphere_pdf(_: Vec3) -> f32 {
	INV_2_PI
}

/// Warp a sample from [0:1[² on the unit hemisphere around the Y-axis with a cosine-weight
pub fn cosine_hemisphere((u, v): (f32, f32)) -> Vec3 {
	let (x, z) = uniform_disk((u, v));
	let y = (1.0 - u).sqrt();
	Vec3::new(x, y, z)
}

pub fn cosine_hemisphere_pdf(v: Vec3) -> f32 {
	v.y * INV_PI
}

/// Warp a sample from [0:1[² on the unit sphere
pub fn uniform_sphere((u, v): (f32, f32)) -> Vec3 {
	let y = 1.0 - 2.0 * u;
	let r = (1.0 - y * y).sqrt();
	let phi = 2.0 * PI * v;
	Vec3::new(r * phi.cos(), y, r * phi.sin())
}

pub fn uniform_sphere_pdf(_: Vec3) -> f32 {
	INV_4_PI
}

/// Warp a sample from [0:1[² on the unit disk
pub fn uniform_disk((u, v): (f32, f32)) -> (f32, f32) {
	let r = u.sqrt();
	let theta = 2.0 * PI * v;
	(r * theta.cos(), r * theta.sin())
}

/*
pub fn beckmann(u: f32, v: f32, alpha: f32) -> Vec3 {
	let phi = u * 2.0 * PI;
	let p = v * phi / (2.0 * PI);
	let lg = (1.0-(2.0*PI*p/phi)).log10();
	let denom = (1.0 - (alpha*alpha*lg)).sqrt();
	let theta = (1.0/denom).acos();

	let phi = 2.0 * PI * v;
	let r = theta.sin();

	let x = r * phi.cos();
	let y = theta.cos();
	let z = r * phi.sin();

	Vec3::new(x, y, z).normalized()
}
*/

/// Warp a sample from [0;1[ to a tent over [-1;1[
pub fn tent1d(u: f32) -> f32 {
	let x = 2.0 * u;
	if x < 1.0 {
		x.sqrt() - 1.0
	} else {
		1.0 - (2.0 - x).sqrt()
	}
}

/// Warp a sample from [0;1[² to a tent over [-1;1[²
pub fn tent((u, v): (f32, f32)) -> (f32, f32) {
	(tent1d(u), tent1d(v))
}
