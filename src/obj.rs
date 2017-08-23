use std::fs::{File, create_dir};
use std::env::temp_dir;
use std::io::{BufRead, BufReader, BufWriter};
use std::path::Path;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::collections::HashMap;
use bincode;

use mesh::*;
use math::*;
use mat4::*;

struct ObjTriangle {
	vidxs: [Index; 3],
	nidxs: [Index; 3],
	uidxs: [Index; 3],
}

pub fn load<P: AsRef<Path>>(path: P, transform: &Mat4) -> Vec<(String, Mesh)> {
	// compute cache path from the filepath
	let hash = {
		let mut hasher = DefaultHasher::new();
		path.as_ref().hash(&mut hasher);
		for f in &transform.0 {
			f.to_bits().hash(&mut hasher);
		}
		hasher.finish()
	};
	let cache_path = temp_dir().join("obj_cache");
	let obj_cache = cache_path.join(hash.to_string());

	// load from cache if possible
	if let Ok(r) = File::open(&obj_cache) {
		println!("Mesh {} found in cache", path.as_ref().to_str().unwrap());
		let mut br = BufReader::new(r);
		if let Ok(meshes) = bincode::deserialize_from(&mut br) {
			return meshes;
		}
		println!("Failed to load OBJ mesh from cache");
	}

	println!("Loading mesh {}", path.as_ref().to_str().unwrap());

	let f = BufReader::new(File::open(path).unwrap());
	let mut vertices: Vec<Vec3> = Vec::new();
	let mut normals: Vec<Vec3> = Vec::new();
	let mut uvs: Vec<(f32, f32)> = Vec::new();
	let mut triangles: Vec<ObjTriangle> = Vec::new();
	let mut meshes = Vec::new();
	let mut curr_name = String::new();

	fn normalize_obj_idx(idx: isize, len: usize) -> Index {
		if idx < 0 {
			(len as isize + idx) as Index
		} else {
			idx as Index - 1
		}
	}

	for line in f.lines() {
		let s = line.unwrap();
		let mut iter = s.split_whitespace();
		match iter.next() {
			Some("v") => {
				let vs: Vec<f32> = iter.map(|s| s.parse::<f32>().unwrap()).collect();
				let v = Vec3::new(vs[0], vs[1], vs[2]);
				vertices.push(transform.transform_point(v));
			},
			Some("vt") => {
				let v: Vec<f32> = iter.map(|s| s.parse::<f32>().unwrap()).collect();
				uvs.push((v[0], v[1]));
			},
			Some("vn") => {
				let v: Vec<f32> = iter.map(|s| s.parse::<f32>().unwrap()).collect();
				let n = Vec3::new(v[0], v[1], v[2]).normalized();
				debug_assert!(!n.has_nan(), "invalid normal");
				// TODO: transform normal
				normals.push(n);
			},
			Some("f") => {
				let g: Vec<(Index, Index, Index)> = iter.map(|group| {
					let mut iter = group.split('/');
					let vi = iter.next().unwrap().parse::<isize>().unwrap();
					let ui = iter.next().unwrap().parse::<isize>().unwrap();
					let ni = iter.next().unwrap().parse::<isize>().unwrap();
					(normalize_obj_idx(vi, vertices.len()), normalize_obj_idx(ui, uvs.len()), normalize_obj_idx(ni, normals.len()))
				}).collect();
				for i in 2..g.len() {
					triangles.push(ObjTriangle {
						vidxs: [g[0].0, g[i-1].0, g[i].0],
						uidxs: [g[0].1, g[i-1].1, g[i].1],
						nidxs: [g[0].2, g[i-1].2, g[i].2],
					});
				}
			},
			Some("o") => {
				if !triangles.is_empty() {
					meshes.push((curr_name.clone(), create_mesh(&vertices, &normals, &uvs, &triangles)));
					triangles.clear();
				}
				curr_name = iter.next().unwrap_or_default().to_owned();
			}
			_ => {}
		}
	}

	if !triangles.is_empty() {
		meshes.push((curr_name.clone(), create_mesh(&vertices, &normals, &uvs, &triangles)));
	}
	println!("Loaded obj scene with {} meshes", meshes.len());

	//println!("Mesh bounding box: {:?}", mesh.bvh.bbox());

	// Cache the mesh
	{
		println!("Caching OBJ mesh");
		let _ = create_dir(cache_path);
		let mut bw = BufWriter::new(File::create(&obj_cache).unwrap());
		// TODO: serializing edges is not necessary as they can be recomputed quickly
		bincode::serialize_into(&mut bw, &meshes).unwrap();
	}

	return meshes;
}

/// create a mesh with given vertices, normals and triangles so that
/// - there is a 1-1 correspondence between vertices and normals
/// - vertices and normals are stored only if referred to in a triangle
fn create_mesh(vertices: &[Vec3], normals: &[Vec3], uvs: &[(f32, f32)], triangles: &[ObjTriangle]) -> Mesh {
	let mut vs = Vec::new();
	let mut ns = Vec::new();
	let mut us = Vec::new();
	let mut ts = Vec::new();
	let mut vertex_map = HashMap::with_capacity(vertices.len());

	for obj_tri in triangles {
		let mut new_tri = Triangle { idxs: [0; 3] };

		for i in 0..3 {
			let vidx = obj_tri.vidxs[i];
			let nidx = obj_tri.nidxs[i];
			let uidx = obj_tri.uidxs[i];

			// check if we already created a new vertex for this couple
			new_tri.idxs[i] = match vertex_map.get(&(vidx, nidx, uidx)) {
				Some(&idx) => idx,
				None => {
					// if not, create a new one and store it
					let idx = vs.len() as Index;
					vs.push(vertices[vidx as usize]);
					ns.push(normals[nidx as usize]);
					us.push(uvs[uidx as usize]);
					vertex_map.insert((vidx, nidx, uidx), idx);
					idx
				}
			};
		}

		ts.push(new_tri);
	}

	println!("Loaded mesh with {} vertices, {} normals and {} triangles", vs.len(), ns.len(), ts.len());
	Mesh::new(vs, ns, us, ts)
}

// Compute smooth normals
/*
if mesh.normals.len() == 0 {
	println!("Computing vertex normals...");
	mesh.normals.resize(nv, Vec3::zero());
	for (i, &(i0, i1, i2)) in mesh.triangles.iter().enumerate() {
		let normal = Vec3::cross(mesh.triangles_e1[i], mesh.triangles_e2[i]); //.normalized();
		mesh.normals[i0] = mesh.normals[i0] + normal;
		mesh.normals[i1] = mesh.normals[i1] + normal;
		mesh.normals[i2] = mesh.normals[i2] + normal;
	}
	for i in 0..nv {
		mesh.normals[i] = mesh.normals[i].normalized();
	}
}
*/

