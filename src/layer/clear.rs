use ash::vk;
use std::default::Default;
use std::sync::{Arc, RwLock};

use crate::base::BaseRef;
use crate::layer::Layer;

pub struct Clear {
	base: BaseRef,
	images: Vec<vk::Image>,
}

impl Clear {
	pub fn new_ref(base: BaseRef) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(Self::new(base)))
	}

	pub fn new(base: BaseRef) -> Self {
		Self {
			base,
			images: Vec::new(),
		}
	}
}

impl Layer for Clear {
	fn set_output(&mut self, images: Vec<vk::Image>) {
		self.images = images;
	}

	fn render(&self, command_buffer: vk::CommandBuffer, idx: usize) { unsafe {
		let base = self.base.read().unwrap();
		let device = &base.device;
		let subresource_range = vk::ImageSubresourceRange {
			aspect_mask: vk::ImageAspectFlags::COLOR,
			level_count: 1,
			layer_count: 1,
			..Default::default()
		};
		let barrier = vk::ImageMemoryBarrier {
			dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
			old_layout: vk::ImageLayout::UNDEFINED,
			new_layout: vk::ImageLayout::GENERAL,
			image: self.images[idx],
			subresource_range,
			..Default::default()
		};
		device.cmd_pipeline_barrier(
			command_buffer,
			vk::PipelineStageFlags::BOTTOM_OF_PIPE,
			vk::PipelineStageFlags::TRANSFER,
			vk::DependencyFlags::empty(),
			&[],
			&[],
			&[barrier],
		);
		device.cmd_clear_color_image(
			command_buffer,
			self.images[idx],
			vk::ImageLayout::GENERAL,
			&vk::ClearColorValue {
				float32: [0.0, 0.0, 0.0, 0.0],
			},
			&[subresource_range],
		);
	}}
}
