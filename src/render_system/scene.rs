use std::{collections::HashMap, sync::Arc};

use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
};

pub struct Scene<K, Vertex> {
    objects: HashMap<K, Vec<Vertex>>,
    memory_allocator: Arc<dyn MemoryAllocator>,
    vertex_buffer: Subbuffer<[Vertex]>,
    vertex_buffer_needs_update: bool,
}

impl<K, Vertex> Scene<K, Vertex>
where
    Vertex: Clone + BufferContents,
{
    pub fn new(
        memory_allocator: Arc<dyn MemoryAllocator>,
        objects: HashMap<K, Vec<Vertex>>,
    ) -> Scene<K, Vertex> {
        Scene {
            vertex_buffer: vertex_buffer(memory_allocator.clone(), objects.values()),
            objects,
            memory_allocator,
            vertex_buffer_needs_update: false,
        }
    }

    pub fn vertex_buffers(&mut self) -> Subbuffer<[Vertex]> {
        if self.vertex_buffer_needs_update {
            self.vertex_buffer =
                vertex_buffer(self.memory_allocator.clone(), self.objects.values());
            self.vertex_buffer_needs_update = false;
        }
        return self.vertex_buffer.clone();
    }
}

fn vertex_buffer<'a, Vertex, Container>(
    memory_allocator: Arc<dyn MemoryAllocator>,
    objects: Container,
) -> Subbuffer<[Vertex]>
where
    Container: IntoIterator<Item = &'a Vec<Vertex>>,
    Vertex: Clone + BufferContents,
{
    Buffer::from_iter(
        memory_allocator,
        BufferCreateInfo {
            usage: BufferUsage::VERTEX_BUFFER,
            ..Default::default()
        },
        AllocationCreateInfo {
            memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
            ..Default::default()
        },
        objects
            .into_iter()
            .flat_map(|o| o.iter())
            .cloned()
            .collect::<Vec<Vertex>>(),
    )
    .unwrap()
}
