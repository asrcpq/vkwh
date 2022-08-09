use ash::util::*;
use ash::vk;
use std::default::Default;
use std::io::Cursor;
use std::ffi::CStr;
use std::mem;
use std::ops::Drop;
use std::sync::{Arc, RwLock};

use crate::base::{BaseRef, find_memorytype_index};
use crate::layer::Layer;
use crate::offset_of;

#[derive(Clone, Debug, Copy)]
struct Vertex {
	pos: [f32; 4],
	color: [f32; 4],
}

pub struct Triangles {
	base: BaseRef,
	graphics_pipelines: Vec<vk::Pipeline>,
	pipeline_layout: vk::PipelineLayout,
	vertex_shader_module: vk::ShaderModule,
	vertex_input_buffer: vk::Buffer,
	vertex_input_buffer_memory: vk::DeviceMemory,
	fragment_shader_module: vk::ShaderModule,
	framebuffers: Vec<vk::Framebuffer>,
	renderpass: vk::RenderPass,
	viewports: Vec<vk::Viewport>,
}

impl Triangles {
	pub fn new_ref(base: BaseRef) -> Arc<RwLock<Self>> {
		Arc::new(RwLock::new(Self::new(base)))
	}

	pub fn new(base: BaseRef) -> Self { unsafe {
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
			vk::AttachmentDescription {
				format: vk::Format::D16_UNORM,
				samples: vk::SampleCountFlags::TYPE_1,
				load_op: vk::AttachmentLoadOp::CLEAR,
				initial_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
				final_layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
				..Default::default()
			},
		];
		let color_attachment_refs = [vk::AttachmentReference {
			attachment: 0,
			layout: vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
		}];
		let depth_attachment_ref = vk::AttachmentReference {
			attachment: 1,
			layout: vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
		};
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
			.depth_stencil_attachment(&depth_attachment_ref)
			.pipeline_bind_point(vk::PipelineBindPoint::GRAPHICS);

		let renderpass_create_info = vk::RenderPassCreateInfo::default()
			.attachments(&renderpass_attachments)
			.subpasses(std::slice::from_ref(&subpass))
			.dependencies(&dependencies);

		let renderpass = device
			.create_render_pass(&renderpass_create_info, None)
			.unwrap();

		let framebuffers: Vec<vk::Framebuffer> = base
			.present_image_views
			.iter()
			.map(|&present_image_view| {
				let framebuffer_attachments = [present_image_view, base.depth_image_view];
				let frame_buffer_create_info = vk::FramebufferCreateInfo::default()
					.render_pass(renderpass)
					.attachments(&framebuffer_attachments)
					.width(base.surface_resolution.width)
					.height(base.surface_resolution.height)
					.layers(1);

				device.create_framebuffer(&frame_buffer_create_info, None)
					.unwrap()
			})
			.collect();
		let mut vertex_spv_file =
			Cursor::new(&include_bytes!("/tmp/vert.spv")[..]);
		let mut frag_spv_file = Cursor::new(&include_bytes!("/tmp/frag.spv")[..]);

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

		let layout_create_info = vk::PipelineLayoutCreateInfo::default();

		let pipeline_layout = device.create_pipeline_layout(&layout_create_info, None)
			.unwrap();

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
				offset: offset_of!(Vertex, color) as u32,
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
			size: 3 * mem::size_of::<Vertex>() as u64,
			usage: vk::BufferUsageFlags::VERTEX_BUFFER,
			sharing_mode: vk::SharingMode::EXCLUSIVE,
			..Default::default()
		};

		// TODO: do not create buffer every frame
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

		let vertices = [
			Vertex {
				pos: [-1.0, 1.0, 0.0, 1.0],
				color: [0.0, 1.0, 0.0, 1.0],
			},
			Vertex {
				pos: [1.0, 1.0, 0.0, 1.0],
				color: [0.0, 0.0, 1.0, 1.0],
			},
			Vertex {
				pos: [0.0, -1.0, 0.0, 1.0],
				color: [1.0, 0.0, 0.0, 1.0],
			},
		];

		let vert_ptr = device.map_memory(
			vertex_input_buffer_memory,
			0,
			vertex_input_buffer_memory_req.size,
			vk::MemoryMapFlags::empty(),
		)
		.unwrap();

