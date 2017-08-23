use texture::Texture;
use math::*;
use warp::*;
use std::sync::Arc;

pub struct BSDFSample {
	pub direction: Vec3,
	pub pdf: f32,
	/// Importance weight of the sample (i.e. the value of the BSDF divided by
	/// the probability density) multiplied by the cosine falloff factor with
	/// respect to the sampled direction
	pub weight: Vec3,
	pub is_specular: bool,
}

const NULL_SAMPLE: BSDFSample = BSDFSample {
	direction: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
	pdf: 0.0,
	weight: Vec3 { x: 0.0, y: 0.0, z: 0.0 },
	is_specular: false,
};

pub trait Material: Sync + Send {
	fn sample(&self, _dir_in: Vec3, _uv: (f32, f32), _rnd: Vec3) -> BSDFSample;

	fn eval(&self, _dir_in: Vec3, _dir_out: Vec3, _uv: (f32, f32)) -> Vec3 {
		Vec3::zero()
	}

	fn pdf(&self, _dir_in: Vec3, _dir_out: Vec3, _uv: (f32, f32)) -> f32 {
		0.0
	}

	fn is_purely_specular(&self) -> bool {
		true
	}
}

#[derive(Clone, Copy)]
pub struct ComplexIOR {
	pub eta: Vec3,
	pub k: Vec3,
}

pub struct Diffuse {
	pub albedo: Texture,
}

impl Material for Diffuse {
	fn sample(&self, dir_in: Vec3, uv: (f32, f32), rnd: Vec3) -> BSDFSample {
		if -cos_theta(dir_in) <= 0.0 {
			return NULL_SAMPLE;
		}

		let d = cosine_hemisphere((rnd.x, rnd.y));

		BSDFSample {
			direction: d,
			pdf: cosine_hemisphere_pdf(d),
			weight: self.albedo.eval(uv),
			is_specular: false,
		}
	}

	fn eval(&self, _dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> Vec3 {
		self.albedo.eval(uv) * (INV_PI * cos_theta(dir_out).max(0.0))
	}

	fn pdf(&self, _dir_in: Vec3, dir_out: Vec3, _uv: (f32, f32)) -> f32 {
		INV_PI * cos_theta(dir_out).max(0.0)
	}

	fn is_purely_specular(&self) -> bool {
		false
	}
}

pub struct Mirror {
	pub albedo: Texture,
}

impl Material for Mirror {
	fn sample(&self, dir_in: Vec3, uv: (f32, f32), _rnd: Vec3) -> BSDFSample {
		if -cos_theta(dir_in) <= 0.0 {
			return NULL_SAMPLE;
		}

		BSDFSample {
			direction: reflect(dir_in),
			pdf: 1.0,
			weight: self.albedo.eval(uv),
			is_specular: true,
		}
	}
}

pub struct Dielectric {
	pub albedo: Texture,
	pub ior: f32,
}

impl Material for Dielectric {
	fn sample(&self, dir_in: Vec3, uv: (f32, f32), rnd: Vec3) -> BSDFSample {
		let eta = if cos_theta(dir_in) >= 0.0 { self.ior } else { 1.0 / self.ior };
		let cos_i = cos_theta(dir_in).abs();
		let (reflectance, cos_t) = fresnel::dielectric_reflectance(eta, cos_i);

		let (direction, pdf) = if rnd.z < reflectance {
			(reflect(dir_in), reflectance)
		} else {
			(refract(dir_in, eta, cos_t), 1.0 - reflectance)
		};

		BSDFSample {
			direction,
			pdf,
			weight: self.albedo.eval(uv),
			is_specular: true,
		}
	}
}

pub struct RoughDielectric {
	/// Material albedo; should be set to `1.0` for physical accuracy
	pub albedo: Texture,
	/// Index of Refraction
	pub ior: f32,
	pub roughness: Texture,
}

impl Material for RoughDielectric {
	fn sample(&self, dir_in: Vec3, uv: (f32, f32), rnd: Vec3) -> BSDFSample {
		let eta = if cos_theta(dir_in) >= 0.0 { self.ior } else { 1.0 / self.ior };
		let cos_i = cos_theta(dir_in).abs();

		let roughness = self.roughness.eval(uv).avg();
		let sample_roughness = (1.2 - 0.2 * cos_i.sqrt()) * roughness;

		let (h, ggx_pdf) = microfacet::sample(sample_roughness, (rnd.x, rnd.y));

		if ggx_pdf < 1e-10 {
			return NULL_SAMPLE;
		}

		let in_dot_h = -Vec3::dot(dir_in, h);
		let eta_h = if in_dot_h < 0.0 { self.ior } else { 1.0 / self.ior };
		let (f, cos_t) = fresnel::dielectric_reflectance(eta_h, in_dot_h.abs());

		let is_reflection = rnd.z < f;
		let direction = if is_reflection {
			2.0 * in_dot_h * h + dir_in
		} else {
			(eta_h * in_dot_h - in_dot_h.signum() * cos_t) * h - eta_h * -dir_in
		}.normalized();

		let reflected = cos_theta(dir_in) * cos_theta(direction) <= 0.0;
		if reflected != is_reflection {
			return NULL_SAMPLE;
		}

		let out_dot_h = Vec3::dot(direction, h);
		let g = microfacet::shadowing(roughness, dir_in, direction, h);
		let d = microfacet::distribution(roughness, cos_theta(h));
		let weight = Vec3::thrice(in_dot_h.abs() * d * g / (ggx_pdf * cos_i));

		let pdf = if is_reflection {
			ggx_pdf / (4.0 * in_dot_h.abs()) * f
		} else {
			let x = eta * in_dot_h + out_dot_h;
			ggx_pdf * out_dot_h.abs() / (x * x) * (1.0 - f)
		};

		BSDFSample {
			direction,
			pdf,
			weight: weight * self.albedo.eval(uv),
			is_specular: false,
		}
	}

