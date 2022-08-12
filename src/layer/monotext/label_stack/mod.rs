pub mod line;
use line::{Char, Line};

use std::collections::HashMap;
use vulkano::pipeline::graphics::viewport::Viewport;

#[derive(Default)]
pub struct LabelStack {
	lines: Vec<Line>,
	names: HashMap<String, usize>,
	pub scaler: f32,
	font_size: [u32; 2],
}

use crate::vertex::VertexText;

impl LabelStack {
	pub fn new(font_size: [u32; 2]) -> Self {
		Self {
			lines: Vec::new(),
			names: HashMap::new(),
			scaler: 1.0,
			font_size,
		}
	}

	pub fn set_scaler(&mut self, k: f32) {
		self.scaler = k;
	}

	pub fn add_text(&mut self, key: &str, line: Line) {
		if let Some(idx) = self.names.get(key) {
			self.lines[*idx] = line;
			return;
		}
		assert!(self
			.names
			.insert(key.to_string(), self.lines.len())
			.is_none());
		self.lines.push(line);
	}

	pub fn to_vertices(&self, viewport: &Viewport) -> Vec<VertexText> {
		let size_x = 1024 / self.font_size[0];
		// let size_y = 1024 / self.font_size[1];
		let mut result = vec![];
		for (idy, line) in self.lines.iter().enumerate() {
			let mut color = [1.0; 4];
			let mut idx: i32 = -1;
			for &ch in line.data.iter() {
				let ch = match ch {
					Char::SetColor(c) => {
						color = c;
						continue;
					}
					Char::Byte(b) => b,
				};
				idx += 1;
				let idx = idx as u32;
				let idy = idy as u32;
				let ux = ch as u32 % size_x;
				let uy = ch as u32 / size_x;
				let upos_list =
					vec![[0, 0], [0, 1], [1, 1], [0, 0], [1, 0], [1, 1]];
				for upos in upos_list.iter() {
					let tex_coord = [
						((ux + upos[0]) * self.font_size[0]) as f32 / 1024f32,
						((uy + upos[1]) * self.font_size[1]) as f32 / 1024f32,
					];
					let pos = [
						-1.0 + ((idx + upos[0]) * self.font_size[0]) as f32
							/ viewport.dimensions[0] * self.scaler,
						-1.0 + ((idy + upos[1]) * self.font_size[1]) as f32
							/ viewport.dimensions[1] * self.scaler,
					];
					result.push(VertexText {
						color,
						pos,
						tex_coord,
					});
				}
			}
		}
		result
	}
}
