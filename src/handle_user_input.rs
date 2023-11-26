use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

pub struct KeyboardState {
    pub w: bool,
    pub a: bool,
    pub s: bool,
    pub d: bool,
    pub q: bool,
    pub e: bool,
    pub up: bool,
    pub left: bool,
    pub down: bool,
    pub right: bool,
}

impl KeyboardState {
    pub fn new() -> KeyboardState {
        KeyboardState {
            w: false,
            a: false,
            s: false,
            d: false,
            q: false,
            e: false,
            up: false,
            left: false,
            down: false,
            right: false,
        }
    }
    pub fn handle_keyboard_input(&mut self, input: KeyboardInput) {
        if let Some(kc) = input.virtual_keycode {
            match kc {
                VirtualKeyCode::W => self.w = input.state == ElementState::Pressed,
                VirtualKeyCode::A => self.a = input.state == ElementState::Pressed,
                VirtualKeyCode::S => self.s = input.state == ElementState::Pressed,
                VirtualKeyCode::D => self.d = input.state == ElementState::Pressed,
                VirtualKeyCode::Q => self.q = input.state == ElementState::Pressed,
                VirtualKeyCode::E => self.e = input.state == ElementState::Pressed,
                VirtualKeyCode::Up => self.up = input.state == ElementState::Pressed,
                VirtualKeyCode::Left => self.left = input.state == ElementState::Pressed,
                VirtualKeyCode::Down => self.down = input.state == ElementState::Pressed,
                VirtualKeyCode::Right => self.right = input.state == ElementState::Pressed,
                _ => (),
            }
        }
    }
}

