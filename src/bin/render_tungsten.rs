#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tracing;

use std::fs::File;
use std::path::Path;
use std::io::BufReader;
use std::collections::HashMap;
use std::sync::Arc;

use tracing::{math, scene, camera, material, texture, primitive, obj, light};

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Vec3 {
	Thrice(f32),
	Explicit(f32, f32, f32),
}

#[derive(Deserialize, Debug)]
struct Scene {
	bsdfs: Vec<Bsdf>,
	primitives: Vec<Primitive>,
	camera: Camera,
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Bsdf {
	Lambert { name: String, albedo: Texture },
	Mirror { name: String, albedo: Texture },
	Conductor { name: String, albedo: Texture, material: String },
	Plastic { name: String, albedo: Texture, ior: f32 },
	Dielectric { name: String, albedo: Texture, ior: f32 },
	RoughDielectric { name: String, albedo: Texture, ior: f32, roughness: Texture },
	RoughConductor { name: String, albedo: Texture, material: String, roughness: Texture },
	RoughPlastic { name: String, albedo: Texture, ior: f32, roughness: Texture },
	SmoothCoat { name: String, ior: f32, sigma_a: Vec3, thickness: f32, substrate: Box<BsdfRef> },
	Transparency { name: String },
	Thinsheet { name: String, albedo: Texture, ior: f32 },
	Null { name: String },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum BsdfRef {
	Bsdf(Bsdf),
	Ref(String),
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Texture {
	Constant(Vec3),
	Procedural(ProceduralTexture),
	Bitmap(String),
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ProceduralTexture {
	Checker { on_color: Vec3, off_color: Vec3, res_u: f32, res_v: f32 },
}

#[derive(Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Primitive {
	Quad { bsdf: BsdfRef, transform: Transform, emission: Option<Vec3> },
	Mesh { bsdf: BsdfRef, transform: Transform, file: String },
	InfiniteSphere { transform: Transform, emission: String },
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Transform {
	LookAt { position: Vec3, look_at: Vec3, up: Vec3 },
	Normal { position: Option<Vec3>, scale: Option<Vec3>, rotation: Option<Vec3> },
}

#[derive(Deserialize, Debug)]
struct Camera {
	resolution: Resolution,
	transform: Transform,
	fov: f32,
	tonemap: Tonemap,
	aperture_size: Option<f32>,
	focus_distance: Option<f32>,
}

#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Resolution {
	Rect(usize, usize),
	Square(usize),
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
enum Tonemap {
	Gamma,
	Filmic,
}

impl Vec3 {
	fn convert(self) -> math::Vec3 {
		match self {
			Vec3::Thrice(v) => math::Vec3::thrice(v),
			Vec3::Explicit(x, y, z) => math::Vec3 { x, y, z },
		}
	}
}

impl Scene {
	fn convert(self, dir: &Path) -> (scene::Scene, camera::Camera) {
		let mut bsdfs = HashMap::new();
		for b in self.bsdfs.into_iter() {
			let (name, mat) = b.convert(dir, &bsdfs);
			bsdfs.insert(name, mat);
		}
		let mut objects = Vec::new();
		let mut envmap = None;
		for p in self.primitives {
			p.convert(dir, &bsdfs, &mut objects, &mut envmap);
		}
		(scene::Scene::new(envmap, objects), self.camera.convert())
	}
}

impl Bsdf {
	fn convert(self, dir: &Path, bsdfs: &HashMap<String, Arc<material::Material>>) -> (String, Arc<material::Material>) {
		match self {
			Bsdf::Lambert { name, albedo } => {
				(name.clone(), Arc::new(material::Diffuse {
					albedo: albedo.convert(dir),
				}))
			}
			Bsdf::RoughPlastic { name, albedo, ior, roughness } => {
				(name.clone(), Arc::new(material::RoughPlastic::new(
					albedo.convert(dir),
					ior,
					roughness.convert(dir),
				)))
			}
			Bsdf::Mirror { name, albedo } => {
				(name.clone(), Arc::new(material::Mirror {
					albedo: albedo.convert(dir),
				}))
			}
			Bsdf::Conductor { name, albedo, material } => {
				(name.clone(), material::Conductor::from_symbol(&material, albedo.convert(dir)).unwrap())
			}
			Bsdf::RoughConductor { name, albedo, material, roughness } => {
				(name.clone(), material::RoughConductor::from_symbol(&material, albedo.convert(dir), roughness.convert(dir)).unwrap())
			}
			Bsdf::Plastic { name, albedo, ior } => {
				(name.clone(), Arc::new(material::Plastic::new(albedo.convert(dir), ior)))
			}
			Bsdf::Dielectric { name, albedo, ior } => {
				(name.clone(), Arc::new(material::Dielectric {
					albedo: albedo.convert(dir),
					ior,
				}))
			}
			Bsdf::RoughDielectric { name, albedo, ior, roughness } => {
				(name.clone(), Arc::new(material::RoughDielectric {
					albedo: albedo.convert(dir),
					ior,
					roughness: roughness.convert(dir),
				}))
			}
			Bsdf::Thinsheet { name, albedo, ior } => {
				// TODO: real thin sheet
				(name.clone(), Arc::new(material::Dielectric {
					albedo: albedo.convert(dir),
					ior,
				}))
			}
			Bsdf::Null { name } => {
				// TODO: real null bsdf
				(name.clone(), Arc::new(material::Diffuse {
					albedo: texture::Texture::Constant(math::Vec3::zero()),
				}))
			}

			Bsdf::SmoothCoat { name, ior, sigma_a, thickness, substrate } => {
				(name.clone(), Arc::new(material::SmoothCoat {
					ior,
					scaled_sigma_a: sigma_a.convert() * thickness,
					substrate: substrate.convert(dir, bsdfs),
				}))
			}
			Bsdf::Transparency { name } => {
				// TODO: add material
				(name.clone(), Arc::new(material::Diffuse {
					albedo: texture::Texture::Constant(math::Vec3::thrice(0.5)),
				}))
			}
		}
	}
}

impl BsdfRef {
	fn convert(self, dir: &Path, bsdfs: &HashMap<String, Arc<material::Material>>) -> Arc<material::Material> {
		// all diffuse white
		//return Arc::new(material::Material {
		//	texture: texture::Texture::Constant(math::Vec3::thrice(0.5)),
		//	bsdf: material::BSDF::Diffuse,
		//});

		match self {
			BsdfRef::Ref(name) => bsdfs.get(&name).unwrap().clone(),
			BsdfRef::Bsdf(bsdf) => bsdf.convert(dir, bsdfs).1,
		}
	}
}

impl Texture {
	fn convert(self, dir: &Path) -> texture::Texture {
		match self {
			Texture::Constant(v) => texture::Texture::Constant(v.convert()),
			Texture::Procedural(ProceduralTexture::Checker { on_color, off_color, res_u, res_v }) => {
				texture::Texture::Checker {
					on_color: on_color.convert(),
					off_color: off_color.convert(),
					resolution: (res_u, res_v)
				}
			}
			Texture::Bitmap(file) => texture::Texture::Bitmap(texture::Image::load_ldr(dir.join(&file))),
		}
	}
}

impl Primitive {
	fn convert(self, dir: &Path, bsdfs: &HashMap<String, Arc<material::Material>>, objects: &mut Vec<scene::Object>, envmap: &mut Option<light::EnvMap>) {
		match self {
			Primitive::Quad { bsdf, transform, emission: None } => {
				objects.push(scene::Object::Scatterer {
					surface: Box::new(primitive::Parallelogram::unit_transform(&transform.convert())),
					material: bsdf.convert(dir, bsdfs),
				})
			}
			Primitive::Quad { bsdf: _, transform, emission: Some(v) } => {
				objects.push(scene::Object::Emitter(light::AreaLight {
					surface: Box::new(primitive::Parallelogram::unit_transform(&transform.convert())),
					emission: v.convert(),
				}))
			}
			Primitive::Mesh { bsdf, transform, file } => {
				let mat = bsdf.convert(dir, bsdfs);
				for (_, mesh) in obj::load(dir.join(&file), &transform.convert()) {
					objects.push(scene::Object::Scatterer {
						surface: Box::new(mesh),
						material: mat.clone(),
					})
				}
			}
			Primitive::InfiniteSphere { transform, emission } => {
				let hdr = texture::Image::load_hdr(dir.join(&emission));
				*envmap = Some(light::EnvMap::from_image(hdr, &transform.convert()))
			}
		}
	}
}

impl Transform {
	fn convert(self) -> math::Mat4 {
		match self {
			Transform::LookAt { position, look_at, up } => {
				math::Mat4::look_at(position.convert(), look_at.convert(), up.convert())
			}
			Transform::Normal { position, scale, rotation } => {
				let mut transform = math::Mat4::identity();
				if let Some(v) = position {
					transform = transform * math::Mat4::translate(v.convert())
				}
				if let Some(v) = rotation {
					transform = transform * math::Mat4::rot_yxz(v.convert())
				}
				if let Some(v) = scale {
					transform = transform * math::Mat4::scale(v.convert())
				}
				transform
			}
		}
	}
}

impl Camera {
	fn convert(self) -> camera::Camera {
		camera::Camera::new(
			&self.transform.convert(),
			self.resolution.convert(),
			self.fov,
			self.tonemap.convert(),
			self.aperture_size,
			self.focus_distance,
		)
	}
}

impl Resolution {
	fn convert(self) -> (usize, usize) {
		match self {
			Resolution::Rect(w, h) => (w, h),
			Resolution::Square(w) => (w, w),
		}
	}
}

impl Tonemap {
	fn convert(self) -> camera::Tonemap {
		match self {
			Tonemap::Gamma => camera::gamma,
			Tonemap::Filmic => camera::filmic,
		}
	}
}

fn main() {
	let args = std::env::args().collect::<Vec<String>>();

	if args.len() < 2 {
		eprintln!("usage: {} tungsten_scene.json", args[0]);
		std::process::exit(1);
	}

	let path = Path::new(&args[1]);
	let file = BufReader::new(File::open(&path).unwrap());
	let tungsten_scene: Scene = serde_json::from_reader(file).unwrap();
	//println!("{:?}", &tungsten_scene);

	let (scene, camera) = tungsten_scene.convert(path.parent().unwrap());

	tracing::render_preview(scene, camera);
	//tracing::render(scene, camera, 16);
}