	fn eval(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> Vec3 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);

		let is_reflection = cos_i * cos_o >= 0.0;
		let roughness = self.roughness.eval(uv).avg();
		let eta = if cos_i < 0.0 { self.ior } else { 1.0 / self.ior };

		let h = if is_reflection {
			cos_i.signum() * (-dir_in + dir_out).normalized()
		} else {
			-(-dir_in*eta + dir_out).normalized()
		};
		let in_dot_h = -Vec3::dot(dir_in, h);
		let out_dot_h = Vec3::dot(dir_out, h);
		let eta_h = if in_dot_h < 0.0 { self.ior } else { 1.0 / self.ior };

		let (f, _) = fresnel::dielectric_reflectance(eta_h, in_dot_h.abs());
		let g = microfacet::shadowing(roughness, dir_in, dir_out, h);
		let d = microfacet::distribution(roughness, cos_theta(h));

		let r = if is_reflection {
			(f * g * d) / (4.0 * cos_i.abs())
		} else {
			let x = eta * in_dot_h + out_dot_h;
			(in_dot_h * out_dot_h).abs() * (1.0 - f) * g * d / (x * x * cos_i.abs())
		};
		r * self.albedo.eval(uv)
	}

	fn pdf(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> f32 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);

		let is_reflection = cos_i * cos_o >= 0.0;
		let roughness = self.roughness.eval(uv).avg();
		let sample_roughness = (1.2 - 0.2 * cos_i.abs().sqrt()) * roughness;
		let eta = if cos_i < 0.0 { self.ior } else { 1.0 / self.ior };

		let h = if is_reflection {
			cos_i.signum() * (-dir_in + dir_out).normalized()
		} else {
			-(-dir_in*eta + dir_out).normalized()
		};
		let in_dot_h = -Vec3::dot(dir_in, h);
		let out_dot_h = Vec3::dot(dir_out, h);
		let eta_h = if in_dot_h < 0.0 { self.ior } else { 1.0 / self.ior };

		let (f, _) = fresnel::dielectric_reflectance(eta_h, in_dot_h.abs());
		let ggx_pdf = microfacet::pdf(sample_roughness, cos_theta(h));

		if is_reflection {
			ggx_pdf / (4.0 * in_dot_h.abs()) * f
		} else {
			let x = eta * in_dot_h + out_dot_h;
			ggx_pdf * out_dot_h.abs() / (x * x) * (1.0 - f)
		}
	}

	fn is_purely_specular(&self) -> bool {
		false
	}
}

pub struct Plastic {
	albedo: Texture,
	ior: f32,
}

impl Plastic {
	pub fn new(albedo: Texture, ior: f32) -> Plastic {
		Plastic {
			albedo,
			ior,
		}
	}
}

impl Material for Plastic {
	fn eval(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> Vec3 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);

		if cos_i <= 0.0 || cos_o <= 0.0 {
			return Vec3::zero();
		}

		let eta = 1.0 / self.ior;
		let (f, _) = fresnel::dielectric_reflectance(eta, cos_i);

