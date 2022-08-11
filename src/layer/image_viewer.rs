use std::default::Default;
use std::ffi::CStr;
use std::io::Cursor;
use std::mem;
use std::sync::{Arc, RwLock};
use ash::util::*;
use ash::vk;

use crate::offset_of;
use crate::layer::Layer;
use crate::base::{BaseRef, record_submit_commandbuffer, find_memorytype_index};

#[derive(Clone, Debug, Copy)]
struct Vertex {
	pos: [f32; 4],
	uv: [f32; 2],
}

pub struct ImageViewer {
	base: BaseRef,
	vertices: Vec<Vertex>,

	image_buffer: vk::Buffer,
	image_buffer_memory: vk::DeviceMemory,
	texture_image: vk::Image,
	texture_memory: vk::DeviceMemory,
	texture_image_view: vk::ImageView,
	descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
	descriptor_sets: Vec<vk::DescriptorSet>,
	descriptor_pool: vk::DescriptorPool,
	sampler: vk::Sampler,

	graphics_pipelines: Vec<vk::Pipeline>,
	pipeline_layout: vk::PipelineLayout,

	vertex_shader_module: vk::ShaderModule,
	vertex_input_buffer: vk::Buffer,
	vertex_input_buffer_memory: vk::DeviceMemory,
	vertex_input_buffer_memory_req: vk::MemoryRequirements,
	fragment_shader_module: vk::ShaderModule,
	output_image_view: vk::ImageView,
	framebuffer: vk::Framebuffer,
	renderpass: vk::RenderPass,
	viewports: Vec<vk::Viewport>,
}

