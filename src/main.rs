use nalgebra::{Point3, Vector3};
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
mod handle_user_input;
mod object;
mod render_system;
mod shader;
mod vertex;
mod entity;

fn build_scene(
    memory_allocator: Arc<StandardMemoryAllocator>,
) -> render_system::scene::Scene<String, vertex::mVertex> {
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

    return scene;
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

    let device_extensions = DeviceExtensions {
        khr_swapchain: true,
        ..DeviceExtensions::empty()
    };

    let window = Arc::new(WindowBuilder::new().build(&event_loop).unwrap());

    let surface = Surface::from_window(instance.clone(), window.clone()).unwrap();

    let (device, queue) = render_system::rendering3d::get_device(
        instance.clone(),
        device_extensions,
        surface.clone(),
    );

    //Print some info about the device currently being used
    println!(
        "Using device: {} (type: {:?})",
        device.physical_device().properties().device_name,
        device.physical_device().properties().device_type
    );

    let vs = shader::vert::load(device.clone())
        .unwrap()
        .entry_point("main")
        .unwrap();
    let fs = shader::frag::load(device.clone())
        .unwrap()
        .entry_point("main")
        .unwrap();

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let mut renderer = render_system::rendering3d::Renderer::new(
        vec![vs, fs],
        surface.clone(),
        queue.clone(),
        memory_allocator.clone(),
    );

    let mut camera = camera::PerspectiveCamera::new(
        Point3::new(0.0, 0.0, -1.0),
        window.inner_size().width,
        window.inner_size().height,
    );

    let mut keyboard_state = handle_user_input::KeyboardState::new();

    let mut start_time = std::time::Instant::now();
    let mut frame_count = 0;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::KeyboardInput { input, .. },
            ..
        } => {
            // Handle keyboard input
            keyboard_state.handle_keyboard_input(input);
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(size),
            ..
        } => {
            // Update the camera
            camera.set_screen(size.width, size.height);
        }
        Event::RedrawEventsCleared => {
            // Update the camera
            keyboard_state.apply_to_camera(&mut camera);

            // print FPS
            frame_count += 1;
            if frame_count > 100 {
                let elapsed = start_time.elapsed();
                println!("FPS: {}", (frame_count as f32) / (elapsed.as_secs_f32()));
                frame_count = 0;
                start_time = std::time::Instant::now();
            }

            let vertex_buffers = scene.vertex_buffers();

            let push_data = shader::vert::PushConstantData {
                mvp: camera.mvp().into(),
            };

            renderer.render([vertex_buffers], push_data);
        }
        _ => (),
    });
}
