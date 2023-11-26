use std::sync::Arc;

use vulkano::{
    buffer::{BufferContents, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        RenderPassBeginInfo,
    },
    device::{
        physical::PhysicalDeviceType, Device, DeviceCreateInfo, DeviceExtensions, DeviceOwned,
        Queue, QueueCreateInfo, QueueFlags,
    },
    format::Format,
    image::{view::ImageView, Image, ImageCreateInfo, ImageType, ImageUsage},
    instance::Instance,
    memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
    pipeline::{
        graphics::{
            color_blend::{ColorBlendAttachmentState, ColorBlendState},
            depth_stencil::{DepthState, DepthStencilState},
            input_assembly::InputAssemblyState,
            multisample::MultisampleState,
            rasterization::RasterizationState,
            vertex_input::{Vertex, VertexBufferDescription, VertexDefinition},
            viewport::{Viewport, ViewportState},
            GraphicsPipelineCreateInfo,
        },
        layout::PipelineDescriptorSetLayoutCreateInfo,
        GraphicsPipeline, Pipeline, PipelineLayout, PipelineShaderStageCreateInfo,
    },
    render_pass::{Framebuffer, FramebufferCreateInfo, RenderPass, Subpass},
    shader::{spirv::ExecutionModel, EntryPoint},
    swapchain::{self, Surface, Swapchain, SwapchainCreateInfo, SwapchainPresentInfo},
    sync::{self, GpuFuture},
    Validated, VulkanError,
};
use winit::window::Window;

pub fn get_device(
    instance: Arc<Instance>,
    device_extensions: DeviceExtensions,
    surface: Arc<Surface>,
) -> (Arc<Device>, Arc<Queue>) {
    // We then choose which physical device to use. First, we enumerate all the available physical
    // devices, then apply filters to narrow them down to those that can support our needs.
    let (physical_device, queue_family_index) = instance
        .enumerate_physical_devices()
        .unwrap()
        .filter(|p| {
            // Some devices may not support the extensions or features that your application, or
            // report properties and limits that are not sufficient for your application. These
            // should be filtered out here.
            p.supported_extensions().contains(&device_extensions)
        })
        .filter_map(|p| {
            // For each physical device, we try to find a suitable queue family that will execute
            // our draw commands.
            //
            // Devices can provide multiple queues to run commands in parallel (for example a draw
            // queue and a compute queue), similar to CPU threads. This is something you have to
            // have to manage manually in Vulkan. Queues of the same type belong to the same
            // queue family.
            //
            // Here, we look for a single queue family that is suitable for our purposes. In a
            // real-life application, you may want to use a separate dedicated transfer queue to
            // handle data transfers in parallel with graphics operations. You may also need a
            // separate queue for compute operations, if your application uses those.
            p.queue_family_properties()
                .iter()
                .enumerate()
                .position(|(i, q)| {
                    // We select a queue family that supports graphics operations. When drawing to
                    // a window surface, as we do in this example, we also need to check that queues
                    // in this queue family are capable of presenting images to the surface.
                    q.queue_flags.intersects(QueueFlags::GRAPHICS)
                        && p.surface_support(i as u32, &surface).unwrap_or(false)
                })
                // The code here searches for the first queue family that is suitable. If none is
                // found, `None` is returned to `filter_map`, which disqualifies this physical
                // device.
                .map(|i| (p, i as u32))
        })
        // All the physical devices that pass the filters above are suitable for the application.
        // However, not every device is equal, some are preferred over others. Now, we assign
        // each physical device a score, and pick the device with the
        // lowest ("best") score.
        //
        // In this example, we simply select the best-scoring device to use in the application.
        // In a real-life setting, you may want to use the best-scoring device only as a
        // "default" or "recommended" device, and let the user choose the device themselves.
        .min_by_key(|(p, _)| {
            // We assign a lower score to device types that are likely to be faster/better.
            match p.properties().device_type {
                PhysicalDeviceType::DiscreteGpu => 0,
                PhysicalDeviceType::IntegratedGpu => 1,
                PhysicalDeviceType::VirtualGpu => 2,
                PhysicalDeviceType::Cpu => 3,
                PhysicalDeviceType::Other => 4,
                _ => 5,
            }
        })
        .expect("No suitable physical device found");

    let (device, mut queues) = Device::new(
        physical_device,
        DeviceCreateInfo {
            enabled_extensions: device_extensions,
            queue_create_infos: vec![QueueCreateInfo {
                queue_family_index,
                ..Default::default()
            }],
            ..Default::default()
        },
    )
    .unwrap();

    let queue = queues.next().unwrap();

    (device, queue)
}

