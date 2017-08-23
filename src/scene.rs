use geometry::*;
use material::*;
use light::*;
use math::*;
use bvh::BVH;
use std::sync::Arc;

pub enum Object {
	Emitter(AreaLight),
	Scatterer {
		surface: Box<Surface + Send + Sync>,
		material: Arc<Material>
	},
}

impl Object {
	fn is_emitter(&self) -> bool {
		match self {
			Object::Emitter(_) => true,
			_ => false,
		}
	}
}

impl Surface for Object {
	fn intersect(&self, ray: Ray) -> Option<Intersection> {
		match *self {
			Object::Emitter(ref area_light) => area_light.surface.intersect(ray),
			Object::Scatterer { ref surface, .. } => surface.intersect(ray),
		}
	}

	fn aabb(&self) -> AABB {
		match *self {
			Object::Emitter(ref area_light) => area_light.surface.aabb(),
			Object::Scatterer { ref surface, .. } => surface.aabb(),
		}
	}
}

pub(crate) enum Hit<'a> {
	Emitter(&'a Light, f32),
	Scatterer(Intersection, &'a Material),
}

pub struct Scene {
	objects: Vec<Object>,
	background: Option<EnvMap>,
	light_idxs: Vec<usize>,
	bvh: BVH,
}

impl Scene {
	pub fn new(background: Option<EnvMap>, objects: Vec<Object>) -> Scene {
		let mut objects = objects;

		let proj_centroid = |o: &Object, axis| o.aabb().center()[axis];
		let obj_bbox = |o: &Object| o.aabb();
		let bvh = BVH::build(&proj_centroid, &obj_bbox, &mut objects[..]);

		let light_idxs: Vec<usize> = objects.iter()
			.enumerate()
			.filter(|(_, o)| o.is_emitter())
			.map(|(i, _)| i)
			.collect();

		Scene { objects, background, bvh, light_idxs }
	}

	pub(crate) fn intersect(&self, ray: Ray) -> Option<Hit> {
		//return self.intersect_objects(ray, 0, self.objects.len());
		let intersect_item = |ray, i| {
			let o: &Object = &self.objects[i];
			match o.intersect(ray) {
				None => (-1.0, Default::default()),
				Some(its) => (its.distance, (its.normal, its.uv)),
			}
		};
		let (t, i, (n, uv)) = self.bvh.intersect(&intersect_item, ray);

		if t > 0.0 {
			Some(match self.objects[i] {
				Object::Scatterer { ref material, .. } => {
					Hit::Scatterer(Intersection { distance: t, normal: n, uv }, material.as_ref())
				}
				Object::Emitter(ref area_light) => {
					Hit::Emitter(area_light, t)
				}
			})
		} else {
			self.background.as_ref().map(|envmap| Hit::Emitter(envmap, INFINITY))
		}
	}

	pub(crate) fn nb_lights(&self) -> usize {
		self.light_idxs.len() + self.background.is_some() as usize
	}

	pub(crate) fn get_light(&self, i: usize) -> Option<&Light> {
		match self.light_idxs.get(i).and_then(|&i_obj| self.objects.get(i_obj)) {
			Some(Object::Emitter(light)) => Some(light),
			_ => match self.background {
				Some(ref envmap) => Some(envmap),
				None => None,
			},
		}
	}

	/*
	fn intersect_objects(&self, ray: Ray, begin: usize, end: usize) -> Option<(Intersection, &Object)> {
		let mut closest = None;

		for object in &self.objects[begin..end] {
			if let Some(intersection) = object.surface.intersect(ray) {
				match closest {
					None => closest = Some((intersection, object)),
					Some((Intersection { distance: t_min, .. }, _)) => {
						if 0.0 < intersection.distance && intersection.distance < t_min {
							closest = Some((intersection, object));
						}
					},
				}
			}
		}

		return closest;
	}
	*/

	pub(crate) fn occluded(&self, point: Vec3, normal: Vec3, dir: Vec3, max_dist: f32) -> bool {
		if Vec3::dot(dir, normal) <= 0.0 {
			return true;
		}

		let intersect_item = |ray, i| {
			let o: &Object = &self.objects[i];
			match o.intersect(ray) {
				None => (-1.0, ()),
				Some(its) => (its.distance, ()),
			}
		};

		let shadow_ray = Ray { origin: point + normal * EPSILON, direction: dir };
		let (t, _, ()) = self.bvh.intersect(&intersect_item, shadow_ray);

		t > 0.0 && t < max_dist - 2.0 * EPSILON
	}
}
