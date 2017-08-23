use time::PreciseTime;

use geometry::*;
use math::*;
use bvh::BVH;

/// Represent vertex indices in triangles; 2^32 vertices should be enough
pub type Index = u32;

#[derive(Serialize, Deserialize)]
pub struct Triangle {
	pub idxs: [Index; 3],
}

// N.B. There is a 1-1 correspondence between vertices and normals.
// Those are addressed through the indices stored in triangles.
#[derive(Serialize, Deserialize)]
pub struct Mesh {
	vertices: Vec<Vec3>,
	normals: Vec<Vec3>,
	uvs: Vec<(f32, f32)>,
	triangles: Vec<Triangle>,
	triangles_e1: Vec<Vec3>,
	triangles_e2: Vec<Vec3>,
	bvh: BVH,
}

impl Mesh {
	pub fn new(vertices: Vec<Vec3>, normals: Vec<Vec3>, uvs: Vec<(f32, f32)>, triangles: Vec<Triangle>) -> Mesh {
		if normals.is_empty() {
			panic!("Missing normals");
		}

		let mut triangles = triangles;

		let bvh = {
			let third = 1.0 / 3.0;
			let proj_centroid = |t: &Triangle, axis| -> f32 {
				let v0 = vertices[t.idxs[0] as usize];
				let v1 = vertices[t.idxs[1] as usize];
				let v2 = vertices[t.idxs[2] as usize];
				(v0[axis] + v1[axis] + v2[axis]) * third
			};

			let tri_bbox = |t: &Triangle| -> AABB {
				let b0 = AABB::from_point(vertices[t.idxs[0] as usize]);
				let b1 = AABB::from_point(vertices[t.idxs[1] as usize]);
				let b2 = AABB::from_point(vertices[t.idxs[2] as usize]);
				b0.union(&b1).union(&b2)
			};

			// build the BVH
			println!("Building BVH for {} triangles...", triangles.len());
			let start = PreciseTime::now();
			let bvh = BVH::build(&proj_centroid, &tri_bbox, &mut triangles[..]);
			let end = PreciseTime::now();
			println!("BVH built in {} seconds", start.to(end).num_milliseconds() as f32 / 1000.0);
			bvh
		};

		let (triangles_e1, triangles_e2) = Mesh::compute_edges(&vertices[..], &triangles[..]);

		Mesh {
			vertices: vertices,
			normals: normals,
			uvs: uvs,
			triangles: triangles,
			triangles_e1: triangles_e1,
			triangles_e2: triangles_e2,
			bvh: bvh,
		}
	}

	fn compute_edges(vertices: &[Vec3], triangles: &[Triangle]) -> (Vec<Vec3>, Vec<Vec3>) {
		// cache triangle edges
		let mut triangles_e1 = Vec::with_capacity(triangles.len());
		let mut triangles_e2 = Vec::with_capacity(triangles.len());
		for t in triangles {
			triangles_e1.push(vertices[t.idxs[1] as usize] - vertices[t.idxs[0] as usize]);
			triangles_e2.push(vertices[t.idxs[2] as usize] - vertices[t.idxs[0] as usize]);
		}
		(triangles_e1, triangles_e2)
	}

	fn intersect_triangle(&self, ray: Ray, i: usize) -> (f32, (f32, f32)) {
		let v0 = self.vertices[self.triangles[i].idxs[0] as usize];
		let edge1 = self.triangles_e1[i];
		let edge2 = self.triangles_e2[i];

		let p = Vec3::cross(ray.direction, edge2);
		let idet = 1.0 / Vec3::dot(edge1, p);

		let t = ray.origin - v0;
		let u = Vec3::dot(t, p) * idet;
		if u < 0.0 || u > 1.0 {
			return (-1.0, (0.0, 0.0));
		}

		let q = Vec3::cross(t, edge1);
		let v = Vec3::dot(ray.direction, q) * idet;
		if v < 0.0 || (u + v) > 1.0 {
			return (-1.0, (0.0, 0.0));
		}

		return (Vec3::dot(edge2, q) * idet, (u, v));
	}

}

impl Surface for Mesh {
	fn intersect(&self, ray: Ray) -> Option<Intersection> {
		let intersect_item = |ray, i| self.intersect_triangle(ray, i);
		let (t, i, (u, v)) = self.bvh.intersect(&intersect_item, ray);

		if 0.0 < t {
			let idxs = self.triangles[i].idxs;
			let n0 = self.normals[idxs[0] as usize];
			let n1 = self.normals[idxs[1] as usize];
			let n2 = self.normals[idxs[2] as usize];
			let n = (n0 * (1.0 - u - v) + n1 * u + n2 * v).normalized();
			//let n = Vec3::cross(self.triangles_e1[i_min], self.triangles_e2[i_min]).normalized();

			let uv0 = self.uvs[idxs[0] as usize];
			let uv1 = self.uvs[idxs[1] as usize];
			let uv2 = self.uvs[idxs[2] as usize];

			let tu = uv0.0 * (1.0 - u - v) + uv1.0 * u + uv2.0 * v;
			let tv = uv0.1 * (1.0 - u - v) + uv1.1 * u + uv2.1 * v;

			Some(Intersection {
				distance: t,
				normal: n,
				uv: (tu, tv),
			})
		} else {
			None
		}
	}

	fn aabb(&self) -> AABB {
		self.bvh.bbox()
	}
}