/// This function is called once during initialization, then again whenever the window is resized.
fn window_size_dependent_setup(
    memory_allocator: Arc<StandardMemoryAllocator>,
    images: &[Arc<Image>],
    render_pass: Arc<RenderPass>,
    stages: Vec<EntryPoint>,
    vertex_buffer_descriptions: &[VertexBufferDescription],
) -> (Arc<GraphicsPipeline>, Vec<Arc<Framebuffer>>) {
    // validate stages
    assert!(stages.len() > 0, "no shader stages provided");
    assert!(
        stages[0].info().execution_model == ExecutionModel::Vertex,
        "first shader stage must be vertex shader"
    );

    let device = memory_allocator.device().clone();
    let extent = images[0].extent();

    let depth_buffer = ImageView::new_default(
        Image::new(
            memory_allocator,
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent: images[0].extent(),
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();

    let framebuffers = images
        .iter()
        .map(|image| {
            let view = ImageView::new_default(image.clone()).unwrap();
            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo {
                    attachments: vec![view, depth_buffer.clone()],
                    ..Default::default()
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();

    let vs = stages[0].clone();

    // In the triangle example we use a dynamic viewport, as its a simple example. However in the
    // teapot example, we recreate the pipelines with a hardcoded viewport instead. This allows the
    // driver to optimize things, at the cost of slower window resizes.
    // https://computergraphics.stackexchange.com/questions/5742/vulkan-best-way-of-updating-pipeline-viewport
    let pipeline = {
        let vertex_input_state = vertex_buffer_descriptions
            .definition(&vs.info().input_interface)
            .unwrap();
        let stages: Vec<_> = stages
            .into_iter()
            .map(PipelineShaderStageCreateInfo::new)
            .collect();
        let layout = PipelineLayout::new(
            device.clone(),
            PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap(),
        )
        .unwrap();
        let subpass = Subpass::from(render_pass, 0).unwrap();

        GraphicsPipeline::new(
            device,
            None,
            GraphicsPipelineCreateInfo {
                stages: stages.into_iter().collect(),
                vertex_input_state: Some(vertex_input_state),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState {
                    viewports: [Viewport {
                        offset: [0.0, 0.0],
                        extent: [extent[0] as f32, extent[1] as f32],
                        depth_range: 0.0..=1.0,
                    }]
                    .into_iter()
                    .collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState::default()),
                depth_stencil_state: Some(DepthStencilState {
                    depth: Some(DepthState::simple()),
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState::default()),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState::default(),
                )),
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(layout)
            },
        )
        .unwrap()
    };

    (pipeline, framebuffers)
}

fn create_swapchain(
    device: Arc<Device>,
    surface: Arc<Surface>,
) -> (Arc<Swapchain>, Vec<Arc<Image>>) {
    // Querying the capabilities of the surface. When we create the swapchain we can only
    // pass values that are allowed by the capabilities.
    let surface_capabilities = device
        .physical_device()
        .surface_capabilities(&surface, Default::default())
        .unwrap();

    // Choosing the internal format that the images will have.
    let image_format = device
        .physical_device()
        .surface_formats(&surface, Default::default())
        .unwrap()[0]
        .0;

    let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();

    // Please take a look at the docs for the meaning of the parameters we didn't mention.
    Swapchain::new(
        device.clone(),
        surface.clone(),
        SwapchainCreateInfo {
            min_image_count: surface_capabilities.min_image_count,

            image_format,
            // The dimensions of the window, only used to initially setup the swapchain.
            // NOTE:
            // On some drivers the swapchain dimensions are specified by
            // `surface_capabilities.current_extent` and the swapchain size must use these
            // dimensions.
            // These dimensions are always the same as the window dimensions.
            //
            // However, other drivers don't specify a value, i.e.
            // `surface_capabilities.current_extent` is `None`. These drivers will allow
            // anything, but the only sensible value is the window
            // dimensions.
            //
            // Both of these cases need the swapchain to use the window dimensions, so we just
            // use that.
            image_extent: window.inner_size().into(),

            image_usage: ImageUsage::COLOR_ATTACHMENT,

            // The alpha mode indicates how the alpha value of the final image will behave. For
            // example, you can choose whether the window will be opaque or transparent.
            composite_alpha: surface_capabilities
                .supported_composite_alpha
                .into_iter()
                .next()
                .unwrap(),

            ..Default::default()
        },
    )
    .unwrap()
}

pub struct Renderer<Vert> {
    stages: Vec<EntryPoint>,
    surface: Arc<Surface>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    render_pass: Arc<RenderPass>,
    swapchain: Arc<Swapchain>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    vertex_buffer_descriptions: Vec<VertexBufferDescription>,
    wdd_needs_rebuild: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    phantom: std::marker::PhantomData<Vert>,
}

impl<T> Renderer<T> {
    pub fn new(
        stages: Vec<EntryPoint>,
        surface: Arc<Surface>,
        queue: Arc<Queue>,
        memory_allocator: Arc<StandardMemoryAllocator>,
    ) -> Renderer<T>
    where
        T: Vertex,
    {
        let device = memory_allocator.device().clone();

        let (swapchain, images) = create_swapchain(device.clone(), surface.clone());

        let vertex_buffer_descriptions = [T::per_vertex()];

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: swapchain.image_format(),
                    samples: 1,
                    load_op: Clear,
                    store_op: Store,
                },
                depth_stencil: {
                    format: Format::D32_SFLOAT,
                    samples: 1,
                    load_op: Clear,
                    store_op: DontCare,
                },
            },
            pass: {
                color: [color],
                depth_stencil: {depth_stencil},
            },
        )
        .unwrap();

        let (pipeline, framebuffers) = window_size_dependent_setup(
            memory_allocator.clone(),
            &images,
            render_pass.clone(),
            stages.clone(),
            &vertex_buffer_descriptions,
        );

        Renderer {
            stages,
            surface,
            command_buffer_allocator: Arc::new(StandardCommandBufferAllocator::new(
                device.clone(),
                Default::default(),
            )),
            previous_frame_end: Some(sync::now(device.clone()).boxed()),
            device,
            queue,
            swapchain,
            pipeline,
            framebuffers,
            memory_allocator,
            render_pass,
            wdd_needs_rebuild: false,
            vertex_buffer_descriptions: vertex_buffer_descriptions.to_vec(),
            phantom: std::marker::PhantomData,
        }
    }

    pub fn rebuild(&mut self, extent: [u32; 2]) {
        let (new_swapchain, new_images) = self
            .swapchain
            .recreate(SwapchainCreateInfo {
                image_extent: extent,
                ..self.swapchain.create_info()
            })
            .expect("failed to recreate swapchain");

        self.swapchain = new_swapchain;
        let (new_pipeline, new_framebuffers) = window_size_dependent_setup(
            self.memory_allocator.clone(),
            &new_images,
            self.render_pass.clone(),
            self.stages.clone(),
            &self.vertex_buffer_descriptions,
        );
        self.pipeline = new_pipeline;
        self.framebuffers = new_framebuffers;
    }

    pub fn render<Pc, VB>(&mut self, vertex_buffers: VB, push_data: Pc)
    where
        Pc: BufferContents,
        VB: IntoIterator<Item = Subbuffer<[T]>>,
    {
        // Do not draw frame when screen dimensions are zero.
        // On Windows, this can occur from minimizing the application.
        let window = self
            .surface
            .object()
            .unwrap()
            .downcast_ref::<Window>()
            .unwrap();
        let extent: [u32; 2] = window.inner_size().into();
        if extent[0] == 0 || extent[1] == 0 {
            return;
        }
        // free memory
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

        // Whenever the window resizes we need to recreate everything dependent on the window size.
        // In this example that includes the swapchain, the framebuffers and the dynamic state viewport.
        if self.wdd_needs_rebuild {
            self.rebuild(extent);
            self.wdd_needs_rebuild = false;
            println!("rebuilt swapchain");
        }

        // This operation returns the index of the image that we are allowed to draw upon.
        let (image_index, suboptimal, acquire_future) =
            match swapchain::acquire_next_image(self.swapchain.clone(), None)
                .map_err(Validated::unwrap)
            {
                Ok(r) => r,
                Err(VulkanError::OutOfDate) => {
                    println!("swapchain out of date (at acquire)");
                    self.wdd_needs_rebuild = true;
                    return;
                }
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        if suboptimal {
            self.wdd_needs_rebuild = true;
        }

        // In order to draw, we have to build a *command buffer*. The command buffer object holds
        // the list of commands that are going to be executed.
        //
        // Building a command buffer is an expensive operation (usually a few hundred
        // microseconds), but it is known to be a hot path in the driver and is expected to be
        // optimized.
        //
        // Note that we have to pass a queue family when we create the command buffer. The command
        // buffer will only be executable on that given queue family.
        let mut builder = AutoCommandBufferBuilder::primary(
            &self.command_buffer_allocator,
            self.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        // Finish building the command buffer by calling `build`.
        builder
            .begin_render_pass(
                RenderPassBeginInfo {
                    clear_values: vec![Some([0.53, 0.81, 0.92, 1.0].into()), Some(1f32.into())],
                    ..RenderPassBeginInfo::framebuffer(
                        self.framebuffers[image_index as usize].clone(),
                    )
                },
                Default::default(),
            )
            .unwrap()
            .bind_pipeline_graphics(self.pipeline.clone())
            .unwrap()
            .push_constants(self.pipeline.layout().clone(), 0, push_data)
            .unwrap();

        // for each vertex buffer, bind it and draw
        for vertex_buffer in vertex_buffers {
            let vertex_count = vertex_buffer.len() as u32;
            builder
                .bind_vertex_buffers(0, vertex_buffer)
                .unwrap()
                .draw(vertex_count, 1, 0, 0)
                .unwrap();
        }

        // We leave the render pass by calling `draw_end`. Note that if we had multiple
        // subpasses we could have called `next_inline` (or `next_secondary`) to jump to the
        // next subpass.
        builder.end_render_pass(Default::default()).unwrap();

        let command_buffer = builder.build().unwrap();

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            // The color output is now expected to contain our triangle. But in order to show it on
            // the screen, we have to *present* the image by calling `present`.
            //
            // This function does not actually present the image immediately. Instead it submits a
            // present command at the end of the queue. This means that it will only be presented once
            // the GPU has finished executing the command buffer that draws the triangle.
            .then_swapchain_present(
                self.queue.clone(),
                SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
            )
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(VulkanError::OutOfDate) => {
                self.wdd_needs_rebuild = true;
                println!("swapchain out of date (at flush)");
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
            }
        }
    }
}
