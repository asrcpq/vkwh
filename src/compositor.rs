use ash::vk;

use crate::layer::LayerRef;
use crate::base::{Base, BaseRef, record_submit_commandbuffer, find_memorytype_index};

pub struct LayerCache {
	pub image: vk::Image,
	pub memory: vk::DeviceMemory,
	pub damage: bool,
}

pub struct LayerObject {
	device: ash::Device,
	layer: LayerRef,
	// no cache: render every time
	cache: Option<LayerCache>,
}

impl LayerObject {
	pub fn new(base: &Base, layer: LayerRef) -> Self {
		Self {
			device: base.device.clone(),
			layer,
			cache: None,
		}
	}

	pub fn build_instant(self, base: &Base) -> Self {
		self.layer.write().unwrap().set_output(base.present_images.clone());
		self
	}

	pub fn build_cache(mut self, base: &Base) -> Self {
		let (image, memory) = unsafe {
			let create_info = vk::ImageCreateInfo::default()
				.image_type(vk::ImageType::TYPE_2D)
				.format(base.surface_format.format)
				.extent(base.render_resolution.into())
				.mip_levels(1)
				.array_layers(1)
				.samples(vk::SampleCountFlags::TYPE_1)
				.tiling(vk::ImageTiling::OPTIMAL)
				.usage(vk::ImageUsageFlags::COLOR_ATTACHMENT |
					vk::ImageUsageFlags::TRANSFER_DST |
					vk::ImageUsageFlags::TRANSFER_SRC);
			let image = base.device.create_image(&create_info, None).unwrap();
			let memory_req = base.device.get_image_memory_requirements(image);
			let memory_index = find_memorytype_index(
				&memory_req,
				&base.device_memory_properties,
				vk::MemoryPropertyFlags::DEVICE_LOCAL,
			).unwrap();
			let allocate_info = vk::MemoryAllocateInfo {
				allocation_size: memory_req.size,
				memory_type_index: memory_index,
				..Default::default()
			};
			let memory = base
				.device
				.allocate_memory(&allocate_info, None)
				.unwrap();
			base.device
				.bind_image_memory(image, memory, 0)
				.expect("Unable to bind depth image memory");
			(image, memory)
		};
		self.layer.write().unwrap().set_output(vec![image]);
		self.cache = Some(LayerCache {
			image,
			memory,
			damage: true,
		});
		self
	}
}

impl Drop for LayerObject {
	fn drop(&mut self) { unsafe {
		self.device.device_wait_idle().unwrap();
		if let Some(cache) = self.cache.take() {
			self.device.destroy_image(cache.image, None);
			self.device.free_memory(cache.memory, None);
		}
	}}
}

struct BarrierBuilder {
	pub subresource_range: vk::ImageSubresourceRange,
	pub device: ash::Device,
	pub command: vk::CommandBuffer,
}

impl BarrierBuilder {
	pub fn new(device: ash::Device, command: vk::CommandBuffer) -> Self {
		let subresource_range = vk::ImageSubresourceRange {
			aspect_mask: vk::ImageAspectFlags::COLOR,
			level_count: 1,
			layer_count: 1,
			..Default::default()
		};
		Self {
			subresource_range,
			device,
			command,
		}
	}

	pub fn build(&self, image: vk::Image, from: vk::ImageLayout, to: vk::ImageLayout) { unsafe {
		let barrier = vk::ImageMemoryBarrier {
			dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
			old_layout: from,
			new_layout: to,
			image,
			subresource_range: self.subresource_range,
			..Default::default()
		};
		self.device.cmd_pipeline_barrier(
			self.command,
			vk::PipelineStageFlags::BOTTOM_OF_PIPE,
			vk::PipelineStageFlags::TRANSFER,
			vk::DependencyFlags::empty(),
			&[],
			&[],
			&[barrier],
		);
	}}
}

pub struct LayerCompositor {
	// TODO: prevent base lock
	base: BaseRef,
	los: Vec<LayerObject>,
}

impl LayerCompositor {
	pub fn new(base: BaseRef) -> Self {
		Self {
			base,
			los: Vec::new(),
		}
	}

	pub fn new_layer(&mut self, layer: LayerRef) {
		let base = self.base.read().unwrap();
		self.los.push(LayerObject::new(&base, layer).build_instant(&base));
	}

	pub fn new_cached_layer(&mut self, layer: LayerRef) {
		let base = self.base.read().unwrap();
		self.los.push(LayerObject::new(&base, layer).build_cache(&base));
	}

	pub fn update_all(&mut self) {
		for lo in self.los.iter_mut() {
			if let Some(mut cache) = lo.cache.as_mut() {
				cache.damage = true;
			}
		}
	}

	pub fn mark_update(&mut self, idx: usize) {
		if let Some(mut cache) = self.los[idx].cache.as_mut() {
			cache.damage = true;
		}
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
				|device, command_buffer| {
					for lo in self.los.iter_mut() {
						if let Some(cache) = lo.cache.as_mut() {
							if cache.damage {
								let layer = lo.layer.read().unwrap();
								layer.render(command_buffer, 0);
								cache.damage = false;
							}
						}
					}
					let image = base.present_images[present_index as usize];
					let bb = BarrierBuilder::new(device.clone(), command_buffer);
					bb.build(
						image,
						vk::ImageLayout::UNDEFINED,
						vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					);
					device.cmd_clear_color_image(
						command_buffer,
						image,
						vk::ImageLayout::TRANSFER_DST_OPTIMAL,
						&vk::ClearColorValue {
							float32: [0.0, 0.0, 0.0, 0.0],
						},
						&[bb.subresource_range],
					);
					bb.build(
						image,
						vk::ImageLayout::TRANSFER_DST_OPTIMAL,
						vk::ImageLayout::PRESENT_SRC_KHR,
					);
					let subresource = vk::ImageSubresourceLayers {
						aspect_mask: vk::ImageAspectFlags::COLOR,
						mip_level: 0,
						base_array_layer: 0,
						layer_count: 1,
					};
					let whole_region = vk::ImageCopy {
						src_subresource: subresource,
						dst_subresource: subresource,
						extent: base.render_resolution.into(),
						..Default::default()
					};
					let mut prev_copy = false;
					for lo in self.los.iter() {
						if let Some(cache) = &lo.cache {
							if !prev_copy {
								bb.build(
									image,
									vk::ImageLayout::PRESENT_SRC_KHR,
									vk::ImageLayout::TRANSFER_DST_OPTIMAL,
								);
							}
							prev_copy = true;
							bb.build(
								cache.image,
								vk::ImageLayout::PRESENT_SRC_KHR,
								vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
							);
							device.cmd_copy_image(
								command_buffer,
								cache.image,
								vk::ImageLayout::TRANSFER_SRC_OPTIMAL,
								image,
								vk::ImageLayout::TRANSFER_DST_OPTIMAL,
								&[whole_region],
							);
						} else {
							if prev_copy {
								bb.build(
									image,
									vk::ImageLayout::TRANSFER_DST_OPTIMAL,
									vk::ImageLayout::PRESENT_SRC_KHR,
								);
							}
							prev_copy = false;
							let layer = lo.layer.read().unwrap();
							layer.render(command_buffer, present_index as usize);
						}
					}
					if prev_copy {
						bb.build(
							image,
							vk::ImageLayout::TRANSFER_DST_OPTIMAL,
							vk::ImageLayout::PRESENT_SRC_KHR,
						);
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