		self.albedo.eval(uv) * (INV_PI * cos_o * (1.0 - f))
	}

	fn sample(&self, dir_in: Vec3, uv: (f32, f32), rnd: Vec3) -> BSDFSample {
		let cos_i = -cos_theta(dir_in);
		if cos_i <= 0.0 {
			return NULL_SAMPLE;
		}

		// Chose between specular and diffuse reflection according to fresnel reflectance.
		let eta = 1.0 / self.ior;
		let (f, _) = fresnel::dielectric_reflectance(eta, cos_i);
		let spec_prob = f;

		if rnd.z < spec_prob {
			// TODO: shouldn't we account for the probably of sampling that
			// direction with diffuse reflection?
			// - Seems to give incorrect results
			// - Reference implementations like Mitsuba do not

			BSDFSample {
				direction: reflect(dir_in),
				pdf: spec_prob,
				weight: Vec3::thrice(1.0),
				is_specular: true,
			}
		} else {
			let direction = cosine_hemisphere((rnd.x, rnd.y));

			BSDFSample {
				direction,
				pdf: cosine_hemisphere_pdf(direction) * (1.0 - spec_prob),
				weight: self.albedo.eval(uv),
				is_specular: false,
			}
		}
	}

	fn pdf(&self, dir_in: Vec3, dir_out: Vec3, _uv: (f32, f32)) -> f32 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);

		if cos_i <= 0.0 || cos_o <= 0.0 {
			return 0.0;
		}

		let eta = 1.0 / self.ior;
		let (f, _) = fresnel::dielectric_reflectance(eta, cos_i);
		let spec_prob = f;

		cosine_hemisphere_pdf(dir_out) * (1.0 - spec_prob)
	}

	fn is_purely_specular(&self) -> bool {
		false
	}
}

pub struct RoughPlastic {
	albedo: Texture,
	ior: f32,
	roughness: Texture,
}

impl RoughPlastic {
	pub fn new(albedo: Texture, ior: f32, roughness: Texture) -> RoughPlastic {
		RoughPlastic {
			albedo,
			ior,
			roughness,
		}
	}
}

impl Material for RoughPlastic {
	fn eval(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> Vec3 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);

		if cos_i <= 0.0 || cos_o <= 0.0 {
			return Vec3::zero();
		}

		let roughness = self.roughness.eval(uv).avg();
		let h = (-dir_in + dir_out).normalized();
		let eta = 1.0 / self.ior;

		let (f, _) = fresnel::dielectric_reflectance(eta, -Vec3::dot(dir_in, h));
		let d = microfacet::distribution(roughness, cos_theta(h));
		let g = microfacet::shadowing(roughness, dir_in, dir_out, h);

		let spec_brdf = Vec3::thrice((f * d * g) / (4.0 * cos_i));
		let diff_brdf = self.albedo.eval(uv) * (INV_PI * cos_o * (1.0 - f));

		spec_brdf + diff_brdf
	}

	fn sample(&self, dir_in: Vec3, uv: (f32, f32), rnd: Vec3) -> BSDFSample {
		let cos_i = -cos_theta(dir_in);
		if cos_i <= 0.0 {
			return NULL_SAMPLE;
		}

		// Uniformly chose between diffuse and glossy reflection.
		// For some reason, it seems to give better results than sampling
		// according to fresnel reflectance.
		let spec_prob = 0.5;

		let direction = if rnd.z < spec_prob {
			let roughness = self.roughness.eval(uv).avg();
			let (h, _) = microfacet::sample(roughness, (rnd.x, rnd.y));
			let direction = (2.0 * -Vec3::dot(dir_in, h) * h + dir_in).normalized();

			if cos_theta(direction) <= 0.0 {
				return NULL_SAMPLE;
			}

			direction
		} else {
			cosine_hemisphere((rnd.x, rnd.y))
		};

		let pdf = self.pdf(dir_in, direction, uv);

		BSDFSample {
			direction,
			pdf,
			weight: self.eval(dir_in, direction, uv) / pdf,
			is_specular: false,
		}
	}

	fn pdf(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> f32 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);

		if cos_i <= 0.0 || cos_o <= 0.0 {
			return 0.0;
		}

		let spec_prob = 0.5;

		let roughness = self.roughness.eval(uv).avg();
		let h = (-dir_in + dir_out).normalized();

		let spec_pdf = microfacet::pdf(roughness, cos_theta(h)) / (4.0 * Vec3::dot(dir_out, h));
		let diff_pdf = cosine_hemisphere_pdf(dir_out);

		spec_pdf * spec_prob + diff_pdf * (1.0 - spec_prob)
	}

	fn is_purely_specular(&self) -> bool {
		false
	}
}

pub struct Conductor {
	pub albedo: Texture,
	pub ior: ComplexIOR,
}


