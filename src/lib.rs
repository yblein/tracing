extern crate rand;
extern crate time;
extern crate bincode;
extern crate image;
extern crate rayon;
#[cfg(feature = "gui")]
extern crate sdl2;
#[macro_use]
extern crate serde_derive;

pub mod camera;
pub mod geometry;
pub mod light;
pub mod material;
pub mod math;
pub mod mesh;
pub mod primitive;
pub mod scene;
pub mod texture;
pub mod obj;

mod distribution;
mod integrator;
mod warp;
mod bvh;

use rand::{Rng, SeedableRng};
use time::PreciseTime;
use rayon::prelude::*;

use math::*;
use scene::*;
use camera::*;
use texture::*;
use integrator::estimate_radiance;

static OUTPUT_FILE: &'static str = "/tmp/image.ppm";

pub fn render(scene: Scene, camera: Camera, spp: u32) {
	let (width, height) = camera.resolution();
	let mut sum_rad = vec![Vec3::zero(); width * height];

	println!("Start rendering with {} samples per pixel...", spp);
	let start = PreciseTime::now();

	sum_rad.par_chunks_mut(width).enumerate().for_each(|(y, row)| {
		let mut rng: rand::XorShiftRng = rand::random();
		for (x, p) in row.iter_mut().enumerate() {
			for _ in 0..spp {
				let ray = camera.make_ray((x, y), rng.gen(), rng.gen());
				let v = estimate_radiance(&scene, ray, &mut rng);
				if !v.has_nan() {
					*p += v;
				}
			}
		}
	});

	let end = PreciseTime::now();
	let tot_s = start.to(end).num_milliseconds() as f32 / 1000.0;
	println!("Rendered {} spp in {:.3}s ({:.3}s per sample)", spp, tot_s, tot_s / spp as f32);

	write_ppm_srgb(OUTPUT_FILE, width, height, camera.tonemap, sum_rad.iter().map(|&sr| sr / spp as f32));
}

pub fn render_preview(scene: Scene, camera: Camera) {
	#[cfg(feature = "gui")]
	render_preview_gui(scene, camera);
	#[cfg(not(feature = "gui"))]
	render_preview_file(scene, camera);
}

#[cfg(feature = "gui")]
pub fn render_preview_gui(scene: Scene, camera: Camera) {
	use sdl2::pixels::PixelFormatEnum;
	use sdl2::event::Event;
	use sdl2::keyboard::Keycode;

	let mut camera = camera;
	let (width, height) = camera.resolution();

	let mut sum_rad = vec![Vec3::zero(); width * height];
	let mut spp = 0;

	let mut rngs: Vec<rand::XorShiftRng> = Vec::new();
	for _ in 0..height {
		rngs.push(rand::random());
	}

	let mut tonemapped: Vec<u8> = vec![0; width * height * 3];

	// init a SDL window and a texture
	let sdl_context = sdl2::init().unwrap();
	let video_subsystem = sdl_context.video().unwrap();
	let window = video_subsystem.window("tracing", width as u32, height as u32).build().unwrap();
	let mut event_pump = sdl_context.event_pump().unwrap();
	let mut canvas = window.into_canvas().build().unwrap();
	let texture_creator = canvas.texture_creator();
	let mut texture = texture_creator.create_texture_streaming(PixelFormatEnum::RGB24, width as u32, height as u32).unwrap();

	println!("Start rendering...");
	let mut start = PreciseTime::now();

	'rendering: loop {
		// render a new frame
		sum_rad.par_chunks_mut(width).enumerate().zip(rngs.par_iter_mut()).for_each(|((y, row), rng)| {
			for (x, p) in row.iter_mut().enumerate() {
				let ray = camera.make_ray((x, y), rng.gen(), rng.gen());
				let v = estimate_radiance(&scene, ray, rng);
				if !v.has_nan() {
					*p += v;
				}
			}
		});
		spp += 1;
		//println!("{} spp", spp);

		// tonemap the current data and display it
		sum_rad.par_iter().zip(tonemapped.par_chunks_mut(3)).for_each(|(&sr, tm)| {
			let v = (camera.tonemap)(sr / spp as f32).map(|x| x * 255.0 + 0.5);
			tm[0] = v.x as u8;
			tm[1] = v.y as u8;
			tm[2] = v.z as u8;
		});
		texture.update(None, &tonemapped, width * 3).unwrap();
		canvas.copy(&texture, None, None).unwrap();
		canvas.present();
		canvas.window_mut().set_title(&format!("tracing - {} spp", spp)).unwrap();

		// process sdl events
		for event in event_pump.poll_iter() {
			match event {
				Event::Quit {..}
				//| Event::KeyDown { keycode: Some(Keycode::Escape), .. }
				| Event::KeyDown { keycode: Some(Keycode::Q), .. } => {
					break 'rendering;
				}
				//Event::Window { win_event: sdl2::event::WindowEvent::Exposed, .. } => {
				//	canvas.copy(&texture, None, None).unwrap();
				//	canvas.present();
				//}
				Event::MouseButtonDown { x, y, .. } => {
					let ray = camera.make_ray((x as usize, y as usize), (0.0, 0.0), (0.0, 0.0));
					match scene.intersect(ray) {
						Some(Hit::Scatterer(its, _)) => {
							println!("restarting with focal distance = {}", its.distance);
							camera.set_focus_dist(Some(its.distance));
							for sr in &mut sum_rad {
								*sr = Vec3::zero();
							}
							spp = 0;
							start = PreciseTime::now();
						}
						_ => println!("no intersection"),
					}
				}
				_ => {}
			};
		}
	}

	let end = PreciseTime::now();
	let tot_s = start.to(end).num_milliseconds() as f32 / 1000.0;
	println!("Rendered {} spp in {:.3}s ({:.3}s per sample)", spp, tot_s, tot_s / spp as f32);

	write_ppm_srgb(OUTPUT_FILE, width, height, camera.tonemap, sum_rad.iter().map(|&sr| sr / spp as f32));
}

