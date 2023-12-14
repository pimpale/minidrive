use std::{marker::PhantomData, sync::Arc};

use vulkano::{
    command_buffer::allocator::StandardCommandBufferAllocator,
    device::{Device, Queue},
    memory::allocator::StandardMemoryAllocator,
    pipeline::{graphics::vertex_input::{VertexBufferDescription, Vertex}, GraphicsPipeline},
    render_pass::{Framebuffer, RenderPass},
    shader::EntryPoint,
    sync::GpuFuture, format::Format, buffer::{BufferContents, Subbuffer},
};

pub struct Renderer<Vert> {
    stages: Vec<EntryPoint>,
    device: Arc<Device>,
    queue: Arc<Queue>,
    memory_allocator: Arc<StandardMemoryAllocator>,
    command_buffer_allocator: Arc<StandardCommandBufferAllocator>,
    render_pass: Arc<RenderPass>,
    pipeline: Arc<GraphicsPipeline>,
    framebuffers: Vec<Arc<Framebuffer>>,
    vertex_buffer_descriptions: Vec<VertexBufferDescription>,
    wdd_needs_rebuild: bool,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
    phantom: std::marker::PhantomData<Vert>,
}

// impl<T> Renderer<T> {
//     pub fn new(
//         stages: Vec<EntryPoint>,
//         queue: Arc<Queue>,
//         memory_allocator: Arc<StandardMemoryAllocator>,
//     ) -> Renderer<T>
//     where
//         T: Vertex,
//     {
//         let device = memory_allocator.device().clone();

//         let (swapchain, images) = create_swapchain(device.clone(), surface.clone());

//         let vertex_buffer_descriptions = [T::per_vertex()];

//         let render_pass = vulkano::single_pass_renderpass!(
//             device.clone(),
//             attachments: {
//                 color: {
//                     format: swapchain.image_format(),
//                     samples: 1,
//                     load_op: Clear,
//                     store_op: Store,
//                 },
//                 depth_stencil: {
//                     format: Format::D32_SFLOAT,
//                     samples: 1,
//                     load_op: Clear,
//                     store_op: DontCare,
//                 },
//             },
//             pass: {
//                 color: [color],
//                 depth_stencil: {depth_stencil},
//             },
//         )
//         .unwrap();

//         let (pipeline, framebuffers) = window_size_dependent_setup(
//             memory_allocator.clone(),
//             &images,
//             render_pass.clone(),
//             stages.clone(),
//             &vertex_buffer_descriptions,
//         );

//         Renderer {
//             stages,
//             surface,
//             command_buffer_allocator: Arc::new(StandardCommandBufferAllocator::new(
//                 device.clone(),
//                 Default::default(),
//             )),
//             previous_frame_end: Some(sync::now(device.clone()).boxed()),
//             device,
//             queue,
//             swapchain,
//             pipeline,
//             framebuffers,
//             memory_allocator,
//             render_pass,
//             wdd_needs_rebuild: false,
//             vertex_buffer_descriptions: vertex_buffer_descriptions.to_vec(),
//             phantom: std::marker::PhantomData,
//         }
//     }

//     pub fn render<Pc, VB>(&mut self, vertex_buffers: VB, push_data: Pc)
//     where
//         Pc: BufferContents,
//         VB: IntoIterator<Item = Subbuffer<[T]>>,
//     {
//         // Do not draw frame when screen dimensions are zero.
//         // On Windows, this can occur from minimizing the application.
//         let extent = get_surface_extent(&self.surface);
//         if extent[0] == 0 || extent[1] == 0 {
//             return;
//         }
//         // free memory
//         self.previous_frame_end.as_mut().unwrap().cleanup_finished();

//         // Whenever the window resizes we need to recreate everything dependent on the window size.
//         // In this example that includes the swapchain, the framebuffers and the dynamic state viewport.
//         if self.wdd_needs_rebuild {
//             self.rebuild(extent);
//             self.wdd_needs_rebuild = false;
//             println!("rebuilt swapchain");
//         }

