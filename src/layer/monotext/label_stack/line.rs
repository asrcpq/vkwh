#[derive(Clone, Copy)]
pub enum Char {
	Byte(u8),
	SetColor([f32; 4]),
}

#[derive(Default)]
pub struct Line {
	pub data: Vec<Char>,
}

impl Line {
	pub fn new_colored(text: Vec<u8>, color: [f32; 4]) -> Self {
		let mut data = vec![Char::SetColor(color)];
		for b in text.into_iter() {
			data.push(Char::Byte(b));
		}
		Self { data }
	}
}