impl ImageViewer {
	pub fn new_ref(base: BaseRef, image: image::RgbaImage) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(Self::new(base, image)))
	}

	pub fn new(base: BaseRef, image: image::RgbaImage) -> Self { unsafe {
		let base_clone = base.clone();
		let base = base.read().unwrap();
		let device = &base.device;

		let renderpass_attachments = [
			vk::AttachmentDescription {
				format: base.surface_format.format,
				samples: vk::SampleCountFlags::TYPE_1,
				load_op: vk::AttachmentLoadOp::CLEAR,
				store_op: vk::AttachmentStoreOp::STORE,
				final_layout: vk::ImageLayout::PRESENT_SRC_KHR,
				..Default::default()
			},
		];
		let color_attachment_refs = [vk::AttachmentReference {
			attachment: 0,
			layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
		}];
		let dependencies = [vk::SubpassDependency {
			src_subpass: vk::SUBPASS_EXTERNAL,
			src_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
			dst_access_mask: vk::AccessFlags::COLOR_ATTACHMENT_READ
				| vk::AccessFlags::COLOR_ATTACHMENT_WRITE,
			dst_stage_mask: vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT,
			..Default::default()
		}];

		let subpass = vk::SubpassDescription::default()
			.color_attachments(&color_attachment_refs)
			.pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

		let renderpass_create_info = vk::RenderPassCreateInfo::default()
			.attachments(&renderpass_attachments)
			.subpasses(std::slice::from_ref(&subpass))
			.dependencies(&dependencies);

		let renderpass = device
			.create_render_pass(&renderpass_create_info, None)
			.unwrap();

		let mut vertex_spv_file =
			Cursor::new(&include_bytes!("../shader/texture_vert.spv")[..]);
		let mut frag_spv_file = Cursor::new(&include_bytes!("../shader/texture_frag.spv")[..]);

		let vertex_code =
			read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
		let vertex_shader_info = vk::ShaderModuleCreateInfo::default().code(&vertex_code);

		let frag_code =
			read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
		let frag_shader_info = vk::ShaderModuleCreateInfo::default().code(&frag_code);

		let vertex_shader_module = device.create_shader_module(&vertex_shader_info, None)
			.expect("Vertex shader module error");

		let fragment_shader_module = device.create_shader_module(&frag_shader_info, None)
			.expect("Fragment shader module error");

		let shader_entry_name = CStr::from_bytes_with_nul_unchecked(b"main\0");
		let shader_stage_create_infos = [
			vk::PipelineShaderStageCreateInfo {
				module: vertex_shader_module,
				p_name: shader_entry_name.as_ptr(),
				stage: vk::ShaderStageFlags::VERTEX,
				..Default::default()
			},
			vk::PipelineShaderStageCreateInfo {
				s_type: vk::StructureType::PIPELINE_SHADER_STAGE_CREATE_INFO,
				module: fragment_shader_module,
				p_name: shader_entry_name.as_ptr(),
				stage: vk::ShaderStageFlags::FRAGMENT,
				..Default::default()
			},
		];
		let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
			binding: 0,
			stride: mem::size_of::<Vertex>() as u32,
			input_rate: vk::VertexInputRate::VERTEX,
		}];
		let vertex_input_attribute_descriptions = [
			vk::VertexInputAttributeDescription {
				location: 0,
				binding: 0,
				format: vk::Format::R32G32B32A32_SFLOAT,
				offset: offset_of!(Vertex, pos) as u32,
			},
			vk::VertexInputAttributeDescription {
				location: 1,
				binding: 0,
				format: vk::Format::R32G32B32A32_SFLOAT,
				offset: offset_of!(Vertex, uv) as u32,
			},
		];

		let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::default()
			.vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
			.vertex_binding_descriptions(&vertex_input_binding_descriptions);
		let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
			topology: vk::PrimitiveTopology::TRIANGLE_LIST,
			..Default::default()
		};
		let vertex_input_buffer_info = vk::BufferCreateInfo {
			size: 100 * mem::size_of::<Vertex>() as u64,
			usage: vk::BufferUsageFlags::VERTEX_BUFFER,
			sharing_mode: vk::SharingMode::EXCLUSIVE,
			..Default::default()
		};

		let vertices = vec![
			Vertex {
				pos: [0.0, 0.0, 0.0, 1.0],
				uv: [0.0, 0.0],
			},
			Vertex {
				pos: [0.0, 1.0, 0.0, 1.0],
				uv: [0.0, 1.0],
			},
			Vertex {
				pos: [1.0, 0.0, 0.0, 1.0],
				uv: [1.0, 0.0],
			},
			Vertex {
				pos: [0.0, 1.0, 0.0, 1.0],
				uv: [0.0, 1.0],
			},
			Vertex {
				pos: [1.0, 0.0, 0.0, 1.0],
				uv: [1.0, 0.0],
			},
			Vertex {
				pos: [1.0, 1.0, 0.0, 1.0],
				uv: [1.0, 1.0],
			},
		];
		let vertex_input_buffer = device
			.create_buffer(&vertex_input_buffer_info, None)
			.unwrap();

		let vertex_input_buffer_memory_req = device
			.get_buffer_memory_requirements(vertex_input_buffer);

		let vertex_input_buffer_memory_index = find_memorytype_index(
			&vertex_input_buffer_memory_req,
			&base.device_memory_properties,
			vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
		)
		.expect("Unable to find suitable memorytype for the vertex buffer.");

		let vertex_buffer_allocate_info = vk::MemoryAllocateInfo {
			allocation_size: vertex_input_buffer_memory_req.size,
			memory_type_index: vertex_input_buffer_memory_index,
			..Default::default()
		};

		let vertex_input_buffer_memory = device
			.allocate_memory(&vertex_buffer_allocate_info, None)
			.unwrap();
		device
			.bind_buffer_memory(vertex_input_buffer, vertex_input_buffer_memory, 0)
			.unwrap();

		let viewports = vec![vk::Viewport {
			x: 0.0,
			y: 0.0,
			width: base.render_resolution.width as f32,
			height: base.render_resolution.height as f32,
			min_depth: 0.0,
			max_depth: 1.0,
		}];
		let scissors = [base.render_resolution.into()];
		let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
			.scissors(&scissors)
			.viewports(&viewports);

		let (width, height) = image.dimensions();
		let image_extent = vk::Extent2D { width, height };
		let image_data = image.into_raw();
		let image_buffer_info = vk::BufferCreateInfo {
			size: (std::mem::size_of::<u8>() * image_data.len()) as u64,
			usage: vk::BufferUsageFlags::TRANSFER_SRC,
			sharing_mode: vk::SharingMode::EXCLUSIVE,
			..Default::default()
		};
		let image_buffer = base.device.create_buffer(&image_buffer_info, None).unwrap();
		let image_buffer_memory_req = base.device.get_buffer_memory_requirements(image_buffer);
		let image_buffer_memory_index = find_memorytype_index(
			&image_buffer_memory_req,
			&base.device_memory_properties,
			vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
		).unwrap();

		let image_buffer_allocate_info = vk::MemoryAllocateInfo {
			allocation_size: image_buffer_memory_req.size,
			memory_type_index: image_buffer_memory_index,
			..Default::default()
		};
		let image_buffer_memory = base
			.device
			.allocate_memory(&image_buffer_allocate_info, None)
			.unwrap();
		let image_ptr = base
			.device
			.map_memory(
				image_buffer_memory,
				0,
				image_buffer_memory_req.size,
				vk::MemoryMapFlags::empty(),
			)
			.unwrap();
		let mut image_slice = Align::new(
			image_ptr,
			std::mem::align_of::<u8>() as u64,
			image_buffer_memory_req.size,
		);
		image_slice.copy_from_slice(&image_data);
		base.device.unmap_memory(image_buffer_memory);
		base.device
			.bind_buffer_memory(image_buffer, image_buffer_memory, 0)
			.unwrap();

		let texture_create_info = vk::ImageCreateInfo {
			image_type: vk::ImageType::TYPE_2D,
			format: vk::Format::R8G8B8A8_UNORM,
			extent: image_extent.into(),
			mip_levels: 1,
			array_layers: 1,
			samples: vk::SampleCountFlags::TYPE_1,
			tiling: vk::ImageTiling::OPTIMAL,
			usage: vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED,
			sharing_mode: vk::SharingMode::EXCLUSIVE,
			..Default::default()
		};
		let texture_image = base
			.device
			.create_image(&texture_create_info, None)
			.unwrap();
		let texture_memory_req = base.device.get_image_memory_requirements(texture_image);
		let texture_memory_index = find_memorytype_index(
			&texture_memory_req,
			&base.device_memory_properties,
			vk::MemoryPropertyFlags::DEVICE_LOCAL,
		).unwrap();

		let texture_allocate_info = vk::MemoryAllocateInfo {
			allocation_size: texture_memory_req.size,
			memory_type_index: texture_memory_index,
			..Default::default()
		};
		let texture_memory = base
			.device
			.allocate_memory(&texture_allocate_info, None)
			.unwrap();
		base.device
			.bind_image_memory(texture_image, texture_memory, 0)
			.unwrap();

		record_submit_commandbuffer(
			&base.device,
			base.setup_command_buffer,
			base.setup_commands_reuse_fence,
			base.present_queue,
			&[],
			&[],
			&[],
			|device, texture_command_buffer| {
				let texture_barrier = vk::ImageMemoryBarrier {
					dst_access_mask: vk::AccessFlags::TRANSFER_WRITE,
					new_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					image: texture_image,
					subresource_range: vk::ImageSubresourceRange {
						aspect_mask: vk::ImageAspectFlags::COLOR,
						level_count: 1,
						layer_count: 1,
						..Default::default()
					},
					..Default::default()
				};
				device.cmd_pipeline_barrier(
					texture_command_buffer,
					vk::PipelineStageFlags::BOTTOM_OF_PIPE,
					vk::PipelineStageFlags::TRANSFER,
					vk::DependencyFlags::empty(),
					&[],
					&[],
					&[texture_barrier],
				);
				let buffer_copy_regions = vk::BufferImageCopy::default()
					.image_subresource(
						vk::ImageSubresourceLayers::default()
							.aspect_mask(vk::ImageAspectFlags::COLOR)
							.layer_count(1),
					)
					.image_extent(image_extent.into());

				device.cmd_copy_buffer_to_image(
					texture_command_buffer,
					image_buffer,
					texture_image,
					vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					&[buffer_copy_regions],
				);
				let texture_barrier_end = vk::ImageMemoryBarrier {
					src_access_mask: vk::AccessFlags::TRANSFER_WRITE,
					dst_access_mask: vk::AccessFlags::SHADER_READ,
					old_layout: vk::ImageLayout::TRANSFER_DST_OPTIMAL,
					new_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
					image: texture_image,
					subresource_range: vk::ImageSubresourceRange {
						aspect_mask: vk::ImageAspectFlags::COLOR,
						level_count: 1,
						layer_count: 1,
						..Default::default()
					},
					..Default::default()
				};
				device.cmd_pipeline_barrier(
					texture_command_buffer,
					vk::PipelineStageFlags::TRANSFER,
					vk::PipelineStageFlags::FRAGMENT_SHADER,
					vk::DependencyFlags::empty(),
					&[],
					&[],
					&[texture_barrier_end],
				);
			},
		);

		let sampler_info = vk::SamplerCreateInfo {
			mag_filter: vk::Filter::LINEAR,
			min_filter: vk::Filter::LINEAR,
			mipmap_mode: vk::SamplerMipmapMode::LINEAR,
			address_mode_u: vk::SamplerAddressMode::MIRRORED_REPEAT,
			address_mode_v: vk::SamplerAddressMode::MIRRORED_REPEAT,
			address_mode_w: vk::SamplerAddressMode::MIRRORED_REPEAT,
			max_anisotropy: 1.0,
			border_color: vk::BorderColor::FLOAT_OPAQUE_WHITE,
			compare_op: vk::CompareOp::NEVER,
			..Default::default()
		};

		let sampler = base.device.create_sampler(&sampler_info, None).unwrap();

		let texture_image_view_info = vk::ImageViewCreateInfo {
			view_type: vk::ImageViewType::TYPE_2D,
			format: texture_create_info.format,
			components: vk::ComponentMapping {
				r: vk::ComponentSwizzle::R,
				g: vk::ComponentSwizzle::G,
				b: vk::ComponentSwizzle::B,
				a: vk::ComponentSwizzle::A,
			},
			subresource_range: vk::ImageSubresourceRange {
				aspect_mask: vk::ImageAspectFlags::COLOR,
				level_count: 1,
				layer_count: 1,
				..Default::default()
			},
			image: texture_image,
			..Default::default()
		};
		let texture_image_view = base
			.device
			.create_image_view(&texture_image_view_info, None)
			.unwrap();
		let descriptor_sizes = [
			vk::DescriptorPoolSize {
				ty: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
				descriptor_count: 1,
			},
		];
		let descriptor_pool_info = vk::DescriptorPoolCreateInfo::default()
			.pool_sizes(&descriptor_sizes)
			.max_sets(1);

		let descriptor_pool = base
			.device
			.create_descriptor_pool(&descriptor_pool_info, None)
			.unwrap();
		let desc_layout_bindings = [
			vk::DescriptorSetLayoutBinding {
				descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
				descriptor_count: 1,
				stage_flags: vk::ShaderStageFlags::FRAGMENT,
				..Default::default()
			},
		];
		let descriptor_info =
			vk::DescriptorSetLayoutCreateInfo::default().bindings(&desc_layout_bindings);

		let descriptor_set_layouts = vec![base
			.device
			.create_descriptor_set_layout(&descriptor_info, None)
			.unwrap()
		];
		let desc_alloc_info = vk::DescriptorSetAllocateInfo::default()
			.descriptor_pool(descriptor_pool)
			.set_layouts(&descriptor_set_layouts);
		let descriptor_sets = base
			.device
			.allocate_descriptor_sets(&desc_alloc_info)
			.unwrap();

		let texture_descriptor = vk::DescriptorImageInfo {
			image_layout: vk::ImageLayout::SHADER_READ_ONLY_OPTIMAL,
			image_view: texture_image_view,
			sampler,
		};

		let write_desc_sets = [
			vk::WriteDescriptorSet {
				dst_set: descriptor_sets[0],
				descriptor_count: 1,
				descriptor_type: vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
				p_image_info: &texture_descriptor,
				..Default::default()
			},
		];
		base.device.update_descriptor_sets(&write_desc_sets, &[]);

		let rasterization_info = vk::PipelineRasterizationStateCreateInfo {
			front_face: vk::FrontFace::COUNTER_CLOCKWISE,
			line_width: 1.0,
			polygon_mode: vk::PolygonMode::FILL,
			..Default::default()
		};
		let multisample_state_info = vk::PipelineMultisampleStateCreateInfo {
			rasterization_samples: vk::SampleCountFlags::TYPE_1,
			..Default::default()
		};
		let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
			blend_enable: 0,
			src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
			dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
			color_blend_op: vk::BlendOp::ADD,
			src_alpha_blend_factor: vk::BlendFactor::ZERO,
			dst_alpha_blend_factor: vk::BlendFactor::ZERO,
			alpha_blend_op: vk::BlendOp::ADD,
			color_write_mask: vk::ColorComponentFlags::RGBA,
		}];
		let color_blend_state = vk::PipelineColorBlendStateCreateInfo::default()
			.logic_op(vk::LogicOp::CLEAR)
			.attachments(&color_blend_attachment_states);

		let dynamic_state = [vk::DynamicState::VIEWPORT, vk::DynamicState::SCISSOR];
		let dynamic_state_info =
			vk::PipelineDynamicStateCreateInfo::default().dynamic_states(&dynamic_state);

		let layout_create_info = vk::PipelineLayoutCreateInfo::default()
			.set_layouts(&descriptor_set_layouts);

		let pipeline_layout = device.create_pipeline_layout(&layout_create_info, None)
			.unwrap();

		let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
			.stages(&shader_stage_create_infos)
			.vertex_input_state(&vertex_input_state_info)
			.input_assembly_state(&vertex_input_assembly_state_info)
			.viewport_state(&viewport_state_info)
			.rasterization_state(&rasterization_info)
			.multisample_state(&multisample_state_info)
			.color_blend_state(&color_blend_state)
			.dynamic_state(&dynamic_state_info)
			.layout(pipeline_layout)
			.render_pass(renderpass);

		let graphics_pipelines = device
			.create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
			.expect("Unable to create graphics pipeline");

		Self {
			base: base_clone,
			vertices,
			graphics_pipelines,
			pipeline_layout,

			image_buffer,
			image_buffer_memory,
			texture_image,
			texture_memory,
			texture_image_view,
			descriptor_set_layouts,
			descriptor_sets,
			descriptor_pool,
			sampler,

			vertex_shader_module,
			vertex_input_buffer,
			vertex_input_buffer_memory,
			vertex_input_buffer_memory_req,
			fragment_shader_module,
			output_image_view: mem::zeroed(),
			framebuffer: mem::zeroed(),
			renderpass,
			viewports,
		}
	}}
}

