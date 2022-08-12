use winit::event_loop::{ControlFlow, EventLoop};
use winit::event::{Event, WindowEvent, VirtualKeyCode as Kc};
use winit::window::WindowBuilder;
use winit::platform::run_return::EventLoopExtRunReturn;

use vkwh::base::*;
use vkwh::compositor::LayerCompositor as Vkc;
use vkwh::layer::triangles::{Triangles, Vertex};
use vkwh::layer::monotext::Monotext;
use vkwh::layer::monotext::label_stack::line::Line;
use vkwh::layer::image_viewer::ImageViewer;

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
	let mut iter = std::env::args();
	iter.next();
	let file = iter.next().unwrap();
	let image = image::open(file).unwrap().into_rgba8();
	let mut el = EventLoop::<CustomEvent>::with_user_event();
	let window = WindowBuilder::new()
		.build(&el)
		.unwrap();
	let base = Base::new_ref(&window);
	
	let layer_t = Triangles::new_ref(base.clone());
	let layer_i = ImageViewer::new_ref(base.clone(), image);
	let layer_m = Monotext::new_ref(
		base.clone(),
		image::open("assets/images/font.png").unwrap().into_luma8(),
	);
	let txt = "hello, world".to_string();
	{
		let mut layer_m = layer_m.write().unwrap();
		layer_m.label_stack.add_text("1", Line::new_colored(
			txt.bytes().collect(),
			[1.0, 0.0, 1.0, 0.0],
		));
		layer_m.label_stack.add_text("2", Line::new_colored(
			txt.bytes().collect(),
			[1.0, 1.0, 0.0, 0.0],
		));
	}
	layer_t.write().unwrap().vertices = vertices;
	let mut vkc = Vkc::new(base.clone());
	vkc.new_cached_layer(layer_t.clone());
	vkc.new_layer(layer_m.clone());
	//vkc.new_layer(layer_i.clone());
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