impl Conductor {
	pub fn from_symbol(symbol: &str, albedo: Texture) -> Option<Arc<Material>> {
		let &(_, ior) = CONDUCTORS_IOR.iter().find(|t| t.0 == symbol)?;
		Some(Arc::new(Conductor { albedo, ior }))
	}
}

impl Material for Conductor {
	fn sample(&self, dir_in: Vec3, uv: (f32, f32), _rnd: Vec3) -> BSDFSample {
		let cos_i = -cos_theta(dir_in);

		if cos_i <= 0.0 {
			return NULL_SAMPLE;
		}

		BSDFSample {
			direction: reflect(dir_in),
			pdf: 1.0,
			weight: self.albedo.eval(uv) * fresnel::conductor_reflectance_rgb(self.ior, cos_i),
			is_specular: true,
		}
	}
}

pub struct RoughConductor {
	pub albedo: Texture,
	pub ior: ComplexIOR,
	pub roughness: Texture,
}

impl RoughConductor {
	pub fn from_symbol(symbol: &str, albedo: Texture, roughness: Texture) -> Option<Arc<Material>> {
		let &(_, ior) = CONDUCTORS_IOR.iter().find(|t| t.0 == symbol)?;
		Some(Arc::new(RoughConductor { albedo, ior, roughness }))
	}
}

impl Material for RoughConductor {
	fn sample(&self, dir_in: Vec3, uv: (f32, f32), rnd: Vec3) -> BSDFSample {
		let roughness = self.roughness.eval(uv).avg();
		let (h, ggx_pdf) = microfacet::sample(roughness, (rnd.x, rnd.y));
		let in_dot_h = -Vec3::dot(dir_in, h);
		let direction = dir_in + (2.0 * in_dot_h) * h;

		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(direction);
		if cos_i <= 0.0 || cos_o <= 0.0 || in_dot_h <= 0.0 {
			return NULL_SAMPLE;
		}

		let g = microfacet::shadowing(roughness, dir_in, direction, h);
		let d = microfacet::distribution(roughness, cos_theta(h));
		let f = fresnel::conductor_reflectance_rgb(self.ior, in_dot_h);

		let pdf = ggx_pdf / (4.0 * in_dot_h);
		let weight = in_dot_h * d * g / (ggx_pdf * cos_i);
		let weight = self.albedo.eval(uv) * (f * weight);

		BSDFSample { direction, pdf, weight, is_specular: false }
	}

	fn eval(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> Vec3 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);
		if cos_i <= 0.0 && cos_o <= 0.0 {
			return Vec3::zero();
		}

		let roughness = self.roughness.eval(uv).avg();
		let h = (-dir_in + dir_out).normalized();
		let f = fresnel::conductor_reflectance_rgb(self.ior, -Vec3::dot(dir_in, h));
		let g = microfacet::shadowing(roughness, dir_in, dir_out, h);
		let d = microfacet::distribution(roughness, cos_theta(h));
		let albedo = self.albedo.eval(uv);
		albedo * f * (g * d / (4.0 * cos_i))
	}

	fn pdf(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> f32 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);
		if cos_i <= 0.0 && cos_o <= 0.0 {
			return 0.0;
		}
		let roughness = self.roughness.eval(uv).avg();
		let h = (-dir_in + dir_out).normalized();
		microfacet::pdf(roughness, cos_theta(h)) / (4.0 * -Vec3::dot(dir_in, h))
	}

	fn is_purely_specular(&self) -> bool {
		false
	}
}

pub struct SmoothCoat {
	pub ior: f32,
	pub scaled_sigma_a: Vec3,
	pub substrate: Arc<Material>,
}

