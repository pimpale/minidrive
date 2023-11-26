use nalgebra::{Matrix4, Point3, Vector3};

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
    fn mvp(&self) -> Matrix4<f32>;
    fn set_position(&mut self, pos: Point3<f32>);
    fn set_screen_extent(&mut self, extent: [u32; 2]);
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

    // Projection Matrix
    projection: Matrix4<f32>,
}

impl PerspectiveCamera {
    pub fn new(pos: Point3<f32>, screen_x: u32, screen_y: u32) -> PerspectiveCamera {
        let pitch = 0.0;
        let yaw = deg2rad(-90.0);

        let worldup = Vector3::new(0.0, -1.0, 0.0);

        PerspectiveCamera {
            pos,
            worldup,
            pitch,
            yaw,
            dirs: DirVecs::new(worldup, pitch, yaw),
            projection: PerspectiveCamera::gen_projection(screen_x, screen_y),
        }
    }

    fn gen_projection(screen_x: u32, screen_y: u32) -> Matrix4<f32> {
        let aspect_ratio = screen_x as f32 / screen_y as f32;
        let fov = deg2rad(90.0);
        let near = 0.1;
        let far = 100.0;
        Matrix4::new_perspective(aspect_ratio, fov, near, far)
    }
}

impl Camera for PerspectiveCamera {
    fn mvp(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(&self.pos, &(self.pos - self.dirs.front), &self.worldup);
        self.projection * view
    }

    fn set_position(&mut self, pos: Point3<f32>) {
        self.pos = pos;
    }

    fn set_screen_extent(&mut self, extent: [u32; 2]) {
        self.projection = PerspectiveCamera::gen_projection(extent[0], extent[1]);
    }
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

    // Projection Matrix
    projection: Matrix4<f32>,
}

impl OrthogonalCamera {
    pub fn new(pos: Point3<f32>, screen_x: u32, screen_y: u32) -> OrthogonalCamera {
        let pitch = 0.0;
        let yaw = deg2rad(-90.0);

        let worldup = Vector3::new(0.0, -1.0, 0.0);

        OrthogonalCamera {
            pos,
            worldup,
            pitch,
            yaw,
            dirs: DirVecs::new(worldup, pitch, yaw),
            projection: OrthogonalCamera::gen_projection(screen_x, screen_y),
        }
    }

    fn gen_projection(screen_x: u32, screen_y: u32) -> Matrix4<f32> {
        let left = -(screen_x as f32) / 2.0;
        let right = screen_x as f32 / 2.0;
        let bottom = -(screen_y as f32) / 2.0;
        let top = screen_y as f32 / 2.0;
        Matrix4::new_orthographic(left, right, bottom, top, 0.0, 10.0)
    }
}

impl Camera for OrthogonalCamera {
    fn mvp(&self) -> Matrix4<f32> {
        let view = Matrix4::look_at_rh(&self.pos, &(self.pos - self.dirs.front), &self.worldup);
        self.projection * view
    }

    fn set_position(&mut self, pos: Point3<f32>) {
        self.pos = pos;
    }

    fn set_screen_extent(&mut self, extent: [u32; 2]) {
        self.projection = OrthogonalCamera::gen_projection(extent[0], extent[1]);
    }
}

pub trait InteractiveCamera {
    fn handle_mouse_event(&mut self, input: &winit::event::MouseScrollDelta);
}
