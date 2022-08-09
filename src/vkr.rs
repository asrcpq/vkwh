use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage};
use vulkano::swapchain::{
	self,
	AcquireError,
	SwapchainCreationError,
	SwapchainCreateInfo,
};
use vulkano::sync::{self, GpuFuture};
use vulkano::pipeline::graphics::viewport::Viewport;
use winit::event_loop::EventLoop;

use crate::vks::Vks;
use crate::vkl::VklRef;
use crate::vkw;

pub struct Vkr {
	pub vks: Vks,
	vkls: Vec<VklRef>,
	finish: Option<vkw::Future>,
	recreate_swapchain: bool,
	viewport: Viewport,
}

impl Vkr {
	pub fn new<T>(el: &EventLoop<T>) -> Self {
		let vks = Vks::new(el);
		let finish = Some(sync::now(vks.device.clone()).boxed());
		Self {
			vks,
			vkls: Default::default(),
			finish,
			recreate_swapchain: false,
			viewport: Viewport {
				origin: [0.0, 0.0],
				dimensions: [0.0, 0.0],
				depth_range: 0.0..1.0,
			},
		}
	}

	pub fn push_vkl(&mut self, vkl: VklRef) {
		self.vkls.push(vkl);
	}

	pub fn recreate_swapchain(&mut self) {
		self.recreate_swapchain = true;
	}

	pub fn render(&mut self) {
		self.finish.as_mut().unwrap().cleanup_finished();
		if self.recreate_swapchain {
			eprintln!("recreate");
			let dimensions: [u32; 2] = self.vks.surface.window().inner_size().into();
			self.viewport.dimensions = [
				dimensions[0] as f32, dimensions[1] as f32,
			];
			if dimensions[0] == 0 || dimensions[1] == 0 {
				return;
			}
			let (new_swapchain, new_images) =
				match self.vks.swapchain.recreate(SwapchainCreateInfo {
					image_extent: dimensions,
					..self.vks.swapchain.create_info()
				}) {
					Ok(r) => r,
					Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
					Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
				};

			self.vks.swapchain = new_swapchain;
			self.vks.images = new_images.clone();
			for vkl in self.vkls.iter() {
				let mut vkl = vkl.lock().unwrap();
				vkl.update_images(&new_images);
			}
			self.recreate_swapchain = false;
		}
		let (image_num, suboptimal, acquire_future) =
			match swapchain::acquire_next_image(
				self.vks.swapchain.clone(),
				None,
			) {
				Ok(r) => r,
				Err(AcquireError::OutOfDate) => {
					self.recreate_swapchain = true;
					return;
				}
				Err(e) => {
					panic!("Failed to acquire next image: {:?}", e)
				}
			};
		if suboptimal { self.recreate_swapchain = true; }

		let mut builder = AutoCommandBufferBuilder::primary(
			self.vks.device.clone(),
			self.vks.queue.family(),
			CommandBufferUsage::OneTimeSubmit,
		)
		.unwrap();

		for vkl in self.vkls.iter() {
			let vkl = vkl.lock().unwrap();
			vkl.render(&mut builder, image_num, self.viewport.clone());
		}
		let command_buffer = Box::new(builder.build().unwrap());
		
		let future = self.finish
			.take()
			.unwrap()
			.join(acquire_future)
			.then_execute(self.vks.queue.clone(), command_buffer)
			.unwrap()
			.then_swapchain_present(
				self.vks.queue.clone(),
				self.vks.swapchain.clone(),
				image_num,
			)
			.then_signal_fence_and_flush();

		match future {
			Ok(future) => {
				self.finish = Some(future.boxed());
			}
			Err(e) => {
				println!("Failed to flush future: {:?}", e);
				self.finish =
					Some(sync::now(self.vks.device.clone()).boxed());
			}
		}
	}
}