impl Material for SmoothCoat {
	fn sample(&self, dir_in: Vec3, uv: (f32, f32), rnd: Vec3) -> BSDFSample {
		let eta = 1.0 / self.ior;
		let cos_i = -cos_theta(dir_in);
		if cos_i <= 0.0 {
			return NULL_SAMPLE;
		}
		let (fi, cos_ti) = fresnel::dielectric_reflectance(eta, cos_i);

		let avg_transmittance = (-2.0 * self.scaled_sigma_a.avg()).exp();
		let sub_weight = avg_transmittance * (1.0 - fi);
		let specular_weight = fi;
		let specular_prob = specular_weight / (specular_weight + sub_weight);

		if rnd.z < specular_prob {
			return BSDFSample {
				direction: reflect(dir_in),
				pdf: specular_prob,
				weight: Vec3::thrice(fi + sub_weight),
				is_specular: true,
			};
		}

		let dir_in_sub = Vec3::new(dir_in.x * eta, -cos_ti, dir_in.z * eta);
		let sub_sample = self.substrate.sample(dir_in_sub, uv, rnd);
		if sub_sample.weight == Vec3::zero() {
			return NULL_SAMPLE;
		}

		let cos_sub = cos_theta(sub_sample.direction);
		let (fo, cos_to) = fresnel::dielectric_reflectance(self.ior, cos_sub);
		if fo == 1.0 {
			return NULL_SAMPLE;
		}

		let dir_out_sub = sub_sample.direction;
		let direction = Vec3::new(dir_out_sub.x * self.ior, cos_to, dir_out_sub.z * self.ior);

		let mut weight = sub_sample.weight * ((1.0 - fi) * (1.0 - fo));
		if self.scaled_sigma_a.max_elem() > 0.0 {
			weight *= (self.scaled_sigma_a * (-1.0 / cos_sub - 1.0 / cos_ti)).map(f32::exp);
		}
		weight = weight / (1.0 - specular_prob);
		let pdf = sub_sample.pdf * (1.0 - specular_prob) * eta*eta*cos_to/cos_sub;

		BSDFSample { direction, pdf, weight, is_specular: sub_sample.is_specular }
	}

	fn eval(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> Vec3 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);
		if cos_i <= 0.0 && cos_o <= 0.0 {
			return Vec3::zero();
		}

		let eta = 1.0 / self.ior;
		let (fi, cos_ti) = fresnel::dielectric_reflectance(eta, cos_i);
		let (fo, cos_to) = fresnel::dielectric_reflectance(eta, cos_o);

		let dir_in_sub = Vec3::new(dir_in.x * eta, -cos_ti, dir_in.z * eta);
		let dir_out_sub = Vec3::new(dir_out.x * eta, cos_to, dir_out.z * eta);

		let mut sub_eval = self.substrate.eval(dir_in_sub, dir_out_sub, uv);

		if self.scaled_sigma_a.max_elem() > 0.0 {
			sub_eval *= (self.scaled_sigma_a * (-1.0 / cos_to - 1.0 / cos_ti)).map(f32::exp);
		}

		let l = eta * eta * cos_o / cos_to;
		l * (1.0 - fi) * (1.0 - fo) * sub_eval
	}

	fn pdf(&self, dir_in: Vec3, dir_out: Vec3, uv: (f32, f32)) -> f32 {
		let cos_i = -cos_theta(dir_in);
		let cos_o = cos_theta(dir_out);
		if cos_i <= 0.0 && cos_o <= 0.0 {
			return 0.0;
		}

		let eta = 1.0 / self.ior;
		let (fi, cos_ti) = fresnel::dielectric_reflectance(eta, cos_i);
		let (_,  cos_to) = fresnel::dielectric_reflectance(eta, cos_o);

		let dir_in_sub = Vec3::new(dir_in.x * eta, -cos_ti, dir_in.z * eta);
		let dir_out_sub = Vec3::new(dir_out.x * eta, cos_to, dir_out.z * eta);

		let avg_transmittance = (-2.0 * self.scaled_sigma_a.avg()).exp();
		let sub_weight = avg_transmittance * (1.0 - fi);
		let specular_weight = fi;
		let specular_prob = specular_weight / (specular_weight + sub_weight);
		let l = eta * eta * (cos_o / cos_to).abs();
		self.substrate.pdf(dir_in_sub, dir_out_sub, uv) * (1.0 - specular_prob) * l
	}

	fn is_purely_specular(&self) -> bool {
		false
	}
}

fn refract(dir_in: Vec3, eta: f32, cos_t: f32) -> Vec3 {
	Vec3::new(dir_in.x * eta, cos_t * cos_theta(dir_in).signum(), dir_in.z * eta)
}

fn reflect(dir_in: Vec3) -> Vec3 {
	Vec3::new(dir_in.x, -dir_in.y, dir_in.z)
}

fn cos_theta(v: Vec3) -> f32 {
	v.y
}

mod fresnel {
	use math::*;
	use super::ComplexIOR;

	// return (reflectance, cos_t)
	pub fn dielectric_reflectance(eta: f32, cos_i: f32) -> (f32, f32) {
		// clamp cos_i before using trigonometric identities
		let cos_i = cos_i.min(1.0).max(-1.0);

		let sin_t2 = eta * eta * (1.0 - cos_i * cos_i);
		if sin_t2 > 1.0 {
			// Total Internal Reflection
			return (1.0, 0.0);
		}

		let cos_t = (1.0 - sin_t2).sqrt();
		let r_s = (eta * cos_i - cos_t) / (eta * cos_i + cos_t);
		let r_p = (eta * cos_t - cos_i) / (eta * cos_t + cos_i);
		return ((r_s * r_s + r_p * r_p) * 0.5, cos_t);
	}

