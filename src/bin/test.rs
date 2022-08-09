use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent};
use winit::platform::run_return::EventLoopExtRunReturn;

use vkwh::base::*;
use vkwh::layer;
use vkwh::compositor::LayerCompositor as Vkc;

fn main() {
	let mut el = EventLoop::new();
	let base = Base::new_ref(&el);
	let layer_t = layer::triangles::Triangles::new_ref(base.clone());
	let mut vkc = Vkc::new(base.clone(), vec![layer_t]);
	el.run_return(|event, _, control_flow| {
		match event {
			Event::WindowEvent {
				event: WindowEvent::CloseRequested,
				..
			} => *control_flow = ControlFlow::Exit,
			Event::MainEventsCleared => {
				vkc.mark_update(0);
				vkc.render();
			}
			_ => {},
		}
	});
}
