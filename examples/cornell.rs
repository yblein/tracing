extern crate tracing;

use std::sync::Arc;

use tracing::math::*;
use tracing::primitive::*;
use tracing::material::*;
use tracing::scene::*;
use tracing::camera::*;
use tracing::*;
use tracing::texture::*;
use tracing::light::*;

const HEIGHT: usize = 512;
const WIDTH: usize = HEIGHT;

fn main() {
	let white = Vec3::new(0.740063,0.742313,0.733934);
	let green = Vec3::new(0.162928,0.408903,0.0833759);
	let red = Vec3::new(0.366046,0.0371827,0.0416385);
	//let light_color = Vec3::new(0.780131,0.780409,0.775833);
	let scene = Scene::new(
		None,
		vec![
			/*
			Object::Emitter(AreaLight { // light
				surface: Box::new(Sphere {
					position: Vec3 { x: 0.0, y: 0.7, z: 0.0 },
					radius: 0.15,
				}),
				emission: Vec3::thrice(1.0) * 20.0,
			}),
			*/
			Object::Emitter(AreaLight { // light
				surface: Box::new(Parallelogram::from_square(
					Vec3 { x: 0.0, y: 1.0 - EPSILON, z: 0.0 },
					Vec3 { x: 0.0, y: -1.0, z: 0.0 },
					0.5
				)),
				emission: Vec3::new(1.0, 0.772549, 0.560784) * 40.0,
			}),
			/*
			Object::Emitter(AreaLight { // light
				surface: Box::new(Disk::new(
					Vec3 { x: 0.0, y: 1.0 - EPSILON, z: 0.0 },
					Vec3 { x: 0.0, y: -1.0, z: 0.0 },
					0.25
				)),
				emission: Vec3::thrice(1.0) * 20.0,
			}),
			*/
			/*
			Object::Emitter(AreaLight { // light
				surface: Box::new(Sphere::new(0.10, Vec3 { x: -3.0, y: 3.0, z: -3.0 })),
				emission: Vec3::new(1.0, 0.772549, 0.560784) * 4000.0,
			}),
			*/
			Object::Scatterer {
				surface: Box::new(Sphere::new(
					0.35,
					Vec3 { x: -0.5, y: -0.65, z: -0.3 },
				)),
				material: Arc::new(Diffuse {
					albedo: Texture::Constant(Vec3::thrice(0.99)),
				}),
			},
			Object::Scatterer {
				surface: Box::new(Sphere::new(
					0.35,
					Vec3 { x: 0.5, y: -0.65, z: 0.3 },
				)),
				material: Arc::new(Diffuse {
					albedo: Texture::Constant(Vec3::thrice(0.99)),
				}),
			},
			/*
			Object::Scatterer {
				surface: Box::new(Mesh::load_obj("data/bunny.obj", Vec3::new(0.4, 0.4, -0.4), 0.0, Vec3::new(-0.0, -0.88, 0.0))),
				//surface: Box::new(Mesh::load_obj("/tmp/dragon.obj", Vec3::thrice(1.0), 0.0, Vec3::new(0.0, 0.0, 0.0))),
				//surface: Box::new(Mesh::load_obj("data/teapot.obj", Vec3::thrice(0.8), 0.0, Vec3::new(0.0, -1.0, 0.0))),
				material: Arc::new(Material {
					bsdf: BSDF::Glass,
					//albedo: Texture::Constant(white),
					albedo: Texture::Constant(Vec3::thrice(0.99)),
				}),
			},
			*/
			Object::Scatterer { // Left wall
				surface: Box::new(Parallelogram::from_square(
					Vec3 { x: -1.0, y: 0.0, z: 0.0 },
					Vec3 { x: 1.0, y: 0.0, z: 0.0 },
					2.0
				)),
				material: Arc::new(Diffuse {
					albedo: Texture::Constant(red),
				}),
			},
			Object::Scatterer { // Right wall
				surface: Box::new(Parallelogram::from_square(
					Vec3 { x: 1.0, y: 0.0, z: 0.0 },
					Vec3 { x: -1.0, y: 0.0, z: 0.0 },
					2.0
				)),
				material: Arc::new(Diffuse {
					albedo: Texture::Constant(green),
				}),
			},
			Object::Scatterer { // Back wall
				surface: Box::new(Parallelogram::from_square(
					Vec3 { x: 0.0, y: 0.0, z: -1.0 },
					Vec3 { x: 0.0, y: 0.0, z: 1.0 },
					2.0
				)),
				material: Arc::new(Diffuse {
					albedo: Texture::Constant(white),
				}),
			},
			Object::Scatterer { // Ceiling
				surface: Box::new(Parallelogram::from_square(
					Vec3 { x: 0.0, y: 1.0, z: 0.0 },
					Vec3 { x: 0.0, y: -1.0, z: 0.0 },
					2.0
				)),
				material: Arc::new(Diffuse {
					albedo: Texture::Constant(white),
				}),
			},
			Object::Scatterer { // Floor
				surface: Box::new(Parallelogram::from_square(
					Vec3 { x: 0.0, y: -1.0, z: 0.0 },
					Vec3 { x: 0.0, y: 1.0, z: 0.0 },
					2.0
				)),
				material: Arc::new(Diffuse {
					//albedo: Texture::Constant(white),
					albedo: Texture::Grid(white, Vec3::thrice(0.25), 4, 0.02),
				}),
			},
		]
	);

	let camera = Camera::new(
		&Mat4::look_at(Vec3::new(0.0, 0.0, 4.5), Vec3::zero(), Vec3::new(0.0, 1.0, 0.0)),
		(WIDTH, HEIGHT),
		30.0,
		gamma,
		None,
		None,
	);

	render_preview(scene, camera);
	//render(scene, camera, 64);
}
