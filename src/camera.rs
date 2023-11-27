use nalgebra::{Matrix4, Point3, Quaternion, Vector3, Vector2, Point};

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
    let [screen_x, screeen_y] = extent;
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
    fn handle_event(&mut self, extent: [u32; 2], input: &winit::event::WindowEvent);
}

// lets you orbit around the central point by clicking and dragging
pub struct TrackballCamera {
    // position of the camera's root point
    root_pos: Point3<f32>,
    // world up
    worldup: Vector3<f32>,


    // how much to damp the rotation at each step
    damping_factor: f32,

    // to track the mouse
    user_input: UserInputState,

    // the following two quaternions are multiplied together to produce the
    // real rotation

    // the base orientation of the object
    base_q: Quaternion<f32>,
    // the rotation added by the mouse (generated each frame)
    curr_q: Quaternion<f32>,
    // the rotation used for momentum
    momentum_q: Quaternion<f32>,

    // momentumMagnitude is the magnitude of the momentum vector
    momentum_magnitude: f32,
}

impl TrackballCamera {
    pub fn new(pos: Point3<f32>) -> TrackballCamera {
        TrackballCamera {
            root_pos: pos,
            worldup: Vector3::new(0.0, -1.0, 0.0),
            damping_factor: 0.9,
            user_input: UserInputState::new(),
            base_q: Quaternion::identity(),
            curr_q: Quaternion::identity(),
            momentum_q: Quaternion::identity(),
            momentum_magnitude: 0.0,
        }
    }

    fn project_trackball(p: Vector2<f32>) -> Vector3<f32> {
        let (x, y) = (p.x, p.y);
        
        let r = 1.0;

        let z = if x * x + y * y <= r * r / 2 {
            f32::sqrt(r * r - (x * x) - (y * y))
        } else {
            (r * r / 2) / f32::sqrt(x * x + y * y)
        };

        return Vector3::new(x, -y, z);
    }

    fn get_normalized_mouse_coords(e: Vector2<f32>, extent:[u32; 2]) -> Vector2<f32> {
        let trackball_radius = extent[0].min(extent[1]) as f32;

        let center = Vector2::new(extent[0] as f32 / 2.0, extent[1] as f32 / 2.0);

        let q = Vector2::new(
            2.0 * (e.x - center.x) / trackball_radius,
            2.0 * (e.y - center.y) / trackball_radius,
        );

        return q;
    }
}

impl Camera for TrackballCamera {
    fn mvp(&self, extent: [u32; 2]) -> Matrix4<f32> {
        let projection = gen_perspective_projection(extent);
        let view = Matrix4::look_at_rh(
            &Point3::new(0.0, 0.0, 0.0),
            &self.root_pos,
            &self.worldup
        );
        projection * view
    }

    fn set_position(&mut self, pos: Point3<f32>) {
        self.root_pos = pos;
    }
}

impl InteractiveCamera 