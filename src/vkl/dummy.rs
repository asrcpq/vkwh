use std::sync::{Arc, Mutex};
use bytemuck::{Pod, Zeroable};
use vulkano::pipeline::GraphicsPipeline;
use vulkano::pipeline::graphics::{
	viewport::{Viewport, ViewportState},
	input_assembly::InputAssemblyState,
	vertex_input::BuffersDefinition,
};
use vulkano::command_buffer::{
	RenderPassBeginInfo, SubpassContents,
};
use vulkano::buffer::{
	BufferUsage, CpuAccessibleBuffer, TypedBufferAccess
};
use vulkano::render_pass::Subpass;

use crate::vks::Vks;
use crate::vkl::Vkl;
use crate::{vkw, vku};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
struct Vertex {
	position: [f32; 2],
}
vulkano::impl_vertex!(Vertex, position);

mod vs {
	vulkano_shaders::shader! {
		ty: "vertex",
		src: r##"#version 450
layout(location = 0) in vec2 position;

void main() {
	gl_Position = vec4(position, 0.0, 1.0);
}
"##
	}
}

mod fs {
	vulkano_shaders::shader! {
		ty: "fragment",
		src: r##"#version 450
layout(location = 0) out vec4 f_color;
void main() {
	f_color = vec4(1.0, 0.0, 0.0, 1.0);
}
"##
	}
}

pub struct Dummy {
	vks: Vks,
	framebuffers: Vec<vkw::Framebuffer>,
	pipeline: vkw::Pipeline,
	render_pass: vkw::RenderPass,
}

impl Dummy {
	pub fn new_ref(vks: Vks) -> Arc<Mutex<Self>> {
		let render_pass = vulkano::single_pass_renderpass!(
			vks.device.clone(),
			attachments: {
				color: {
					load: Clear,
					store: Store,
					format: vks.swapchain.image_format(),
					samples: 1,
				}
			},
			pass: {
				color: [color],
				depth_stencil: {}
			}
		).unwrap();
		
		let vs = vs::load(vks.device.clone()).unwrap();
		let fs = fs::load(vks.device.clone()).unwrap();
		let pipeline = GraphicsPipeline::start()
			.render_pass(Subpass::from(render_pass.clone(), 0).unwrap())
			.vertex_input_state(BuffersDefinition::new().vertex::<Vertex>())
			.input_assembly_state(InputAssemblyState::new())
			.vertex_shader(vs.entry_point("main").unwrap(), ())
			.viewport_state(ViewportState::viewport_dynamic_scissor_irrelevant())
			.fragment_shader(fs.entry_point("main").unwrap(), ())
			.build(vks.device.clone())
			.unwrap();

		let framebuffers =
			vku::window_size_dependent_setup(render_pass.clone(), &vks.images);
		Arc::new(Mutex::new(Dummy {
			vks,
			framebuffers,
			pipeline,
			render_pass,
		}))
	}
}

impl Vkl for Dummy {
	fn render(&self, builder: &mut vkw::CommandBuilder, image_num: usize, viewport: Viewport) {
		let vertices = vec![
			Vertex {
				position: [-0.5, -0.25],
			},
			Vertex {
				position: [100.0, 100.5],
			},
			Vertex {
				position: [100.25, -100.1],
			},
		];
		let vertex_buffer =
			CpuAccessibleBuffer::from_iter(
				self.vks.device.clone(),
				BufferUsage::all(),
				false,
				vertices
			).unwrap();
		builder
			.begin_render_pass(
				RenderPassBeginInfo {
					clear_values: vec![Some([0.0, 0.0, 1.0, 1.0].into())],
					..RenderPassBeginInfo::framebuffer(self.framebuffers[image_num].clone())
				},
				SubpassContents::Inline,
			)
			.unwrap()
			.set_viewport(0, [viewport])
			.bind_pipeline_graphics(self.pipeline.clone())
			.bind_vertex_buffers(0, vertex_buffer.clone())
			.draw(vertex_buffer.len() as u32, 1, 0, 0)
			.unwrap();
		builder.end_render_pass().unwrap();
	}

	fn update_images(&mut self, images: &vkw::Images) {
		self.framebuffers =
			vku::window_size_dependent_setup(self.render_pass.clone(), images);
	}
}
