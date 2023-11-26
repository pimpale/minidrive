use std::collections::HashMap;
use std::sync::Arc;

use nalgebra::Isometry3;
use nalgebra::Vector3;
use rapier3d::dynamics::CCDSolver;
use rapier3d::dynamics::ImpulseJointSet;
use rapier3d::dynamics::IntegrationParameters;
use rapier3d::dynamics::IslandManager;
use rapier3d::dynamics::MultibodyJointSet;
use rapier3d::dynamics::RigidBodyBuilder;
use rapier3d::dynamics::RigidBodyHandle;
use rapier3d::dynamics::RigidBodySet;
use rapier3d::geometry::BroadPhase;
use rapier3d::geometry::ColliderBuilder;
use rapier3d::geometry::ColliderHandle;
use rapier3d::geometry::ColliderSet;
use rapier3d::geometry::NarrowPhase;
use rapier3d::pipeline::PhysicsPipeline;
use vulkano::buffer::Subbuffer;
use vulkano::memory::allocator::StandardMemoryAllocator;

use crate::camera::Camera;
use crate::camera::InteractiveCamera;
use crate::object;
use crate::render_system::scene::Scene;
use crate::vertex::mVertex;

struct EntityCreationPhysicsData {
    // describes the dimensions of the object (aligned with the mesh)
    hitbox: Vector3<f32>,
    // if true, the object can be moved by the physics engine
    // if false, then the object will not move due to forces. If hitbox is specified, it can still be collided with
    is_dynamic: bool,
}

struct EntityCreationData {
    // render
    interactive_camera: Option<Box<dyn InteractiveCamera>>,
    // gather data
    cameras: Vec<Box<dyn Camera>>,
    // if not specified then the object is visual only
    physics: Option<EntityCreationPhysicsData>,
    // mesh (untransformed)
    mesh: Vec<mVertex>,
    // initial transformation
    // position and rotation in space
    isometry: Isometry3<f32>,
}

struct Entity {
    // render
    interactive_camera: Option<Box<dyn InteractiveCamera>>,
    // gather data
    cameras: Vec<Box<dyn Camera>>,
    // physics
    rigid_body_handle: Option<RigidBodyHandle>,
    // mesh (untransformed)
    mesh: Vec<mVertex>,
    // transformation from origin
    isometry: Isometry3<f32>,
}

struct GameWorld {
    entities: HashMap<u32, Entity>,
    // scene for objects that change infrequently (e.g. terrain, roads)
    dynamic_scene: Scene<u32, mVertex>,
    // scene for objects that change frequently (e.g. cars, pedestrians)
    static_scene: Scene<u32, mVertex>,
    // physics data
    rigid_body_set: RigidBodySet,
    collider_set: ColliderSet,
    physics_pipeline: PhysicsPipeline,
    island_manager: IslandManager,
    broad_phase: BroadPhase,
    narrow_phase: NarrowPhase,
    impulse_joint_set: ImpulseJointSet,
    multibody_joint_set: MultibodyJointSet,
    ccd_solver: CCDSolver,
}

impl GameWorld {
    fn new(memory_allocator: Arc<StandardMemoryAllocator>) -> GameWorld {
        let dynamic_scene = Scene::new(memory_allocator.clone(), HashMap::new());
        let static_scene = Scene::new(memory_allocator.clone(), HashMap::new());

        GameWorld {
            entities: HashMap::new(),
            dynamic_scene,
            static_scene,
            rigid_body_set: RigidBodySet::new(),
            collider_set: ColliderSet::new(),
            physics_pipeline: PhysicsPipeline::new(),
            island_manager: IslandManager::new(),
            broad_phase: BroadPhase::new(),
            narrow_phase: NarrowPhase::new(),
            impulse_joint_set: ImpulseJointSet::new(),
            multibody_joint_set: MultibodyJointSet::new(),
            ccd_solver: CCDSolver::new(),
        }
    }

    fn step(&mut self) {
        // step physics
        self.physics_pipeline.step(
            &Vector3::new(0.0, -9.81, 0.0),
            &IntegrationParameters::default(),
            &mut self.island_manager,
            &mut self.broad_phase,
            &mut self.narrow_phase,
            &mut self.rigid_body_set,
            &mut self.collider_set,
            &mut self.impulse_joint_set,
            &mut self.multibody_joint_set,
            &mut self.ccd_solver,
            None,
            &(),
            &(),
        );

        // update entity positions from physics and update mesh if necessary
        for (&entity_id, entity) in self.entities.iter_mut() {
            let (mut scene, new_isometry) = match entity {
                Entity {
                    rigid_body_handle: Some(rigid_body_handle),
                    ..
                } => (
                    self.dynamic_scene,
                    self.rigid_body_set[*rigid_body_handle].position(),
                ),
                Entity { ref isometry, .. } => (self.static_scene, isometry),
            };

            if new_isometry != &entity.isometry {
                entity.isometry = new_isometry.clone();
                scene.add_object(
                    entity_id,
                    object::transform(entity.mesh.clone(), new_isometry),
                );
            }
        }
    }

    fn add_entity(&mut self, entity_id: u32, entity_creation_data: EntityCreationData) {
        let EntityCreationData {
            interactive_camera,
            cameras,
            physics,
            mesh,
            isometry,
        } = entity_creation_data;

        let (mut scene, rigid_body_handle) = match physics {
            Some(EntityCreationPhysicsData { hitbox, is_dynamic }) => {
                let rigid_body = match is_dynamic {
                    true => RigidBodyBuilder::dynamic(),
                    false => RigidBodyBuilder::fixed(),
                }
                .position(isometry)
                .build();

                let collider = ColliderBuilder::cuboid(hitbox.x, hitbox.y, hitbox.z).build();

                let rigid_body_handle = self.rigid_body_set.insert(rigid_body);
                let collider_handle = self.collider_set.insert_with_parent(
                    collider,
                    rigid_body_handle,
                    &mut self.rigid_body_set,
                );

                (self.dynamic_scene, Some(rigid_body_handle))
            }
            None => (self.static_scene, None),
        };

        scene.add_object(entity_id, object::transform(mesh, &isometry));

        self.entities.insert(
            entity_id,
            Entity {
                interactive_camera,
                cameras,
                rigid_body_handle,
                mesh,
                isometry,
            },
        );
    }

    fn remove_entity(&mut self, entity_id: u32) {
        let entity = self.entities.remove(&entity_id);
        match entity {
            Some(Entity {
                rigid_body_handle: Some(rigid_body_handle),
                ..
            }) => {
                self.rigid_body_set.remove(
                    rigid_body_handle,
                    &mut self.island_manager,
                    &mut self.collider_set,
                    &mut self.impulse_joint_set,
                    &mut self.multibody_joint_set,
                    true,
                );
            }
            _ => (),
        }
        self.dynamic_scene.remove_object(entity_id);
        self.static_scene.remove_object(entity_id);
    }

    fn get_vertex_buffers(&mut self) -> Vec<Subbuffer<[mVertex]>> {
        vec![
            self.dynamic_scene.vertex_buffer(),
            self.static_scene.vertex_buffer(),
        ]
    }

    fn handle_mouse_event(&mut self, input: &winit::event::MouseScrollDelta) {
        for (_, entity) in self.entities.iter_mut() {
            if let Some(ref mut interactive_camera) = entity.interactive_camera {
                interactive_camera.handle_mouse_event(input);
            }
        }
    }
}