		let mut vert_align = Align::new(
			vert_ptr,
			mem::align_of::<Vertex>() as u64,
			vertex_input_buffer_memory_req.size,
		);
		vert_align.copy_from_slice(&vertices);
		device.unmap_memory(vertex_input_buffer_memory);
		device
			.bind_buffer_memory(vertex_input_buffer, vertex_input_buffer_memory, 0)
			.unwrap();
		let viewports = vec![vk::Viewport {
			x: 0.0,
			y: 0.0,
			width: base.surface_resolution.width as f32,
			height: base.surface_resolution.height as f32,
			min_depth: 0.0,
			max_depth: 1.0,
		}];
		let scissors = [base.surface_resolution.into()];
		let viewport_state_info = vk::PipelineViewportStateCreateInfo::default()
			.scissors(&scissors)
			.viewports(&viewports);

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
		let noop_stencil_state = vk::StencilOpState {
			fail_op: vk::StencilOp::KEEP,
			pass_op: vk::StencilOp::KEEP,
			depth_fail_op: vk::StencilOp::KEEP,
			compare_op: vk::CompareOp::ALWAYS,
			..Default::default()
		};
		let depth_state_info = vk::PipelineDepthStencilStateCreateInfo {
			depth_test_enable: 1,
			depth_write_enable: 1,
			depth_compare_op: vk::CompareOp::LESS_OR_EQUAL,
			front: noop_stencil_state,
			back: noop_stencil_state,
			max_depth_bounds: 1.0,
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

		let graphic_pipeline_info = vk::GraphicsPipelineCreateInfo::default()
			.stages(&shader_stage_create_infos)
			.vertex_input_state(&vertex_input_state_info)
			.input_assembly_state(&vertex_input_assembly_state_info)
			.viewport_state(&viewport_state_info)
			.rasterization_state(&rasterization_info)
			.multisample_state(&multisample_state_info)
			.depth_stencil_state(&depth_state_info)
			.color_blend_state(&color_blend_state)
			.dynamic_state(&dynamic_state_info)
			.layout(pipeline_layout)
			.render_pass(renderpass);

		let graphics_pipelines = device
			.create_graphics_pipelines(vk::PipelineCache::null(), &[graphic_pipeline_info], None)
			.expect("Unable to create graphics pipeline");

		Self {
			base: base_clone,
			graphics_pipelines,
			pipeline_layout,
			vertex_shader_module,
			vertex_input_buffer,
			vertex_input_buffer_memory,
			fragment_shader_module,
			framebuffers,
			renderpass,
			viewports,
		}
	}}
}

impl Drop for Triangles {
	fn drop(&mut self) { unsafe {
		let base = self.base.read().unwrap();
		let device = &base.device;
		device.device_wait_idle().unwrap();
		for pipeline in mem::take(&mut self.graphics_pipelines) {
			device.destroy_pipeline(pipeline, None);
		}
		device.destroy_pipeline_layout(self.pipeline_layout, None);
		device
			.destroy_shader_module(self.vertex_shader_module, None);
		device
			.destroy_shader_module(self.fragment_shader_module, None);
		for framebuffer in mem::take(&mut self.framebuffers) {
			device.destroy_framebuffer(framebuffer, None);
		}
		device.destroy_render_pass(self.renderpass, None);
		device.free_memory(self.vertex_input_buffer_memory, None);
		device.destroy_buffer(self.vertex_input_buffer, None);
	}}
}

impl Layer for Triangles {
	fn render(&self, draw_command_buffer: vk::CommandBuffer, image_idx: u32) { unsafe {
		let clear_values = [
			vk::ClearValue {
				color: vk::ClearColorValue {
					float32: [0.0, 0.0, 0.0, 0.0],
				},
			},
			vk::ClearValue {
				depth_stencil: vk::ClearDepthStencilValue {
					depth: 1.0,
					stencil: 0,
				},
			},
		];

		let base = self.base.read().unwrap();
		let device = &base.device;
		let render_pass_begin_info = vk::RenderPassBeginInfo::default()
			.render_pass(self.renderpass)
			.framebuffer(self.framebuffers[image_idx as usize])
			.render_area(base.surface_resolution.into())
			.clear_values(&clear_values);
		device.cmd_begin_render_pass(
			draw_command_buffer,
			&render_pass_begin_info,
			vk::SubpassContents::INLINE,
		);
		device.cmd_bind_pipeline(
			draw_command_buffer,
			vk::PipelineBindPoint::GRAPHICS,
			self.graphics_pipelines[0],
		);
		device.cmd_set_viewport(draw_command_buffer, 0, &self.viewports);
        let scissors = [base.surface_resolution.into()];
		device.cmd_set_scissor(draw_command_buffer, 0, &scissors);
		device.cmd_bind_vertex_buffers(
			draw_command_buffer,
			0,
			&[self.vertex_input_buffer],
			&[0],
		);
		device.cmd_draw(draw_command_buffer, 3, 1, 0, 0);
		device.cmd_end_render_pass(draw_command_buffer);
	}}
}
