use std::sync::Arc;
use vulkano as vk;

pub type CommandBuilder = vk::command_buffer::AutoCommandBufferBuilder<vk::command_buffer::PrimaryAutoCommandBuffer>;
pub type Device = Arc<vk::device::Device>;
pub type Framebuffer = Arc<vk::render_pass::Framebuffer>;
pub type Future = Box<dyn vk::sync::GpuFuture>;
pub type Images = Vec<Arc<vk::image::SwapchainImage<winit::window::Window>>>;
pub type Instance = Arc<vk::instance::Instance>;
pub type Pipeline = Arc<vk::pipeline::GraphicsPipeline>;
pub type Queue = Arc<vk::device::Queue>;
pub type RenderPass = Arc<vk::render_pass::RenderPass>;
pub type Surface<W> = Arc<vk::swapchain::Surface<W>>;
pub type Swapchain<W> = Arc<vk::swapchain::Swapchain<W>>;
pub type TexCoords = Vec<Vec<[f32; 2]>>;
pub type TextureSet = Arc<vk::descriptor_set::PersistentDescriptorSet>;
