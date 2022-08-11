pub struct Dest {
	base: BaseRef,
	image: vk::Image,
}

impl Dest {
	pub fn new(base: BaseRef, image: vk::Image) {
		let base_clone = base.clone();
		let base = base_clone().read().unwrap();
		Self {
			base,
			image,
		}
	}

	pub fn render(&self, command_buffer: vk::CommandBuffer) { unsafe {
	}}
}
