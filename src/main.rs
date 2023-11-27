use entity::{GameWorld, InteractiveRenderingConfig};
use nalgebra::{Point3, Vector3, Isometry};
use std::collections::HashMap;
use std::convert::TryFrom;
use std::sync::Arc;
use vulkano::buffer::{Buffer, BufferCreateInfo, BufferUsage};
use vulkano::command_buffer::allocator::StandardCommandBufferAllocator;
use vulkano::command_buffer::{AutoCommandBufferBuilder, CommandBufferUsage, RenderPassBeginInfo};
use vulkano::device::physical::PhysicalDeviceType;
use vulkano::device::{
    Device, DeviceCreateInfo, DeviceExtensions, DeviceOwned, QueueCreateInfo, QueueFlags,
};
use vulkano::image::view::ImageView;
use vulkano::image::{Image, ImageCreateInfo, ImageType, ImageUsage};
use vulkano::instance::{Instance, InstanceCreateFlags, InstanceCreateInfo};
use vulkano::memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator};
use vulkano::pipeline::graphics::color_blend::{ColorBlendAttachmentState, ColorBlendState};
use vulkano::pipeline::graphics::depth_stencil::{DepthState, DepthStencilState};
use vulkano::pipeline::graphics::multisample::MultisampleState;
use vulkano::pipeline::graphics::rasterization::RasterizationState;
use vulkano::pipeline::graphics::vertex_input::{Vertex, VertexDefinition};
use vulkano::pipeline::graphics::viewport::{Viewport, ViewportState};
use vulkano::pipeline::graphics::GraphicsPipelineCreateInfo;
use vulkano::pipeline::layout::PipelineDescriptorSetLayoutCreateInfo;
use vulkano::pipeline::{
    GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
};
use vulkano::render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass};
use vulkano::shader::EntryPoint;
use vulkano::swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo};
use vulkano::sync::GpuFuture;
use vulkano::{format::*, Validated, VulkanLibrary};
use vulkano::{sync, VulkanError};
use winit::event_loop::{ControlFlow, EventLoop};

use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::window::{Window, WindowBuilder};

mod camera;
mod entity;
mod handle_user_input;
mod object;
mod render_system;
mod shader;
mod vertex;

fn build_scene(
    queue: Arc<vulkano::device::Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    surface: Arc<Surface<Window>>,
) -> GameWorld {
    let rd = vec![
        [0.0, 0.0, 0.0].into(),
        [1.0, 0.0, 0.0].into(),
        [2.0, 0.0, 0.0].into(),
        [3.0, 0.0, 0.0].into(),
        [4.0, 0.0, 0.0].into(),
        [5.0, 0.0, 0.0].into(),
        [6.0, 0.0, 0.0].into(),
        [7.0, 0.0, 0.0].into(),
        [8.0, 0.0, 0.0].into(),
        [9.0, 0.0, 0.0].into(),
        [10.0, 0.0, 0.0].into(),
        [11.0, 0.0, 0.0].into(),
        [12.0, 0.0, 0.0].into(),
        [13.0, 0.0, 0.0].into(),
        [14.0, 0.0, 0.0].into(),
        [15.0, 0.0, 0.0].into(),
        [15.0, 0.0, 1.0].into(),
        [15.0, 0.0, 2.0].into(),
        [15.0, 0.0, 3.0].into(),
        [15.0, 0.0, 4.0].into(),
        [15.0, 0.0, 5.0].into(),
        [15.0, 0.0, 6.0].into(),
        [15.0, 0.0, 7.0].into(),
        [15.0, 0.0, 8.0].into(),
        [15.0, 0.0, 9.0].into(),
        [15.0, 0.0, 10.0].into(),
        [15.0, 0.0, 11.0].into(),
        [15.0, 0.0, 12.0].into(),
        [15.0, 0.0, 13.0].into(),
        [15.0, 0.0, 14.0].into(),
        [15.0, 0.0, 15.0].into(),
    ];

    let g = vec![[0.0, -0.1, -50.0].into(), [0.0, -0.1, 50.0].into()];
    // scene
    let mut scene = render_system::scene::Scene::new(
        memory_allocator.clone(),
        HashMap::from([
            (
                "road".to_owned(),
                object::flat_polyline(rd.clone(), 1.0, [0.5, 0.5, 0.5, 1.0]),
            ),
            (
                "roadyellowline".to_owned(),
                object::flat_polyline(
                    rd.iter().map(|v| v + Vector3::new(0.0, 0.1, 0.0)).collect(),
                    0.1,
                    [1.0, 1.0, 0.0, 1.0],
                ),
            ),
            (
                "ground".to_owned(),
                object::flat_polyline(g.clone(), 50.0, [0.3, 0.5, 0.3, 1.0]),
            ),
            ("cube".to_owned(), object::unitcube()),
        ]),
    );

    let mut world = GameWorld::new(
        queue,
        memory_allocator,
        Some(InteractiveRenderingConfig {
            surface,
            tracking_entity: 0,
            camera: Box::new(camera::TrackballCamera::new(Point3::new(0.0, 0.0, -1.0))),
        }),
    );

    world.add_entity(0, EntityCreationData {
        cameras: vec![],
        physics: None,
        mesh: object::unitcube(),
        isometry: Isometry3::identity(),
    });

    world
}

fn main() {
    let library = VulkanLibrary::new().unwrap();
    let event_loop = EventLoop::new();
    let required_extensions = Surface::required_extensions(&event_loop);

    let instance = Instance::new(
        library,
        InstanceCreateInfo {
            flags: InstanceCreateFlags::ENUMERATE_PORTABILITY,
            enabled_extensions: required_extensions,
            ..Default::default()
        },
    )
    .unwrap();

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

    let surface = Surface::from_window(instance.clone(), window).unwrap();

    let (device, queue) = render_system::interactive_rendering::get_device_for_rendering_on(
        instance.clone(),
        surface.clone(),
    );

    //Print some info about the device currently being used
    println!(
        "Using device: {} (type: {:?})",
        device.physical_device().properties().device_name,
        device.physical_device().properties().device_type
    );

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let mut start_time = std::time::Instant::now();
    let mut frame_count = 0;

    let world = build_scene(queue.clone(), memory_allocator.clone(), surface.clone());

    event_loop.run_return(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        e @ Event::WindowEvent => {
            world.handle_event(e);
        }
        Event::RedrawEventsCleared => {
            // print fps
            frame_count += 1;
            let elapsed = start_time.elapsed();
            if elapsed.as_secs() >= 1 {
                println!("fps: {}", frame_count);
                frame_count = 0;
                start_time = std::time::Instant::now();
            }

            // game step and render
            game.step();
            game.render();
        }
        _ => (),
    });
}
