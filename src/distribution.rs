use rayon::prelude::*;

/// Make an importance samplable 1D distribution
///
/// Will normalize both the `pdf` and the computed `cdf`.
/// Returns the sum of the given PDF.
fn make1d(pdf: &mut [f32], cdf: &mut [f32]) -> f32 {
	cdf[0] = pdf[0];
	for i in 1..cdf.len() {
		cdf[i] = cdf[i - 1] + pdf[i];
	}

	let total = *cdf.last().unwrap();
	debug_assert!(total.is_finite(), "distribution has non-finite values");

	if total <= 0.0 {
		println!("Warning: the distribution is null and should not be sampled");
		return 0.0;
	}

	for v in pdf.iter_mut() {
		*v /= total;
	}
	for v in cdf.iter_mut() {
		*v /= total;
	}

	return total;
}

/// Importance sample a 1D distribution
///
/// Returns the sampled index and its PDF.
fn sample1d(pdf: &[f32], cdf: &[f32], u: f32) -> (usize, f32) {
	let i = match cdf.binary_search_by(|v| v.partial_cmp(&u).unwrap()) {
		Ok(v) => v,
		Err(v) => v,
	};
	(i, pdf[i])
}

pub struct Distribution1D {
	pdf: Vec<f32>,
	cdf: Vec<f32>,
}

impl Distribution1D {
	pub fn new(weights: Vec<f32>) -> Distribution1D {
		let mut pdf = weights;
		let mut cdf = vec![0.0; pdf.len()];
		let _ = make1d(&mut pdf[..], &mut cdf[..]);
		Distribution1D { pdf, cdf }
	}

	pub fn sample(&self, u: f32) -> (usize, f32) {
		sample1d(&self.pdf[..], &self.cdf[..], u)
	}
}

pub struct Distribution2D {
	width: usize,
	height: usize,
	conditional_pdf: Vec<f32>,
	conditional_cdf: Vec<f32>,
	marginal_pdf: Vec<f32>,
	marginal_cdf: Vec<f32>,
}

impl Distribution2D {
	pub fn new(weights: Vec<f32>, width: usize, height: usize) -> Distribution2D {
		debug_assert_eq!(weights.len(), width * height);

		let mut conditional_cdf = vec![0.0; weights.len()];
		let mut conditional_pdf = weights;
		let mut marginal_cdf = vec![0.0; height];
		let mut marginal_pdf = {
			let chunks_pdf = conditional_pdf.par_chunks_mut(width);
			let chunks_cdf = conditional_cdf.par_chunks_mut(width);
			chunks_pdf.zip(chunks_cdf).map(|(pdf, cdf)| make1d(pdf, cdf)).collect::<Vec<f32>>()
		};
		let _ = make1d(&mut marginal_pdf[..], &mut marginal_cdf[..]);

		Distribution2D { width, height, conditional_pdf, conditional_cdf, marginal_pdf, marginal_cdf }
	}

	pub fn sample(&self, u: f32, v: f32) -> ((usize, usize), f32) {
		// first sample the marginal distribution to find a row
		let (s1, p1) = sample1d(&self.marginal_pdf[..], &self.marginal_cdf[..], v);
		// then sample the distribution of the chosen row `s1`
		let i0 = self.width * s1;
		let i1 = i0 + self.width;
		let (s0, p0) = sample1d(&self.conditional_pdf[i0..i1], &self.conditional_cdf[i0..i1], u);
		((s0, s1), p0 * p1)
	}

	pub fn pdf(&self, x: usize, y: usize) -> f32 {
		let x = x.min(self.width - 1);
		let y = y.min(self.height - 1);
		self.conditional_pdf[y * self.width + x] * self.marginal_pdf[y]
	}
}
