use std::fs::File;
use std::io::{BufReader, Write, BufWriter};
use std::path::Path;
use image;
use math::{Vec3, bilerp};
use camera::Tonemap;

pub enum Texture {
	Constant(Vec3),
	Grid(Vec3, Vec3, usize, f32),
	Checker { on_color: Vec3, off_color: Vec3, resolution: (f32, f32) },
	Bitmap(Image),
}

impl Texture {
	pub fn eval(&self, (u, v): (f32, f32)) -> Vec3 {
		match *self {
			Texture::Constant(c) => c,
			Texture::Grid(c1, c2, s, w) => {
				let m = 1.0 / s as f32;
				let in_band = (u + 1.0 + w / 2.0) % m < w || (v + 1.0 + w / 2.0) % m < w;
				if in_band { c2 } else { c1 }
			},
			Texture::Checker { on_color, off_color, resolution } => {
				let ui = (resolution.0 * u) as i32;
				let vi = (resolution.1 * v) as i32;
				let on = (ui ^ vi) & 1 != 0;
				if on { on_color } else { off_color }
			},
			Texture::Bitmap(ref img) => {
				img.eval((u, v))
			},
		}
	}
}

pub struct Image {
	pub width: usize,
	pub height: usize,
	pixels: Vec<Vec3>,
}

impl Image {
	pub fn load_ldr<P: AsRef<Path>>(filepath: P) -> Image {
		let img = image::open(&filepath).expect("failed to load texture").to_rgb();
		let (width, height) = img.dimensions();

		Image {
			width: width as usize,
			height: height as usize,
			pixels: img.pixels().map(|p| gamma_decode(p.data)).collect(),
		}
	}

	pub fn load_hdr<P: AsRef<Path>>(filepath: P) -> Image {
		let reader = BufReader::new(File::open(filepath).expect("failed to open texture"));
		let decoder = image::hdr::HDRDecoder::with_strictness(reader, false).expect("failed to decode texture");
		let meta = decoder.metadata();
		let pixels = decoder.read_image_hdr().expect("failed to decode texture");

		Image {
			width: meta.width as usize,
			height: meta.height as usize,
			pixels: pixels.iter().map(|p| Vec3::new(p[0], p[1], p[2])).collect(),
		}
	}

	pub fn get(&self, x: usize, y: usize) -> Vec3 {
		self.pixels[self.width * y + x]
	}

	/// Evaluate the texture using parametric coordinates and bilinear interpolation
	pub fn eval(&self, (u, v): (f32, f32)) -> Vec3 {
		let w = self.width as isize;
		let h = self.height as isize;

		// Convert parametric coordinates to texture coordinates, accounting for
		// - the half-pixel offset due to the continuous to discrete conversion
		// - the vertical flip of texture coordinates
		let tu = w as f32 * u - 0.5;
		let tv = h as f32 * (1.0 - v) - 0.5;

		let x0 = tu.floor() as isize;
		let y0 = tv.floor() as isize;
		let x1 = x0 + 1;
		let y1 = y0 + 1;

		// Compute the offset from the pixel due to the fractional part of coordinates
		let dx = tu - x0 as f32;
		let dy = tv - y0 as f32;

		// Handle off-boundaries coordinates by wrapping them around, thus repeating the texture
		let x0 = modulo(x0, w);
		let x1 = modulo(x1, w);
		let y0 = modulo(y0, h);
		let y1 = modulo(y1, h);

		// Finally, returns the bilinear interpolation of the 4 surrounding pixel values
		let v00 = self.get(x0, y0);
		let v01 = self.get(x1, y0);
		let v10 = self.get(x0, y1);
		let v11 = self.get(x1, y1);
		bilerp(v00, v01, v10, v11, (dx, dy))
	}
}

/// Non-negative remainder of a divided by b.
fn modulo(a: isize, b: isize) -> usize {
	let r = a % b;
	(if r < 0 { r + b } else { r }) as usize
}

fn gamma_decode([r, g, b]: [u8; 3]) -> Vec3 {
	// TODO: static lookup table for converting sRGB values into linear RGB values
	// static GAMMA_LOOKUP: Vec<f32> = (0..256).map(|v| (v as f32 / 255.0).powf(2.2)).collect();
	let f = |v| (v as f32 / 255.0).powf(2.2);
	Vec3::new(f(r), f(g), f(b))
}

pub fn luminance(color: Vec3) -> f32 {
	0.2126 * color.x + 0.7152 * color.y + 0.0722 * color.z
}

pub fn write_ppm_srgb<I>(path: &str, width: usize, height: usize, tonemap: Tonemap, pixels: I)
	where I: IntoIterator<Item=Vec3>
{
	let mut f = BufWriter::new(File::create(path).unwrap());
	write!(f, "P6\n{} {}\n{}\n", width, height, 255).unwrap();
	for p in pixels {
		let v = tonemap(p).map(|x| x * 255.0 + 0.5);
		f.write(&[v.x as u8, v.y as u8, v.z as u8]).unwrap();
	}
}

pub fn write_ppm_raw(path: &str, width: usize, height: usize, pixels: &[u8])
{
	let mut f = BufWriter::new(File::create(path).unwrap());
	write!(f, "P6\n{} {}\n{}\n", width, height, 255).unwrap();
	f.write_all(&pixels).unwrap();
}

/*
pub fn write_hdr(path: &str, width: usize, height: usize, buf: &[(Vec3, f32)]) {
	let mut data: Vec<image::Rgb<f32>> = Vec::with_capacity(width * height);
	for &(v, w) in buf {
		let r = v / w;
		data.push(image::Rgb { data: [r.x, r.y, r.z]});
	}

	let f = BufWriter::new(File::create(path).unwrap());
	let enc = image::hdr::HDREncoder::new(f);
	enc.encode(&data[..], width, height).unwrap();
}
*/