//         // This operation returns the index of the image that we are allowed to draw upon.
//         let (image_index, suboptimal, acquire_future) =
//             match swapchain::acquire_next_image(self.swapchain.clone(), None)
//                 .map_err(Validated::unwrap)
//             {
//                 Ok(r) => r,
//                 Err(VulkanError::OutOfDate) => {
//                     println!("swapchain out of date (at acquire)");
//                     self.wdd_needs_rebuild = true;
//                     return;
//                 }
//                 Err(e) => panic!("Failed to acquire next image: {:?}", e),
//             };

//         if suboptimal {
//             self.wdd_needs_rebuild = true;
//         }

//         // In order to draw, we have to build a *command buffer*. The command buffer object holds
//         // the list of commands that are going to be executed.
//         //
//         // Building a command buffer is an expensive operation (usually a few hundred
//         // microseconds), but it is known to be a hot path in the driver and is expected to be
//         // optimized.
//         //
//         // Note that we have to pass a queue family when we create the command buffer. The command
//         // buffer will only be executable on that given queue family.
//         let mut builder = AutoCommandBufferBuilder::primary(
//             &self.command_buffer_allocator,
//             self.queue.queue_family_index(),
//             CommandBufferUsage::OneTimeSubmit,
//         )
//         .unwrap();

//         // Finish building the command buffer by calling `build`.
//         builder
//             .begin_render_pass(
//                 RenderPassBeginInfo {
//                     clear_values: vec![Some([0.53, 0.81, 0.92, 1.0].into()), Some(1f32.into())],
//                     ..RenderPassBeginInfo::framebuffer(
//                         self.framebuffers[image_index as usize].clone(),
//                     )
//                 },
//                 Default::default(),
//             )
//             .unwrap()
//             .bind_pipeline_graphics(self.pipeline.clone())
//             .unwrap()
//             .push_constants(self.pipeline.layout().clone(), 0, push_data)
//             .unwrap();

//         // for each vertex buffer, bind it and draw
//         for vertex_buffer in vertex_buffers {
//             let vertex_count = vertex_buffer.len() as u32;
//             builder
//                 .bind_vertex_buffers(0, vertex_buffer)
//                 .unwrap()
//                 .draw(vertex_count, 1, 0, 0)
//                 .unwrap();
//         }

//         // We leave the render pass by calling `draw_end`. Note that if we had multiple
//         // subpasses we could have called `next_inline` (or `next_secondary`) to jump to the
//         // next subpass.
//         builder.end_render_pass(Default::default()).unwrap();

//         let command_buffer = builder.build().unwrap();

//         let future = self
//             .previous_frame_end
//             .take()
//             .unwrap()
//             .join(acquire_future)
//             .then_execute(self.queue.clone(), command_buffer)
//             .unwrap()
//             // The color output is now expected to contain our triangle. But in order to show it on
//             // the screen, we have to *present* the image by calling `present`.
//             //
//             // This function does not actually present the image immediately. Instead it submits a
//             // present command at the end of the queue. This means that it will only be presented once
//             // the GPU has finished executing the command buffer that draws the triangle.
//             .then_swapchain_present(
//                 self.queue.clone(),
//                 SwapchainPresentInfo::swapchain_image_index(self.swapchain.clone(), image_index),
//             )
//             .then_signal_fence_and_flush();

//         match future.map_err(Validated::unwrap) {
//             Ok(future) => {
//                 self.previous_frame_end = Some(future.boxed());
//             }
//             Err(VulkanError::OutOfDate) => {
//                 self.wdd_needs_rebuild = true;
//                 println!("swapchain out of date (at flush)");
//                 self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
//             }
//             Err(e) => {
//                 println!("failed to flush future: {e}");
//                 self.previous_frame_end = Some(sync::now(self.device.clone()).boxed());
//             }
//         }
//     }
// }
