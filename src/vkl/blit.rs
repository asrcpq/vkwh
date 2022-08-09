use bytemuck::{Pod, Zeroable};
use std::sync::{Arc, Mutex};
use std::ops::DerefMut;
use vulkano::buffer::{
	BufferUsage,
	CpuAccessibleBuffer,
};
use vulkano::image::{
	ImageAspects,
	ImageSubresourceLayers,
	ImageLayout,
};
use vulkano::command_buffer::{
	BufferImageCopy,
	CopyBufferToImageInfo,
};
use vulkano::pipeline::graphics::viewport::Viewport;

use crate::vks::Vks;
use crate::vkl::Vkl;
use crate::vkw;

const MAX_SIZE: [u32; 2] = [3840, 2160];

#[repr(C)]
#[derive(Copy, Clone, Debug, Default, Pod, Zeroable)]
pub struct Pixel {
	r: u8,
	g: u8,
	b: u8,
	a: u8,
}

pub struct Blit {
	vks: Vks,
	buffer: Arc<CpuAccessibleBuffer<[Pixel]>>,
	size: [u32; 2],
}

impl Blit {
	pub fn new_ref(vks: Vks, size: [u32; 2]) -> Arc<Mutex<Self>> {
		let buffer = unsafe {
			CpuAccessibleBuffer::uninitialized_array(
				vks.device.clone(),
				(MAX_SIZE[0] * MAX_SIZE[1]) as u64,
				BufferUsage::transfer_src(),
				true,
			).unwrap()
		};
		Arc::new(Mutex::new(Blit {
			buffer,
			vks,
			size,
		}))
	}
}

impl Vkl for Blit {
	fn render(&self, builder: &mut vkw::CommandBuilder, image_num: usize, _viewport: Viewport) {
		let subresource = ImageSubresourceLayers {
			aspects: ImageAspects {
				color: true,
				..ImageAspects::none()
			},
			mip_level: 0,
			array_layers: 0..1,
		};
		let image_copy = BufferImageCopy {
			buffer_row_length: MAX_SIZE[0] * 4,
			buffer_image_height: MAX_SIZE[1],
			image_subresource: subresource,
			image_extent: [self.size[0], self.size[1], 0],
			..Default::default()
		}; // TODO save image copy
		let copyinfo = CopyBufferToImageInfo {
			dst_image_layout: ImageLayout::General,
			regions: vec![image_copy].into(),
			..CopyBufferToImageInfo::buffer_image(
				self.buffer.clone(),
				self.vks.images[image_num].clone(),
			)
		};
		builder.copy_buffer_to_image(copyinfo).unwrap();
	}

	fn update_images(&mut self, _images: &vkw::Images) {}
}
