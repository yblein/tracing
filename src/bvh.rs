use rayon;

use geometry::*;
use math::*;

#[derive(Serialize, Deserialize)]
pub struct BVH {
	bbox: AABB,
	node: Node,
}

#[derive(Serialize, Deserialize)]
enum Node {
	Leaf { begin: usize, end: usize },
	Split { split_axis: Axis, children: [Box<BVH>; 2] },
}

impl BVH {
	pub fn bbox(&self) -> AABB {
		self.bbox
	}

	pub fn build<I, F, G>(proj_centroid: &F, item_bbox: &G, items: &mut [I]) -> BVH
		where I: Send, F: (Fn(&I, Axis) -> f32) + Sync, G: (Fn(&I) -> AABB) + Sync
	{
		let n = items.len();
		build_rec(proj_centroid, item_bbox, items, 0, n, &mut vec![0.0; n])
	}

	/// Return (t, i, data) for the closest item i or t == -1 if miss
	pub fn intersect<D, F>(&self, intersect_item: &F, ray: Ray) -> (f32, usize, D)
		where D: Default, F: Fn(Ray, usize) -> (f32, D)
	{
		intersect_rec(intersect_item, ray, self, INFINITY, 1.0 / ray.direction)
	}
}

fn build_rec<I, F, G>(proj_centroid: &F, item_bbox: &G, items: &mut [I], begin: usize, end: usize, buffer: &mut [f32]) -> BVH
	where I: Send, F: (Fn(&I, Axis) -> f32) + Sync, G: (Fn(&I) -> AABB) + Sync
{
	const INTERSECTION_COST: f32 = 1.0;
	const TRAVERSAL_COST: f32 = 1.5;

	let n = end - begin;
	let mut best_axis = None;
	let mut best_cost = INTERSECTION_COST * n as f32;
	let mut best_index = 0;
	let mut node_bbox = AABB::empty();

	// Try splitting along every axis
	for &axis in &[Axis::X, Axis::Y, Axis::Z] {
		sort_projected_centroid(&proj_centroid, items, axis);

		// Compute AABB surface areas incrementally from the left
		let mut bbox = AABB::empty();
		for (i, t) in items.iter().enumerate() {
			bbox = bbox.union(&item_bbox(t));
			buffer[i] = bbox.surface_area();
		}

		if axis == Axis::X {
			node_bbox = bbox;
		}

		/* Choose the split plane by computing AABB surface areas incrementally from
		 * the right, and comparing them against the one from the left with the heuristic */
		let mut bbox = AABB::empty();
		let tri_factor = INTERSECTION_COST / node_bbox.surface_area();
		for (i, t) in items.iter().enumerate().skip(1).rev() {
			bbox = bbox.union(&item_bbox(t));

			let left_area = buffer[i - 1];
			let right_area = bbox.surface_area();
			let prims_left = i as f32;
			let prims_right = (n - i) as f32;

			let sah_cost = 2.0 * TRAVERSAL_COST
				+ tri_factor * (prims_left * left_area + prims_right * right_area);
			if sah_cost < best_cost {
				best_cost = sah_cost;
				best_axis = Some(axis);
				best_index = i;
			}
		}
	}

	let node = match best_axis {
		None => Node::Leaf { begin, end }, // Couldn't find a split that reduces the cost, make a leaf
		Some(axis) => {
			if axis != Axis::Z { // we just sorted on Z, only sort for X and Y
				// TODO: sorting is not necessary, we just need to partition
				sort_projected_centroid(&proj_centroid, items, axis);
			}
			let mid = begin + best_index;
			let (buf1, buf2) = buffer.split_at_mut(best_index);
			let (tri1, tri2) = items.split_at_mut(best_index);
			let build1 = || Box::new(build_rec(proj_centroid, item_bbox, tri1, begin, mid, buf1));
			let build2 = || Box::new(build_rec(proj_centroid, item_bbox, tri2, mid,   end, buf2));
			// TODO: sequential fallback
			let (c1, c2) = rayon::join(build1, build2);
			Node::Split { split_axis: axis, children: [c1, c2] }
		},
	};

	BVH { bbox: node_bbox, node: node }
}

// return (t, i, data) for the closest item i or t == -1 if miss
fn intersect_rec<D, F>(intersect_item: &F, ray: Ray, bvh: &BVH, dist_max: f32, inv_dir: Vec3) -> (f32, usize, D)
	where D: Default, F: Fn(Ray, usize) -> (f32, D)
{
	let (t_near, t_far) = bvh.bbox.intersect_fast(ray, inv_dir);
	if t_far < 0.0 || (t_near >= 0.0 && t_near >= dist_max) {
		return (-1.0, 0, Default::default());
	}

	match bvh.node {
		Node::Leaf { begin, end } => intersect_items(intersect_item, ray, begin, end),
		Node::Split { split_axis, ref children } => {
			// order the children according to ray direction
			let (c1, c2) = if ray.direction[split_axis] < 0.0 {
				(&children[1], &children[0])
			} else {
				(&children[0], &children[1])
			};

			let its1 = intersect_rec(intersect_item, ray, c1, dist_max, inv_dir);
			if its1.0 < 0.0 {
				// no intersection in first child, check the other one
				intersect_rec(intersect_item, ray, c2, dist_max, inv_dir)
			} else {
				// intersection in first child, check if there is a closer intersection in the other one
				let its2 = intersect_rec(intersect_item, ray, c2, its1.0, inv_dir);
				if its2.0 < 0.0 || its1.0 < its2.0 {
					its1
				} else {
					its2
				}
			}
		},
	}
}

// returns (t, i, data) for the closest item i or t == -1 if miss
fn intersect_items<D, F>(intersect_item: F, ray: Ray, begin: usize, end: usize) -> (f32, usize, D)
	where D: Default, F: Fn(Ray, usize) -> (f32, D)
{
	let mut t_min = INFINITY;
	let mut i_min = 0;
	let mut d_min = Default::default();

	for i in begin..end {
		let (t, d) = intersect_item(ray, i);
		if 0.0 < t && t < t_min {
			t_min = t;
			i_min = i;
			d_min = d;
		}
	}

	if t_min == INFINITY {
		(-1.0, i_min, d_min)
	} else {
		(t_min, i_min, d_min)
	}
}

fn sort_projected_centroid<I, F>(proj_centroid: F, items: &mut [I], axis: Axis)
	where F: Fn(&I, Axis) -> f32
{
	items.sort_by(|a, b|
		proj_centroid(a, axis).partial_cmp(&proj_centroid(b, axis)).expect("centroid is NaN")
	);
}

/*
// old BVH build_rec based on median on longuest axis instead of SAH
fn build_rec(&mut self, begin: usize, end: usize) -> BVH {
	let mut aabb = AABB::empty();
	for &((i0, i1, i2), _) in &self.triangles[begin..end] {
		aabb.extend_point(self.vertices[i0]);
		aabb.extend_point(self.vertices[i1]);
		aabb.extend_point(self.vertices[i2]);
	}

	let n = end - begin;
	if n <= 8 {
		return BVH::Leaf(aabb, begin, end);
	}

	let split_axis = aabb.longuest_axis();
	self.sort_projected_centroid(split_axis, begin, end);

	let mid = begin + (n + 1) / 2;
	let c1 = Box::new(self.build_rec(begin, mid));
	let c2 = Box::new(self.build_rec(mid, end));
	return BVH::Node(aabb, split_axis, c1, c2);
}
*/

