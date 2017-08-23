use math::*;
use geometry::*;
use scene::*;
use texture::*;
use material::*;
use warp::*;
use light::*;

use rand::Rng;

pub fn estimate_radiance<R: Rng>(scene: &Scene, ray: Ray, rng: &mut R) -> Vec3 {
	let nb_lights = scene.nb_lights();
	let light_pick_prob = 1.0 / nb_lights as f32;

	let mut path_weight = Vec3::thrice(1.0);
	let mut radiance = Vec3::zero();
	let mut ray = ray;
	let mut specular_bounce = true;
	let mut last_pdf_dir = 1.0;

	for nb_bounces in 0.. {
		let (intersection, material) = match scene.intersect(ray) {
			Some(Hit::Scatterer(its, mat)) => (its, mat),
			Some(Hit::Emitter(light, dist)) => {
				let contrib = light.eval_direct(ray.direction);
				let mis_weight = if !specular_bounce {
					let direct_pdf = light.pdf_direct(ray.direction, dist);
					mis2(last_pdf_dir, direct_pdf * light_pick_prob)
				} else {
					1.0
				};
				radiance += path_weight * mis_weight * contrib;

				// lights do not reflect; stop here
				break;
			}
			None => break,
		};

		// compute some geometry at intersection
		let normal = intersection.normal;
		let hit = ray.point_at(intersection.distance);
		let shading_frame = Frame::from_up(normal);
		let local_in = shading_frame.to_local(ray.direction);

		let cont_prob = path_weight.max_elem().min(1.0);

		if !material.is_purely_specular() && nb_lights > 0 {
			// direct light sampling (also known as "next event estimation")
			let light_idx = (nb_lights as f32 * rng.gen::<f32>()) as usize;
			if let Some(ref light) = scene.get_light(light_idx) {
				let (emission, light_sample) = light.sample_direct(hit, rng.gen());
				if !scene.occluded(hit, normal, light_sample.dir, light_sample.dist) {
					let local_out = shading_frame.to_local(light_sample.dir);
					let bsdf_eval = material.eval(local_in, local_out, intersection.uv);
					let bsdf_pdf = material.pdf(local_in, local_out, intersection.uv);
					let mis_weight = mis2(light_sample.pdf * light_pick_prob, bsdf_pdf * cont_prob);
					radiance += path_weight * emission * bsdf_eval * (mis_weight / (light_sample.pdf * light_pick_prob));
				}
			}
		}

		let bsdf_sample = material.sample(local_in, intersection.uv, Vec3::new(rng.gen(), rng.gen(), rng.gen()));

		if bsdf_sample.weight == Vec3::zero() {
			break;
		}

		last_pdf_dir = bsdf_sample.pdf;

		// possibly terminate path
		//if nb_bounces >= 3 {
			if nb_bounces > 64 {
				// we are probably stuck inside a non transmitive object
				//eprintln!("Infinite bouncing detected (cont_prob {:?}, path_weight {:?}, bsdf_sample {:?})", cont_prob, path_weight, (bsdf_sample.weight, bsdf_sample.pdf));
				break;
			}
			// russian roulette
			last_pdf_dir *= cont_prob;
			if cont_prob < 1.0 {
				if rng.gen::<f32>() >= cont_prob {
					break;
				}
				path_weight = path_weight / cont_prob;
			}
		//}

		path_weight *= bsdf_sample.weight;
		specular_bounce = bsdf_sample.is_specular;

		// nudge ray origin to avoid self-intersection
		ray.direction = shading_frame.to_world(bsdf_sample.direction).normalized();
		let eps = if Vec3::dot(normal, ray.direction) >= 0.0 { EPSILON } else { -EPSILON };
		ray.origin = hit + normal * eps * 2.0;
	}

	radiance
}

fn mis2(sample_pdf: f32, other_pdf: f32) -> f32 {
	let power = |x| x*x;
	power(sample_pdf) / (power(sample_pdf) + power(other_pdf))
}
