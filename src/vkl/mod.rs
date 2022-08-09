use std::sync::{Arc, Mutex};
use vulkano::pipeline::graphics::viewport::Viewport;

use crate::vkw;

pub mod dummy;
pub mod blit;

pub type VklRef = Arc<Mutex<dyn Vkl>>;
pub trait Vkl {
	fn render(&self, builder: &mut vkw::CommandBuilder, image_num: usize, viewport: Viewport);
	fn update_images(&mut self, images: &vkw::Images);
}
