use std::collections::HashMap;
use std::sync::Arc;

use nalgebra::Vector3;
use rapier3d::dynamics::RigidBodyHandle;
use rapier3d::geometry::ColliderHandle;
use vulkano::memory::allocator::StandardMemoryAllocator;

use crate::camera::Camera;
use crate::camera::InteractiveCamera;
use crate::render_system::scene::Scene;
use crate::vertex::mVertex;


struct EntityCreationData {
    // render
    interactive_camera: Option<Box<dyn InteractiveCamera>>,
    // gather data
    cameras: Vec<Box<dyn Camera>>,
    // describes the dimensions of the object (aligned with the mesh)
    // if specified, object can be collided with. If not, object is visual only
    hitbox: Option<Vector3<f32>>,
    // if true, the object can be moved by the physics engine
    // if false, then the object will not move due to forces. If hitbox is specified, it can still be collided with
    is_dynamic: bool,
}


struct Entity {
    // render
    interactive_camera: Option<Box<dyn InteractiveCamera>>,
    // gather data
    cameras: Vec<Box<dyn Camera>>,
    // physics
    collider_handle: Option<ColliderHandle>,
    rigid_body_handle: Option<RigidBodyHandle>,
    // mesh (untransformed)
    mesh: Vec<mVertex>,
}

struct GameWorld {
    entities: Vec<Entity>,
    // scene for objects that change infrequently (e.g. terrain, roads)
    dynamic_scene: Scene<String, mVertex>,
    // scene for objects that change frequently (e.g. cars, pedestrians)
    static_scene: Scene<String, mVertex>,
}

fn create_gameworld(memory_allocator: Arc<StandardMemoryAllocator>) -> GameWorld {
    let dynamic_scene = Scene::new(memory_allocator.clone(), HashMap::new());
    let static_scene = Scene::new(memory_allocator.clone(), HashMap::new());

    GameWorld {
        entities: Vec::new(),
        dynamic_scene,
        static_scene,
    }
}
