use vulkano as vk;
use vk::render_pass::FramebufferCreateInfo;
use vk::image::view::ImageView;

use crate::vkw;

pub fn window_size_dependent_setup(
	render_pass: vkw::RenderPass,
	images: &vkw::Images,
) -> Vec<vkw::Framebuffer> {
	images.iter()
		.map(|image| {
			let view = ImageView::new_default(image.clone()).unwrap();
			vk::render_pass::Framebuffer::new(
				render_pass.clone(),
				FramebufferCreateInfo {
					attachments: vec![view],
					..Default::default()
				},
			)
			.unwrap()
		})
		.collect::<Vec<_>>()
}
