use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent};

use vkwh::vkr::Vkr;
use vkwh::vkl::dummy::Dummy;

fn main() {
	let el = EventLoop::new();
	let mut vkr = Vkr::new(&el);
	let dummy = Dummy::new_ref(vkr.vks.clone());
	vkr.push_vkl(dummy);
	el.run(move |event, _, control_flow| {
		match event {
			Event::WindowEvent {
				event: WindowEvent::CloseRequested,
				..
			} => {
				*control_flow = ControlFlow::Exit;
			},
			Event::WindowEvent {
				event: WindowEvent::Resized(_),
				..
			} => {
				vkr.recreate_swapchain();
			}
			Event::RedrawEventsCleared => {
				vkr.render();
			},
			_ => {},
		}
	});
}
