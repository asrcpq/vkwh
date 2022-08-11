use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent, VirtualKeyCode as Kc};
use winit::window::WindowBuilder;
use winit::platform::run_return::EventLoopExtRunReturn;

use vkwh::base::*;
use vkwh::compositor::LayerCompositor as Vkc;
use vkwh::layer::triangles::{Triangles, Vertex};

enum CustomEvent {}

fn main() {
	let vertices = vec![
		Vertex {
			pos: [1.0, 0.0, 0.0, 1.0],
			color: [0.0, 1.0, 0.0, 0.5],
		},
		Vertex {
			pos: [0.0, 1.0, 0.0, 1.0],
			color: [0.0, 0.0, 1.0, 0.5],
		},
		Vertex {
			pos: [1.0, 1.0, 0.0, 1.0],
			color: [1.0, 0.0, 0.0, 0.5],
		},
		Vertex {
			pos: [0.0, 0.0, 0.0, 1.0],
			color: [0.0, 1.0, 0.0, 0.5],
		},
		Vertex {
			pos: [0.0, 1.0, 0.0, 1.0],
			color: [0.0, 0.0, 1.0, 0.5],
		},
		Vertex {
			pos: [1.0, 0.0, 0.0, 1.0],
			color: [1.0, 0.0, 0.0, 0.5],
		},
	];
	let mut el = EventLoop::<CustomEvent>::with_user_event();
	let window = WindowBuilder::new()
		.build(&el)
		.unwrap();
	let base = Base::new_ref(&window);
	
	let layer_t = Triangles::new_ref(base.clone());
	layer_t.write().unwrap().vertices = vertices;
	let mut vkc = Vkc::new(base.clone());
	vkc.push_layer(layer_t.clone());
	let dx = 0.1;
	el.run_return(|event, _, control_flow| {
		match event {
			Event::WindowEvent {
				event,
				..
			} => match event {
				WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
				WindowEvent::KeyboardInput {
					input,
					..
				} => match input.virtual_keycode {
					Some(Kc::H) => {
						for vertex in layer_t.write().unwrap().vertices.iter_mut().take(3) {
							vertex.pos[0] -= dx;
						}
						window.request_redraw();
					}
					Some(Kc::L) => {
						for vertex in layer_t.write().unwrap().vertices.iter_mut().take(3) {
							vertex.pos[0] += dx;
						}
						window.request_redraw();
					}
					Some(Kc::J) => {
						for vertex in layer_t.write().unwrap().vertices.iter_mut().take(3) {
							vertex.pos[1] += dx;
						}
						window.request_redraw();
					}
					Some(Kc::K) => {
						for vertex in layer_t.write().unwrap().vertices.iter_mut().take(3) {
							vertex.pos[1] -= dx;
						}
						window.request_redraw();
					}
					_ => {},
				}
				_ => {},
			}
			Event::RedrawRequested(_) => {
				vkc.update_all();
				vkc.render();
				*control_flow = ControlFlow::Wait;
			}
			_ => {},
		}
	});
}
