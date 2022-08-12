pub mod triangles;
pub mod image_viewer;
pub mod clear;
pub mod monotext;

use ash::vk;
use std::sync::{Arc, RwLock};

pub type LayerRef = Arc<RwLock<dyn Layer>>;
pub trait Layer {
	fn set_output(&mut self, image: Vec<vk::Image>);
	fn render(&self, command_buffer: vk::CommandBuffer, idx: usize);
}
