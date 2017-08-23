use distribution::Distribution2D;
use math::*;
use texture::*;
use geometry::Surface;

pub struct DirectSample {
	pub dir: Vec3,
	pub dist: f32,
	pub pdf: f32,
}

pub trait SampleDirectSurface: Surface {
	fn sample_direct(&self, p: Vec3, uv: (f32, f32)) -> DirectSample;
	fn pdf_direct(&self, dir: Vec3, dist: f32) -> f32;
}

pub trait Light {
	fn eval_direct(&self, dir: Vec3) -> Vec3;

	fn sample_direct(&self, p: Vec3, uv: (f32, f32)) -> (Vec3, DirectSample);
	fn pdf_direct(&self, dir: Vec3, dist: f32) -> f32;
}

pub struct EnvMap {
	img: Image,
	dist: Distribution2D,
	transform: Mat4,
	inv_transform: Mat4,
}

impl EnvMap {
	pub fn from_image(img: Image, transform: &Mat4) -> EnvMap {
		// Construct a row-major 2d distribution based on texels luminance.
		// It will be used to importance sample texels according to their contribution.
		// The sin(theta) factor counteracts the deformation of equi-rectangular mapping.
		let mut weights = vec![0f32; img.width * img.height];
		for y in 0..img.height {
			// Compute sin(theta) for the current row, accounting for the half-pixel offset
			// due to conversion from discrete to continuous coordinates
			let sin_theta = ((y as f32 + 0.5) * PI / img.height as f32).sin();
			for x in 0..img.width {
				weights[y * img.width + x] = luminance(img.get(x, y)) * sin_theta;
			}
		}
		let dist = Distribution2D::new(weights, img.width, img.height);

		// TODO: only keep rotation from transform
		EnvMap {
			img,
			dist,
			transform: transform.clone(),
			inv_transform: transform.inverse(),
		}
	}

	fn direction_to_uv(&self, d: Vec3) -> ((f32, f32), f32) {
		let l = self.inv_transform.transform_vector(d);
		let u = l.z.atan2(l.x) * INV_2_PI + 0.5;
		let v = (-l.y).acos() * INV_PI;
		let sin_theta = (1.0 - l.y * l.y).max(EPSILON).sqrt();
		((u, v), sin_theta)
	}

	fn uv_to_direction(&self, (u, v): (f32, f32)) -> (Vec3, f32) {
		let phi = (u - 0.5) * 2.0 * PI;
		let theta = v * PI;
		let (sin_theta, cos_theta) = theta.sin_cos();
		let (sin_phi, cos_phi) = phi.sin_cos();
		let local_dir = Vec3::new(sin_theta * cos_phi, -cos_theta, sin_theta * sin_phi);
		let dir = self.transform.transform_vector(local_dir);
		(dir, sin_theta)
	}
}

impl Light for EnvMap {
	fn eval_direct(&self, dir: Vec3) -> Vec3 {
		let (uv, _) = self.direction_to_uv(dir);
		self.img.eval(uv)
	}

	fn sample_direct(&self, _p: Vec3, (u, v): (f32, f32)) -> (Vec3, DirectSample) {
		let ((x, y), tex_pdf) = self.dist.sample(u, v);

		let u = (x as f32 + 0.5) / self.img.width  as f32;
		let v = 1.0 - (y as f32 + 0.5) / self.img.height as f32;
		let (dir, sin_theta) = self.uv_to_direction((u, v));

		let img_size = (self.img.width * self.img.height) as f32;
		let dir_pdf = tex_pdf * img_size / (2.0 * PI * PI * sin_theta);
		let emission = self.img.eval((u, v));

		(emission, DirectSample { dir, dist: INFINITY, pdf: dir_pdf })
	}

	fn pdf_direct(&self, dir: Vec3, _dist: f32) -> f32 {
		let ((u, v), sin_theta) = self.direction_to_uv(dir);
		let x = (u * self.img.width  as f32) as usize;
		let y = ((1.0 - v) * self.img.height as f32) as usize;

		let img_size = (self.img.width * self.img.height) as f32;
		let tex_pdf = self.dist.pdf(x, y);

		tex_pdf * img_size / (2.0 * PI * PI * sin_theta)
	}
}

pub struct AreaLight {
	pub surface: Box<SampleDirectSurface + Send + Sync>,
	pub emission: Vec3,
}

impl Light for AreaLight {
	fn eval_direct(&self, _dir: Vec3) -> Vec3 {
		self.emission
	}

	fn sample_direct(&self, p: Vec3, uv: (f32, f32)) -> (Vec3, DirectSample) {
		(self.emission, self.surface.sample_direct(p, uv))
	}

	fn pdf_direct(&self, dir: Vec3, dist: f32) -> f32 {
		self.surface.pdf_direct(dir, dist)
	}
}