	fn conductor_reflectance(eta: f32, k: f32, cos_i: f32) -> f32 {
		let cos_i2 = cos_i * cos_i;
		let sin_i2 = 1.0 - cos_i2;
		let sin_i4 = sin_i2 * sin_i2;

		let x = eta * eta - k * k - sin_i2;
		let a2_b2 = (x * x + 4.0 * eta * eta * k * k).max(0.0).sqrt();
		let a = ((a2_b2 + x) * 0.5).sqrt();

		let r_s = ((a2_b2 + cos_i2) - (2.0 * a * cos_i)) / ((a2_b2 + cos_i2) + (2.0 * a * cos_i));
		let r_p = ((cos_i2 * a2_b2 + sin_i4) - (2.0 * a * cos_i * sin_i2)) / ((cos_i2 * a2_b2 + sin_i4) + (2.0 * a * cos_i * sin_i2));

		return (r_s + r_s * r_p) * 0.5;
	}

	pub fn conductor_reflectance_rgb(ior: ComplexIOR, cos_i: f32) -> Vec3 {
		// clamp cos_i before using trigonometric identities
		let cos_i = cos_i.min(1.0).max(-1.0);

		Vec3 {
			x: conductor_reflectance(ior.eta.x, ior.k.x, cos_i),
			y: conductor_reflectance(ior.eta.y, ior.k.y, cos_i),
			z: conductor_reflectance(ior.eta.z, ior.k.z, cos_i),
		}
	}
}

mod microfacet {
	use math::*;
	use super::cos_theta;

	pub fn distribution(alpha: f32, cos_theta: f32) -> f32 {
		if cos_theta <= 0.0 {
			return 0.0;
		}

		let alpha2 = alpha * alpha;
		let cos_theta2 = cos_theta * cos_theta;
		let tan_theta2 = (1.0 - cos_theta2) / cos_theta2;
		let cos_theta4 = cos_theta2 * cos_theta2;
		let x = alpha2 + tan_theta2;
		alpha2 / (PI * cos_theta4 * x * x)
	}

	fn shadowing_1d(alpha: f32, v: Vec3, h: Vec3) -> f32 {
		let cos_theta = cos_theta(v);
		if Vec3::dot(v, h) * cos_theta <= 0.0 {
			return 0.0;
		}

		let alpha2 = alpha * alpha;
		let cos_theta2 = cos_theta * cos_theta;
		let tan_theta2 = (1.0 - cos_theta2) / cos_theta2;
		2.0 / (1.0 + (1.0 + alpha2 * tan_theta2).sqrt())
	}

	pub fn shadowing(alpha: f32, dir_in: Vec3, dir_out: Vec3, h: Vec3) -> f32 {
		shadowing_1d(alpha, -dir_in, h) * shadowing_1d(alpha, dir_out, h)
	}

	pub fn sample(alpha: f32, (u, v): (f32, f32)) -> (Vec3, f32) {
		let phi = v * 2.0 * PI;
		let tan_theta2 = alpha * alpha * u / (1.0 - u);
		let cos_theta = 1.0 / (1.0 + tan_theta2).sqrt();
		let r = (1.0 - cos_theta * cos_theta).sqrt();
		(Vec3::new(phi.cos() * r, cos_theta, phi.sin() * r), distribution(alpha, cos_theta) * cos_theta)
	}

	pub fn pdf(alpha: f32, cos_theta: f32) -> f32 {
		distribution(alpha, cos_theta) * cos_theta
	}
}