impl Drop for ImageViewer {
	fn drop(&mut self) { unsafe {
		let base = self.base.read().unwrap();
		let device = &base.device;
		device.device_wait_idle().unwrap();
		for pipeline in mem::take(&mut self.graphics_pipelines) {
			device.destroy_pipeline(pipeline, None);
		}
		device.destroy_pipeline_layout(self.pipeline_layout, None);

		device.free_memory(self.image_buffer_memory, None);
		device.free_memory(self.texture_memory, None);
		device.destroy_buffer(self.image_buffer, None);
		device.destroy_image(self.texture_image, None);
		device.destroy_image_view(self.texture_image_view, None);
		for &descset_layout in self.descriptor_set_layouts.iter() {
			device.destroy_descriptor_set_layout(descset_layout, None);
		}
		device.destroy_descriptor_pool(self.descriptor_pool, None);
		device.destroy_sampler(self.sampler, None);

		device
			.destroy_shader_module(self.vertex_shader_module, None);
		device
			.destroy_shader_module(self.fragment_shader_module, None);
		device.destroy_image_view(self.output_image_view, None);
		device.destroy_framebuffer(self.framebuffer, None);
		device.destroy_render_pass(self.renderpass, None);
		device.free_memory(self.vertex_input_buffer_memory, None);
		device.destroy_buffer(self.vertex_input_buffer, None);
	}}
}

