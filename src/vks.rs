use crate::vkw;

use vulkano as vk;
use vk::VulkanLibrary;
use vk::instance::{Instance, InstanceCreateInfo};
use vk::device::{DeviceCreateInfo, Features, QueueCreateInfo, Device, DeviceExtensions};
use vk::device::physical::{PhysicalDevice, PhysicalDeviceType};
use vk::swapchain::{Swapchain, SwapchainCreateInfo};
use vulkano_win::VkSurfaceBuild;
use winit::window::{Window, WindowBuilder};
use winit::event_loop::EventLoop;

#[derive(Clone)]
pub struct Vks {
	pub device: vkw::Device,
	pub queue: vkw::Queue,
	pub surface: vkw::Surface<Window>,
	pub swapchain: vkw::Swapchain<Window>,
	pub images: vkw::Images,
}

impl Vks {
	pub fn new<T>(el: &EventLoop<T>) -> Self {
		let library = VulkanLibrary::new().unwrap();
		let required_extensions = vulkano_win::required_extensions(&library);
		let instance = Instance::new(
			library,
			InstanceCreateInfo {
				enabled_extensions: required_extensions,
				..Default::default()
			},
		)
		.unwrap();

		let surface = WindowBuilder::new()
			.build_vk_surface(el, instance.clone())
			.unwrap();

		let device_extensions = DeviceExtensions {
			khr_swapchain: true,
			..DeviceExtensions::none()
		};

		let (physical_device, queue_family) = PhysicalDevice::enumerate(&instance)
			.filter(|&p| p.api_version() >= vk::Version::V1_3)
			.filter(|&p| {
				p.supported_extensions().is_superset_of(&device_extensions)
			})
			.filter_map(|p| {
				p.queue_families()
					.find(|&q| {
						q.supports_graphics()
							&& q.supports_surface(&surface).unwrap_or(false)
					})
					.map(|q| (p, q))
			})
			.min_by_key(|(p, _)| match p.properties().device_type {
				PhysicalDeviceType::DiscreteGpu => 0,
				PhysicalDeviceType::IntegratedGpu => 1,
				PhysicalDeviceType::VirtualGpu => 2,
				PhysicalDeviceType::Cpu => 3,
				PhysicalDeviceType::Other => 4,
			})
			.expect("No suitable physical device found");
		eprintln!("device: {}", physical_device.properties().device_name);

		let (device, mut queues) = Device::new(
			physical_device,
			DeviceCreateInfo {
				enabled_extensions: device_extensions,
				enabled_features: Features {
					dynamic_rendering: true,
					..Features::none()
				},
				queue_create_infos: vec![QueueCreateInfo::family(queue_family)],
				..Default::default()
			},
		)
		.unwrap();
		let queue = queues.next().unwrap();
		let (swapchain, images) = {
			let surface_caps = physical_device
				.surface_capabilities(&surface, Default::default())
				.unwrap();
			let formats = physical_device
				.surface_formats(&surface, Default::default())
				.unwrap();
			eprintln!("available formats:");
			for format in formats.iter() {
				eprintln!("\t{:?}", format);
			}
			let image_format = Some(formats[0].0);
			eprintln!("selected format: {:?}", image_format);
			let composite_alpha = surface_caps
				.supported_composite_alpha
				.iter()
				.next()
				.unwrap();
			eprintln!("composite alpha: {:?}", composite_alpha);

			Swapchain::new(
				device.clone(),
				surface.clone(),
				SwapchainCreateInfo {
					min_image_count: surface_caps.min_image_count,
					image_format,
					image_extent: surface.window().inner_size().into(),
					image_usage: vk::image::ImageUsage::color_attachment(),
					composite_alpha,
					..Default::default()
				},
			)
			.unwrap()
		};
		Self {
			device,
			queue,
			surface,
			images,
			swapchain,
		}
	}
}