// List of complex conductor IOR taken from Tungsten:
// https://github.com/tunabrain/tungsten/blob/master/src/core/bsdfs/ComplexIorData.hpp
pub const CONDUCTORS_IOR: [(&'static str, ComplexIOR); 40] = [
	("a-C",    ComplexIOR { eta: Vec3 { x: 2.9440999183, y: 2.2271502925, z: 1.9681668794 }, k: Vec3 { x: 0.8874329109, y: 0.7993216383, z: 0.8152862927 } }),
	("Ag",     ComplexIOR { eta: Vec3 { x: 0.1552646489, y: 0.1167232965, z: 0.1383806959 }, k: Vec3 { x: 4.8283433224, y: 3.1222459278, z: 2.1469504455 } }),
	("Al",     ComplexIOR { eta: Vec3 { x: 1.6574599595, y: 0.8803689579, z: 0.5212287346 }, k: Vec3 { x: 9.2238691996, y: 6.2695232477, z: 4.8370012281 } }),
	("AlAs",   ComplexIOR { eta: Vec3 { x: 3.6051023902, y: 3.2329365777, z: 2.2175611545 }, k: Vec3 { x: 0.0006670247, y: -0.000499940, z: 0.0074261204 } }),
	("AlSb",   ComplexIOR { eta: Vec3 { x: -0.048522570, y: 4.1427547893, z: 4.6697691348 }, k: Vec3 { x: -0.036374191, y: 0.0937665154, z: 1.3007390124 } }),
	("Au",     ComplexIOR { eta: Vec3 { x: 0.1431189557, y: 0.3749570432, z: 1.4424785571 }, k: Vec3 { x: 3.9831604247, y: 2.3857207478, z: 1.6032152899 } }),
	("Be",     ComplexIOR { eta: Vec3 { x: 4.1850592788, y: 3.1850604423, z: 2.7840913457 }, k: Vec3 { x: 3.8354398268, y: 3.0101260162, z: 2.8690088743 } }),
	("Cr",     ComplexIOR { eta: Vec3 { x: 4.3696828663, y: 2.9167024892, z: 1.6547005413 }, k: Vec3 { x: 5.2064337956, y: 4.2313645277, z: 3.7549467933 } }),
	("CsI",    ComplexIOR { eta: Vec3 { x: 2.1449030413, y: 1.7023164587, z: 1.6624194173 }, k: Vec3 { x: 0.0000000000, y: 0.0000000000, z: 0.0000000000 } }),
	("Cu",     ComplexIOR { eta: Vec3 { x: 0.2004376970, y: 0.9240334304, z: 1.1022119527 }, k: Vec3 { x: 3.9129485033, y: 2.4528477015, z: 2.1421879552 } }),
	("Cu2O",   ComplexIOR { eta: Vec3 { x: 3.5492833755, y: 2.9520622449, z: 2.7369202137 }, k: Vec3 { x: 0.1132179294, y: 0.1946659670, z: 0.6001681264 } }),
	("CuO",    ComplexIOR { eta: Vec3 { x: 3.2453822204, y: 2.4496293965, z: 2.1974114493 }, k: Vec3 { x: 0.5202739621, y: 0.5707372756, z: 0.7172250613 } }),
	("d-C",    ComplexIOR { eta: Vec3 { x: 2.7112524747, y: 2.3185812849, z: 2.2288565009 }, k: Vec3 { x: 0.0000000000, y: 0.0000000000, z: 0.0000000000 } }),
	("Hg",     ComplexIOR { eta: Vec3 { x: 2.3989314904, y: 1.4400254917, z: 0.9095512090 }, k: Vec3 { x: 6.3276269444, y: 4.3719414152, z: 3.4217899270 } }),
	("HgTe",   ComplexIOR { eta: Vec3 { x: 4.7795267752, y: 3.2309984581, z: 2.6600252401 }, k: Vec3 { x: 1.6319827058, y: 1.5808189339, z: 1.7295753852 } }),
	("Ir",     ComplexIOR { eta: Vec3 { x: 3.0864098394, y: 2.0821938440, z: 1.6178866805 }, k: Vec3 { x: 5.5921510077, y: 4.0671757150, z: 3.2672611269 } }),
	("K",      ComplexIOR { eta: Vec3 { x: 0.0640493070, y: 0.0464100621, z: 0.0381842017 }, k: Vec3 { x: 2.1042155920, y: 1.3489364357, z: 0.9132113889 } }),
	("Li",     ComplexIOR { eta: Vec3 { x: 0.2657871942, y: 0.1956102432, z: 0.2209198538 }, k: Vec3 { x: 3.5401743407, y: 2.3111306542, z: 1.6685930000 } }),
	("MgO",    ComplexIOR { eta: Vec3 { x: 2.0895885542, y: 1.6507224525, z: 1.5948759692 }, k: Vec3 { x: 0.0000000000, y: 0.0000000000, z: 0.0000000000 } }),
	("Mo",     ComplexIOR { eta: Vec3 { x: 4.4837010280, y: 3.5254578255, z: 2.7760769438 }, k: Vec3 { x: 4.1111307988, y: 3.4208716252, z: 3.1506031404 } }),
	("Na",     ComplexIOR { eta: Vec3 { x: 0.0602665320, y: 0.0561412435, z: 0.0619909494 }, k: Vec3 { x: 3.1792906496, y: 2.1124800781, z: 1.5790940266 } }),
	("Nb",     ComplexIOR { eta: Vec3 { x: 3.4201353595, y: 2.7901921379, z: 2.3955856658 }, k: Vec3 { x: 3.4413817900, y: 2.7376437930, z: 2.5799132708 } }),
	("Ni",     ComplexIOR { eta: Vec3 { x: 2.3672753521, y: 1.6633583302, z: 1.4670554172 }, k: Vec3 { x: 4.4988329911, y: 3.0501643957, z: 2.3454274399 } }),
	("Rh",     ComplexIOR { eta: Vec3 { x: 2.5857954933, y: 1.8601866068, z: 1.5544279524 }, k: Vec3 { x: 6.7822927110, y: 4.7029501026, z: 3.9760892461 } }),
	("Se-e",   ComplexIOR { eta: Vec3 { x: 5.7242724833, y: 4.1653992967, z: 4.0816099264 }, k: Vec3 { x: 0.8713747439, y: 1.1052845009, z: 1.5647788766 } }),
	("Se",     ComplexIOR { eta: Vec3 { x: 4.0592611085, y: 2.8426947380, z: 2.8207582835 }, k: Vec3 { x: 0.7543791750, y: 0.6385150558, z: 0.5215872029 } }),
	("SiC",    ComplexIOR { eta: Vec3 { x: 3.1723450205, y: 2.5259677964, z: 2.4793623897 }, k: Vec3 { x: 0.0000007284, y: -0.000000686, z: 0.0000100150 } }),
	("SnTe",   ComplexIOR { eta: Vec3 { x: 4.5251865890, y: 1.9811525984, z: 1.2816819226 }, k: Vec3 { x: 0.0000000000, y: 0.0000000000, z: 0.0000000000 } }),
	("Ta",     ComplexIOR { eta: Vec3 { x: 2.0625846607, y: 2.3930915569, z: 2.6280684948 }, k: Vec3 { x: 2.4080467973, y: 1.7413705864, z: 1.9470377016 } }),
	("Te-e",   ComplexIOR { eta: Vec3 { x: 7.5090397678, y: 4.2964603080, z: 2.3698732430 }, k: Vec3 { x: 5.5842076830, y: 4.9476231084, z: 3.9975145063 } }),
	("Te",     ComplexIOR { eta: Vec3 { x: 7.3908396088, y: 4.4821028985, z: 2.6370708478 }, k: Vec3 { x: 3.2561412892, y: 3.5273908133, z: 3.2921683116 } }),
	("ThF4",   ComplexIOR { eta: Vec3 { x: 1.8307187117, y: 1.4422274283, z: 1.3876488528 }, k: Vec3 { x: 0.0000000000, y: 0.0000000000, z: 0.0000000000 } }),
	("TiC",    ComplexIOR { eta: Vec3 { x: 3.7004673762, y: 2.8374356509, z: 2.5823030278 }, k: Vec3 { x: 3.2656905818, y: 2.3515586388, z: 2.1727857800 } }),
	("TiN",    ComplexIOR { eta: Vec3 { x: 1.6484691607, y: 1.1504482522, z: 1.3797795097 }, k: Vec3 { x: 3.3684596226, y: 1.9434888540, z: 1.1020123347 } }),
	("TiO2-e", ComplexIOR { eta: Vec3 { x: 3.1065574823, y: 2.5131551146, z: 2.5823844157 }, k: Vec3 { x: 0.0000289537, y: -0.000025148, z: 0.0001775555 } }),
	("TiO2",   ComplexIOR { eta: Vec3 { x: 3.4566203131, y: 2.8017076558, z: 2.9051485020 }, k: Vec3 { x: 0.0001026662, y: -0.000089753, z: 0.0006356902 } }),
	("VC",     ComplexIOR { eta: Vec3 { x: 3.6575665991, y: 2.7527298065, z: 2.5326814570 }, k: Vec3 { x: 3.0683516659, y: 2.1986687713, z: 1.9631816252 } }),
	("VN",     ComplexIOR { eta: Vec3 { x: 2.8656011588, y: 2.1191817791, z: 1.9400767149 }, k: Vec3 { x: 3.0323264950, y: 2.0561075580, z: 1.6162930914 } }),
	("V",      ComplexIOR { eta: Vec3 { x: 4.2775126218, y: 3.5131538236, z: 2.7611257461 }, k: Vec3 { x: 3.4911844504, y: 2.8893580874, z: 3.1116965117 } }),
	("W",      ComplexIOR { eta: Vec3 { x: 4.3707029924, y: 3.3002972445, z: 2.9982666528 }, k: Vec3 { x: 3.5006778591, y: 2.6048652781, z: 2.2731930614 } }),
];
