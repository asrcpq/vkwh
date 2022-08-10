use ash::vk;

use crate::layer::LayerRef;
use crate::base::{BaseRef, record_submit_commandbuffer};

pub struct LayerCompositor {
	base: BaseRef,
	layers: Vec<LayerRef>,
	update_flag: Vec<bool>,
}

impl LayerCompositor {
	pub fn new(base: BaseRef, layers: Vec<LayerRef>) -> Self {
		Self {
			base,
			update_flag: vec![false; layers.len()],
			layers,
		}
	}

	pub fn update_all(&mut self) {
		for flag in self.update_flag.iter_mut() {
			*flag = true;
		}
	}

	pub fn mark_update(&mut self, idx: usize) {
		self.update_flag[idx] = true;
	}

	pub fn render(&mut self) {
		unsafe {
			let base = self.base.read().unwrap();
			let (present_index, _) = base
				.swapchain_loader
				.acquire_next_image(
					base.swapchain,
					std::u64::MAX,
					base.present_complete_semaphore,
					vk::Fence::null(),
				)
				.unwrap();
			record_submit_commandbuffer(
				&base.device,
				base.draw_command_buffer,
				base.draw_commands_reuse_fence,
				base.present_queue,
				&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT],
				&[base.present_complete_semaphore],
				&[base.rendering_complete_semaphore],
				|_device, draw_command_buffer| {
					for (idx, layer) in self.layers.iter().enumerate() {
						if self.update_flag[idx] {
							let layer = layer.read().unwrap();
							layer.render(draw_command_buffer, present_index);
							self.update_flag[idx] = false;
						}
					}
				},
			);
			let wait_semaphors = [base.rendering_complete_semaphore];
			let swapchains = [base.swapchain];
			let image_indices = [present_index];
			let present_info = vk::PresentInfoKHR::default()
				.wait_semaphores(&wait_semaphors) // &base.rendering_complete_semaphore)
				.swapchains(&swapchains)
				.image_indices(&image_indices);
	
			base.swapchain_loader
				.queue_present(base.present_queue, &present_info)
				.unwrap();
		}
	}
}
