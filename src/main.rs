use cgmath::Point3;
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
mod grid;
mod rendering3d;
mod shader;
mod util;
mod vertex;
mod handle_user_input;

use camera::*;
use grid::*;

use crate::vertex::mVertex;

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

    let (device, queue) =
        rendering3d::get_device(instance.clone(), device_extensions, surface.clone());

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



    //Compute stuff

    // The 3d size of the simulation in meters
    let sim_x_size: u32 = 10;
    let sim_y_size: u32 = 10;
    let sim_z_size: u32 = 10;

    let mut grid_buffer = GridBuffer::new(sim_x_size, sim_y_size, sim_z_size);

    for x in 0..sim_x_size {
        for z in 0..sim_y_size {
            let height = ((sim_y_size as f32) * rand::random::<f32>()) as u32;
            for y in 0..sim_z_size {
                grid_buffer.set(
                    x,
                    y,
                    z,
                    GridCell {
                        //Initialize the array to be filled with dirt halfway
                        typeCode: if y > height {
                            grid::GRIDCELL_TYPE_AIR
                        } else {
                            grid::GRIDCELL_TYPE_SOIL
                        },
                        temperature: 0,
                        moisture: 0,
                        sunlight: 0,
                        gravity: 0,
                        plantDensity: 0,
                    },
                );
            }
        }
    }

    let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

    let mut renderer = rendering3d::Renderer::new(
        vec![vs, fs],
        surface.clone(),
        queue.clone(),
        memory_allocator.clone(),
    );

    let mut camera = Camera::new(Point3::new(0.0, 0.0, -1.0), 50, 50);
    camera.set_screen(window.inner_size().width, window.inner_size().height);

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

            let vertexes = grid_buffer.gen_vertex();
            let vertex_buffer = {
                Buffer::from_iter(
                    memory_allocator.clone(),
                    BufferCreateInfo {
                        usage: BufferUsage::VERTEX_BUFFER,
                        ..Default::default()
                    },
                    AllocationCreateInfo {
                        memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                            | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                        ..Default::default()
                    },
                    vertexes.into_iter(),
                )
                .unwrap()
            };

            let push_data = shader::vert::PushConstantData {
                mvp: camera.mvp().into(),
            };

            renderer.render(vertex_buffer, push_data);
        }
        _ => (),
    });
}
