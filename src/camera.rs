use nalgebra::{Matrix, Matrix4, Point, Point3, Quaternion, UnitQuaternion, Vector2, Vector3, Point2};
use winit::event::ElementState;

use crate::handle_user_input::UserInputState;

fn deg2rad(deg: f32) -> f32 {
    deg * std::f32::consts::PI / 180.0
}

// vectors giving the current perception of the camera
#[derive(Clone, Debug)]
struct DirVecs {
    // NOTE: front is actually backwards
    front: Vector3<f32>,
    right: Vector3<f32>,
    up: Vector3<f32>,
}

impl DirVecs {
    fn new(worldup: Vector3<f32>, pitch: f32, yaw: f32) -> DirVecs {
        let front = Vector3::new(
            yaw.cos() * pitch.cos(),
            pitch.sin(),
            yaw.sin() * pitch.cos(),
        )
        .normalize();
        // get other vectors
        let right = front.cross(&worldup).normalize();
        let up = right.cross(&front).normalize();
        // return values
        DirVecs { front, right, up }
    }
}

pub trait Camera {
    fn mvp(&self, extent: [u32; 2]) -> Matrix4<f32>;
    fn set_position(&mut self, pos: Point3<f32>);
}

fn gen_perspective_projection(extent: [u32; 2]) -> Matrix4<f32> {
    let [screen_x, screen_y] = extent;
    let aspect_ratio = screen_x as f32 / screen_y as f32;
    let fov = deg2rad(90.0);
    let near = 0.1;
    let far = 100.0;
    Matrix4::new_perspective(aspect_ratio, fov, near, far)
}

#[derive(Clone, Debug)]
pub struct PerspectiveCamera {
    // global camera position
    pos: Point3<f32>,
    // the global up
    worldup: Vector3<f32>,

    // pitch and yaw values
    pitch: f32,
    yaw: f32,

    // relative directions
    dirs: DirVecs,
}

impl PerspectiveCamera {
    pub fn new(pos: Point3<f32>) -> PerspectiveCamera {
        let pitch = 0.0;
        let yaw = deg2rad(-90.0);

        let worldup = Vector3::new(0.0, -1.0, 0.0);

        PerspectiveCamera {
            pos,
            worldup,
            pitch,
            yaw,
            dirs: DirVecs::new(worldup, pitch, yaw),
        }
    }
}

impl Camera for PerspectiveCamera {
    fn mvp(&self, extent: [u32; 2]) -> Matrix4<f32> {
        let projection = gen_perspective_projection(extent);
        let view = Matrix4::look_at_rh(&self.pos, &(self.pos - self.dirs.front), &self.worldup);
        projection * view
    }

    fn set_position(&mut self, pos: Point3<f32>) {
        self.pos = pos;
    }
}

fn gen_orthogonal_projection([screen_x, screen_y]: [u32; 2]) -> Matrix4<f32> {
    let left = -(screen_x as f32) / 2.0;
    let right = screen_x as f32 / 2.0;
    let bottom = -(screen_y as f32) / 2.0;
    let top = screen_y as f32 / 2.0;
    Matrix4::new_orthographic(left, right, bottom, top, 0.0, 10.0)
}

#[derive(Clone, Debug)]
pub struct OrthogonalCamera {
    // pitch and yaw values
    pitch: f32,
    yaw: f32,

    // global camera position
    pos: Point3<f32>,
    // the global up
    worldup: Vector3<f32>,

    // relative directions
    dirs: DirVecs,
}

impl OrthogonalCamera {
    pub fn new(pos: Point3<f32>) -> OrthogonalCamera {
        let pitch = 0.0;
        let yaw = deg2rad(-90.0);

        let worldup = Vector3::new(0.0, -1.0, 0.0);

        OrthogonalCamera {
            pos,
            worldup,
            pitch,
            yaw,
            dirs: DirVecs::new(worldup, pitch, yaw),
        }
    }
}

impl Camera for OrthogonalCamera {
    fn mvp(&self, extent: [u32; 2]) -> Matrix4<f32> {
        let projection = gen_orthogonal_projection(extent);
        let view = Matrix4::look_at_rh(&self.pos, &(self.pos - self.dirs.front), &self.worldup);
        projection * view
    }

    fn set_position(&mut self, pos: Point3<f32>) {
        self.pos = pos;
    }
}

pub trait InteractiveCamera: Camera {
    fn update(&mut self);
    fn handle_event(&mut self, extent: [u32; 2], input: &winit::event::WindowEvent);
}

