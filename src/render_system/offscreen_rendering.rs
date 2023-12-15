use std::sync::Arc;

use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    command_buffer::{
        allocator::StandardCommandBufferAllocator, AutoCommandBufferBuilder, CommandBufferUsage,
        CopyImageToBufferInfo, RenderPassBeginInfo,
    },
    device::{Device, DeviceOwned, Queue},
    format::Format,
    image::{
        view::ImageView, Image, ImageCreateInfo, ImageLayout, ImageTiling, ImageType, ImageUsage,
    },
    memory::allocator::{AllocationCreateInfo, MemoryTypeFilter, StandardMemoryAllocator},
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
    sync::{self, future::FenceSignalFuture, GpuFuture},
    Validated,
};

fn construct_offscreen_pipeline(
    memory_allocator: Arc<StandardMemoryAllocator>,
    image: Arc<Image>,
    render_pass: Arc<RenderPass>,
    stages: Vec<EntryPoint>,
    vertex_buffer_descriptions: &[VertexBufferDescription],
) -> (Arc<GraphicsPipeline>, Arc<Framebuffer>) {
    // validate stages
    assert!(stages.len() > 0, "no shader stages provided");
    assert!(
        stages[0].info().execution_model == ExecutionModel::Vertex,
        "first shader stage must be vertex shader"
    );

    let device = memory_allocator.device().clone();
    let extent = image.extent();

    let depth_buffer = ImageView::new_default(
        Image::new(
            memory_allocator,
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::D32_SFLOAT,
                extent,
                usage: ImageUsage::DEPTH_STENCIL_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                ..Default::default()
            },
            AllocationCreateInfo::default(),
        )
        .unwrap(),
    )
    .unwrap();

    let framebuffer = {
        let view = ImageView::new_default(image.clone()).unwrap();
        Framebuffer::new(
            render_pass.clone(),
            FramebufferCreateInfo {
                attachments: vec![view, depth_buffer.clone()],
                ..Default::default()
            },
        )
        .unwrap()
    };

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

    (pipeline, framebuffer)
}

pub struct Renderer<Vert> {
    extent: [u32; 2],
    stages: Vec<EntryPoint>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    image: Arc<Image>,
    framebuffer: Arc<Framebuffer>,
    staging_buffer: Subbuffer<[u8]>,
    vertex_buffer_descriptions: Vec<VertexBufferDescription>,
    previous_frame_end: Option<FenceSignalFuture<Box<dyn GpuFuture>>>,
    phantom: std::marker::PhantomData<Vert>,
}

impl<T> Renderer<T> {
    pub fn new(
        extent: [u32; 2],
        stages: Vec<EntryPoint>,
        queue: Arc<Queue>,
        memory_allocator: Arc<StandardMemoryAllocator>,
    ) -> Renderer<T>
    where
        T: Vertex,
    {
        // validate stages
        assert!(stages.len() > 0, "no shader stages provided");
        assert!(
            stages[0].info().execution_model == ExecutionModel::Vertex,
            "first shader stage must be vertex shader"
        );
        let device = memory_allocator.device().clone();

        // the image we render to
        let image = Image::new(
            memory_allocator.clone(),
            ImageCreateInfo {
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [extent[0], extent[1], 1],
                tiling: ImageTiling::Optimal,
                usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_SRC,
                ..ImageCreateInfo::default()
            },
            AllocationCreateInfo {
                ..AllocationCreateInfo::default()
            },
        )
        .unwrap();

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    format: image.format(),
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

        let vertex_buffer_descriptions = [T::per_vertex()];

        let (pipeline, framebuffer) = construct_offscreen_pipeline(
            memory_allocator.clone(),
            image.clone(),
            render_pass.clone(),
            stages.clone(),
            &vertex_buffer_descriptions,
        );

        let staging_buffer = Buffer::new_unsized(
            memory_allocator.clone(),
            BufferCreateInfo {
                usage: BufferUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo {
                memory_type_filter: MemoryTypeFilter::HOST_RANDOM_ACCESS,
                ..Default::default()
            },
            (extent[0] * extent[1] * 4) as u64,
        )
        .unwrap();

        Renderer {
            extent,
            stages,
            command_buffer_allocator: Arc::new(StandardCommandBufferAllocator::new(
                device.clone(),
                Default::default(),
            )),
            previous_frame_end: Some(sync::now(device.clone()).boxed().then_signal_fence()),
            device,
            queue,
            pipeline,
            image,
            framebuffer,
            staging_buffer,
            memory_allocator,
            render_pass,
            vertex_buffer_descriptions: vertex_buffer_descriptions.to_vec(),
            phantom: std::marker::PhantomData,
        }
    }

    pub fn render<Pc, VB>(&mut self, vertex_buffers: VB, push_data: Pc)
    where
        Pc: BufferContents,
        VB: IntoIterator<Item = Subbuffer<[T]>>,
    {
        // free memory
        self.previous_frame_end.as_mut().unwrap().cleanup_finished();

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
                    ..RenderPassBeginInfo::framebuffer(self.framebuffer.clone())
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

        // we now copy the results of the render to the staging buffer
        builder
            .copy_image_to_buffer(CopyImageToBufferInfo::image_buffer(
                self.image.clone(),
                self.staging_buffer.clone(),
            ))
            .unwrap();

        let command_buffer = builder.build().unwrap();

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .then_execute(self.queue.clone(), command_buffer)
            .unwrap()
            .boxed()
            .then_signal_fence_and_flush();

        match future.map_err(Validated::unwrap) {
            Ok(future) => {
                self.previous_frame_end = Some(future);
            }
            Err(e) => {
                println!("failed to flush future: {e}");
                self.previous_frame_end =
                    Some(sync::now(self.device.clone()).boxed().then_signal_fence());
            }
        }
    }

    fn get_image_data(&mut self) -> Vec<u8> {
        self.previous_frame_end
            .as_mut()
            .unwrap()
            .wait(None)
            .unwrap();
        self.staging_buffer.read().unwrap().to_vec()
    }
}
