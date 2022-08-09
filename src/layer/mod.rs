pub mod triangles;

use ash::vk;
use std::sync::{Arc, RwLock};

pub type LayerRef = Arc<RwLock<dyn Layer>>;
pub trait Layer {
	fn render(&self, command_buffer: vk::CommandBuffer, image_idx: u32);
}
