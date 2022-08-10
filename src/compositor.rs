use ash::vk;

use crate::layer::LayerRef;
use crate::base::{BaseRef, record_submit_commandbuffer};

pub struct LayerObject {
	layer: LayerRef,
	damage: bool,
	image: vk::Image,
}

pub struct LayerCompositor {
	base: BaseRef,
	los: Vec<LayerObject>,
}

impl LayerCompositor {
	pub fn new(base: BaseRef, layers: Vec<LayerRef>) -> Self { unsafe {
		let base_clone = base.clone();
		let base = base.read().unwrap();
		let create_info = vk::ImageCreateInfo::default()
			.image_type(vk::ImageType::TYPE_2D)
			.format(base.surface_format.format)
			.extent(base.surface_resolution.into())
			.mip_levels(1)
			.array_layers(1)
			.samples(vk::SampleCountFlags::TYPE_1)
			.tiling(vk::ImageTiling::OPTIMAL)
			.usage(vk::ImageUsageFlags::COLOR_ATTACHMENT);
		Self {
			base: base_clone,
			los: layers.into_iter()
				.map(|layer| {
					LayerObject {
						layer,
						damage: false,
						image: base.device.create_image(&create_info, None).unwrap(),
					}
				})
				.collect(),
		}
	}}

	pub fn update_all(&mut self) {
		for lo in self.los.iter_mut() {
			lo.damage = true;
		}
	}

	pub fn mark_update(&mut self, idx: usize) {
		self.los[idx].damage = true;
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
					for lo in self.los.iter_mut() {
						if lo.damage {
							let layer = lo.layer.read().unwrap();
							layer.render(draw_command_buffer, present_index);
							lo.damage = false;
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