impl Layer for ImageViewer {
	fn set_output(&mut self, image: vk::Image) { unsafe {
		let base = self.base.read().unwrap();
		let create_view_info = vk::ImageViewCreateInfo::default()
			.view_type(vk::ImageViewType::TYPE_2D)
			.format(base.surface_format.format)
			.components(vk::ComponentMapping {
				r: vk::ComponentSwizzle::R,
				g: vk::ComponentSwizzle::G,
				b: vk::ComponentSwizzle::B,
				a: vk::ComponentSwizzle::A,
			})
			.subresource_range(vk::ImageSubresourceRange {
				aspect_mask: vk::ImageAspectFlags::COLOR,
				base_mip_level: 0,
				level_count: 1,
				base_array_layer: 0,
				layer_count: 1,
			})
			.image(image);
		let image_view = base.device.create_image_view(&create_view_info, None).unwrap();
		let framebuffer_attachments = [image_view];
		let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
			.render_pass(self.renderpass)
			.attachments(&framebuffer_attachments)
			.width(base.render_resolution.width)
			.height(base.render_resolution.height)
			.layers(1);
		let framebuffer = base.device
			.create_framebuffer(&frame_buffer_create_info, None)
			.unwrap();
		self.framebuffer = framebuffer;
		self.output_image_view = image_view;
	}}

