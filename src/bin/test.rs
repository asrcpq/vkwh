use winit::event_loop::EventLoop;
// use winit::event::{Event, WindowEvent};

use vkwh::vks::Vks;

fn main() {
	let el = EventLoop::new();
	let vks = Vks::new(&el);
}
