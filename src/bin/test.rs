use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent};
use winit::window::WindowBuilder;
use winit::platform::run_return::EventLoopExtRunReturn;

use vkwh::base::*;
use vkwh::layer;
use vkwh::compositor::LayerCompositor as Vkc;

enum CustomEvent {
	Update
}

fn main() {
	let mut el = EventLoop::<CustomEvent>::with_user_event();
	let window = WindowBuilder::new()
		.build(&el)
		.unwrap();
	let event_loop_proxy = el.create_proxy();

	let base = Base::new_ref(&window);
	
	std::thread::spawn(move || {
		loop {
			std::thread::sleep(std::time::Duration::from_millis(10));
			event_loop_proxy.send_event(CustomEvent::Update).ok();
		}
	});
	let layer_c = layer::clear::Clear::new_ref(base.clone());
	let layer_t = layer::triangles::Triangles::new_ref(base.clone());
	let mut vkc = Vkc::new(base.clone(), vec![layer_c.clone(), layer_t.clone()]);
	el.run_return(|event, _, control_flow| {
		match event {
			Event::WindowEvent {
				event: WindowEvent::CloseRequested,
				..
			} => *control_flow = ControlFlow::Exit,
			Event::RedrawRequested(_) => {
				layer_t.write().unwrap().update();
				vkc.mark_update(0);
				vkc.mark_update(1);
				vkc.render();
				*control_flow = ControlFlow::Wait;
			}
			Event::UserEvent(_) => {
				window.request_redraw();
			}
			_ => {},
		}
	});
}