	fn render(&self, draw_command_buffer: vk::CommandBuffer) { unsafe {
		let base = self.base.read().unwrap();
		let device = &base.device;

		let clear_values = [
			vk::ClearValue {
				color: vk::ClearColorValue {
					float32: [0.0, 0.0, 0.0, 0.0],
				},
			},
		];

		let vert_ptr = device.map_memory(
			self.vertex_input_buffer_memory,
			0,
			self.vertex_input_buffer_memory_req.size,
			vk::MemoryMapFlags::empty(),
		)
		.unwrap();
		let mut vert_align = Align::new(
			vert_ptr,
			mem::align_of::<Vertex>() as u64,
			self.vertex_input_buffer_memory_req.size,
		);
		vert_align.copy_from_slice(&self.vertices);
		device.unmap_memory(self.vertex_input_buffer_memory);

		let render_pass_begin_info = vk::RenderPassBeginInfo::default()
			.render_pass(self.renderpass)
			.framebuffer(self.framebuffer)
			.render_area(base.render_resolution.into())
			.clear_values(&clear_values);
		device.cmd_begin_render_pass(
			draw_command_buffer,
			&render_pass_begin_info,
			vk::SubpassContents::INLINE,
		);
		device.cmd_bind_descriptor_sets(
			draw_command_buffer,
			vk::PipelineBindPoint::GRAPHICS,
			self.pipeline_layout,
			0,
			&self.descriptor_sets[..],
			&[],
		);

		device.cmd_bind_pipeline(
			draw_command_buffer,
			vk::PipelineBindPoint::GRAPHICS,
			self.graphics_pipelines[0],
		);
		device.cmd_set_viewport(draw_command_buffer, 0, &self.viewports);
		let scissors = [base.render_resolution.into()];
		device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
		device.cmd_bind_vertex_buffers(
			draw_command_buffer,
			0,
			&[self.vertex_input_buffer],
			&[0],
		);
		device.cmd_draw(draw_command_buffer, 6, 1, 0, 0);
		device.cmd_end_render_pass(draw_command_buffer);
	}}
}
