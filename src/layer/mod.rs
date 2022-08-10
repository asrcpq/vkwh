pub mod triangles;
pub mod clear;

use ash::vk;
use std::sync::{Arc, RwLock};

pub type LayerRef = Arc<RwLock<dyn Layer>>;
pub trait Layer {
	fn set_output(&mut self, image: vk::Image);
	fn render(&self, command_buffer: vk::CommandBuffer);
}