// lets you orbit around the central point by clicking and dragging
pub struct TrackballCamera {
    // position of the camera's root point
    root_pos: Point3<f32>,
    // world up
    worldup: Vector3<f32>,
    // offset from the root position
    offset: f32,

    // contains mouse data (if being dragged)
    mouse_down: bool,
    mouse_start: Point2<f32>,
    mouse_prev: Point2<f32>,
    mouse_curr: Point2<f32>,

    // how much to damp the rotation at each step
    damping_factor: f32,

    // the following two quaternions are multiplied together to produce the
    // real rotation

    // the base orientation of the object
    base_q: UnitQuaternion<f32>,
    // the rotation added by the mouse (generated each frame)
    curr_q: UnitQuaternion<f32>,
    // the rotation used for momentum
    momentum_q: UnitQuaternion<f32>,

    // momentumMagnitude is the magnitude of the momentum vector
    momentum_magnitude: f32,
}

impl TrackballCamera {
    pub fn new(pos: Point3<f32>) -> TrackballCamera {
        TrackballCamera {
            root_pos: pos,
            worldup: Vector3::new(0.0, -1.0, 0.0),
            offset: 3.0,
            damping_factor: 0.9,
            mouse_down: false,
            mouse_start: Default::default(),
            mouse_prev: Default::default(),
            mouse_curr: Default::default(),
            base_q: UnitQuaternion::identity(),
            curr_q: UnitQuaternion::identity(),
            momentum_q: UnitQuaternion::identity(),
            momentum_magnitude: 0.0,
        }
    }

    fn project_trackball(p: &Point2<f32>) -> Vector3<f32> {
        let (x, y) = (p.x, p.y);

        let r = 1.0;

        let z = if x * x + y * y <= r * r / 2.0 {
            f32::sqrt(r * r - (x * x) - (y * y))
        } else {
            (r * r / 2.0) / f32::sqrt(x * x + y * y)
        };

        return Vector3::new(x, -y, z);
    }

    fn get_normalized_mouse_coords(e: Point2<f32>, extent: [u32; 2]) -> Point2<f32> {
        let trackball_radius = extent[0].min(extent[1]) as f32;
        let center = Vector2::new(extent[0] as f32 / 2.0, extent[1] as f32 / 2.0);
        return (e - center) / trackball_radius;
    }
}

impl Camera for TrackballCamera {
    fn mvp(&self, extent: [u32; 2]) -> Matrix4<f32> {
        let rot = self.curr_q * self.base_q;
        let model = Matrix4::new_translation(
            &(self.root_pos.coords + rot * Vector3::new(self.offset, self.offset, self.offset)),
        );
        let view = Matrix4::from(rot);
        let projection = gen_perspective_projection(extent);
        projection * view * model
    }

    fn set_position(&mut self, pos: Point3<f32>) {
        self.root_pos = pos;
    }
}

impl InteractiveCamera for TrackballCamera {
    fn update(&mut self) {
        if !self.mouse_down {
            let combined_q =
                UnitQuaternion::identity().slerp(&self.momentum_q, self.momentum_magnitude);
            self.momentum_magnitude *= self.damping_factor;
            self.base_q = combined_q * self.base_q;
        }
    }

    fn handle_event(&mut self, extent: [u32; 2], event: &winit::event::WindowEvent) {
        match event {
            // mouse down
            winit::event::WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            } => {
                self.mouse_down = true;
                self.mouse_start = self.mouse_curr;
            }
            // cursor move
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                self.mouse_prev = self.mouse_curr;
                self.mouse_curr = Point2::new(position.x as f32, position.y as f32);
                if self.mouse_down {
                    // current and past
                    let curr_t = Self::project_trackball(&self.mouse_curr).normalize();
                    let start_t = Self::project_trackball(&self.mouse_start).normalize();

                    self.curr_q = UnitQuaternion::rotation_between(&start_t, &curr_t).unwrap();
                }
            }
            // mouse up
            winit::event::WindowEvent::MouseInput {
                state: ElementState::Released,
                ..
            } => {
                if self.mouse_down {
                    let curr = Self::project_trackball(&self.mouse_curr).normalize();
                    let prev = Self::project_trackball(&self.mouse_prev).normalize();
                    // create momentum (for less jerky camera movements)
                    self.momentum_q =
                        UnitQuaternion::rotation_between(&prev, &curr).unwrap_or_default();
                    self.momentum_magnitude = 1.0;

                    // set the base rotation our rotation that we were thinking about
                    self.base_q = self.curr_q * self.base_q;

                    // reset the current rotation
                    self.curr_q = UnitQuaternion::identity();
                }
                self.mouse_down = false;
            }
            _ => {}
        }
    }
}