pub fn render_preview_file(scene: Scene, camera: Camera) {
	let camera = camera;
	let (width, height) = camera.resolution();

	let mut sum_rad = vec![Vec3::zero(); width * height];
	let mut spp = 0;

	let mut rngs: Vec<rand::XorShiftRng> = Vec::new();
	for _ in 0..height {
		rngs.push(rand::random());
	}

	let mut tonemapped: Vec<u8> = vec![0; width * height * 3];

	println!("Start rendering...");
	let start = PreciseTime::now();

	const SPP_STEP: usize = 16;

	'rendering: loop {
		// render a new frame
		sum_rad.par_chunks_mut(width).enumerate().zip(rngs.par_iter_mut()).for_each(|((y, row), rng)| {
			let mut local_rng = rng.clone();
			for (x, p) in row.iter_mut().enumerate() {
				for _ in 0..SPP_STEP {
					let ray = camera.make_ray((x, y), local_rng.gen(), local_rng.gen());
					let v = estimate_radiance(&scene, ray, &mut local_rng);
					if !v.has_nan() {
						*p += v;
					}
				}
			}
			*rng = local_rng.clone();
		});
		spp += SPP_STEP;

		// tonemap the current data and dump it
		sum_rad.par_iter().zip(tonemapped.par_chunks_mut(3)).for_each(|(&sr, tm)| {
			let v = (camera.tonemap)(sr / spp as f32).map(|x| x * 255.0 + 0.5);
			tm[0] = v.x as u8;
			tm[1] = v.y as u8;
			tm[2] = v.z as u8;
		});
		write_ppm_raw(OUTPUT_FILE, width, height, &tonemapped);

		let end = PreciseTime::now();
		let tot_s = start.to(end).num_milliseconds() as f32 / 1000.0;
		println!("Rendered {} spp in {:.3}s ({:.3}s per sample)", spp, tot_s, tot_s / spp as f32);
	}
}

pub fn render_seq(scene: Scene, camera: Camera, spp: u32) {
	let (width, height) = camera.resolution();
	let mut sum_rad = vec![Vec3::zero(); width * height];
	//let mut rng: rand::XorShiftRng = rand::random();
	let mut rng = rand::XorShiftRng::from_seed([0u32, 1u32, 2u32, 3u32]);

	println!("Start rendering with {} samples per pixel...", spp);
	let start = PreciseTime::now();

	sum_rad.chunks_mut(width).enumerate().for_each(|(y, row)| {
		for (x, p) in row.iter_mut().enumerate() {
			for _ in 0..spp {
				let ray = camera.make_ray((x, y), rng.gen(), rng.gen());
				let v = estimate_radiance(&scene, ray, &mut rng);
				if !v.has_nan() {
					*p += v;
				}
			}
		}
	});

	let end = PreciseTime::now();
	let tot_s = start.to(end).num_milliseconds() as f32 / 1000.0;
	println!("Rendered {} spp in {:.3}s ({:.3}s per sample)", spp, tot_s, tot_s / spp as f32);

	write_ppm_srgb(OUTPUT_FILE, width, height, camera.tonemap, sum_rad.iter().map(|&sr| sr / spp as f32));
}
