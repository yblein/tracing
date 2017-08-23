use math::*;
use warp;

#[derive(Clone)]
pub struct Camera {
	pos: Vec3,
	transform: Mat4,

	resolution: (usize, usize),
	ratio: f32,
	pixel_size: (f32, f32),

	fov_rad: f32,
	plane_dist: f32,

	aperture_radius: f32,
	focus_dist: f32,

	pub tonemap: Tonemap,
}

impl Camera {
	pub fn new(transform: &Mat4, resolution: (usize, usize), fov: f32, tonemap: Tonemap, aperture_radius: Option<f32>, focus_dist: Option<f32>) -> Camera {
		let fov_rad = fov * PI / 180.0;
		let plane_dist = 1.0 / (fov_rad * 0.5).tan();

		Camera {
			pos: transform.transform_point(Vec3::zero()),
			transform: transform.clone(),
			resolution,
			ratio: resolution.1 as f32 / resolution.0 as f32,
			pixel_size: (1.0 / resolution.0 as f32, 1.0 / resolution.1 as f32),
			fov_rad,
			plane_dist,
			aperture_radius: aperture_radius.unwrap_or(0.0),
			focus_dist: focus_dist.unwrap_or(plane_dist),
			tonemap,
		}
	}

	pub fn make_ray(&self, pixel: (usize, usize), img_uv: (f32, f32), lens_uv: (f32, f32)) -> Ray {
		let pj = warp::tent(img_uv);
		let img_plane_pos = Vec3 {
			x: -1.0       + (pixel.0 as f32 + pj.0 + 0.5) * 2.0 * self.pixel_size.0,
			y: self.ratio - (pixel.1 as f32 + pj.1 + 0.5) * 2.0 * self.pixel_size.0,
			z: self.plane_dist,
		};
		let focus_plane_pos = img_plane_pos * (self.focus_dist / img_plane_pos.z);
		let lj = warp::uniform_disk(lens_uv);
		let lens_pos = Vec3::new(lj.0 * self.aperture_radius, lj.1 * self.aperture_radius, 0.0);
		let local_dir = (focus_plane_pos - lens_pos).normalized();

		Ray {
			origin: self.transform.transform_point(lens_pos),
			direction: self.transform.transform_vector(local_dir).normalized(),
		}
	}

	pub fn resolution(&self) -> (usize, usize) {
		self.resolution
	}

	pub fn set_focus_dist(&mut self, focus_dist: Option<f32>) {
		self.focus_dist = focus_dist.unwrap_or(self.plane_dist)
	}
}

pub type Tonemap = fn(Vec3) -> Vec3;

pub fn gamma(c: Vec3) -> Vec3 {
	c.map(|v| v.min(1.0).powf(1.0 / 2.2))
}

pub fn filmic(c: Vec3) -> Vec3 {
	c.map(|v| {
		let x = (v - 0.004).max(0.0);
		(x*(6.2*x + 0.5)) / (x*(6.2*x + 1.7) + 0.06)
	})
}
