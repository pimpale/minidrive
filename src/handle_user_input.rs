use winit::event::{ElementState, KeyboardInput, VirtualKeyCode};

use crate::camera::{PerspectiveCamera, CameraMovementDir, CameraRotationDir};

pub struct KeyboardState {
    w: bool,
    a: bool,
    s: bool,
    d: bool,
    q: bool,
    e: bool,
    up: bool,
    left: bool,
    down: bool,
    right: bool,
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

    pub fn apply_to_camera(&self, camera: &mut PerspectiveCamera) {
        if self.w {
            camera.dir_move(CameraMovementDir::Forward);
        }
        if self.a {
            camera.dir_move(CameraMovementDir::Left);
        }
        if self.s {
            camera.dir_move(CameraMovementDir::Backward);
        }
        if self.d {
            camera.dir_move(CameraMovementDir::Right);
        }
        if self.q {
            camera.dir_move(CameraMovementDir::Upward);
        }
        if self.e {
            camera.dir_move(CameraMovementDir::Downward);
        }
        if self.up {
            camera.dir_rotate(CameraRotationDir::Upward);
        }
        if self.down {
            camera.dir_rotate(CameraRotationDir::Downward);
        }
        if self.left {
            camera.dir_rotate(CameraRotationDir::Left);
        }
        if self.right {
            camera.dir_rotate(CameraRotationDir::Right);
        }
    }
    
}

