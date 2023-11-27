use std::{collections::HashMap, sync::Arc};

use vulkano::{
    buffer::{Buffer, BufferContents, BufferCreateInfo, BufferUsage, Subbuffer},
    memory::allocator::{AllocationCreateInfo, MemoryAllocator, MemoryTypeFilter},
};

pub struct Scene<K, Vertex> {
    objects: HashMap<K, Vec<Vertex>>,
    memory_allocator: Arc<dyn MemoryAllocator>,
    vertex_buffer: Option<Subbuffer<[Vertex]>>,
    vertex_buffer_needs_update: bool,
}

#[allow(dead_code)]
impl<K, Vertex> Scene<K, Vertex>
where
    Vertex: Clone + BufferContents,
    K: std::cmp::Eq + std::hash::Hash,
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

    pub fn add_object(&mut self, key: K, object: Vec<Vertex>) {
        self.objects.insert(key, object);
        self.vertex_buffer_needs_update = true;
    }

    pub fn remove_object(&mut self, key: K) {
        let removed = self.objects.remove(&key);
        if removed.is_some() {
            self.vertex_buffer_needs_update = true;
        }
    }

    pub fn objects(&self) -> &HashMap<K, Vec<Vertex>> {
        &self.objects
    }

    pub fn vertex_buffer(&mut self) -> Option<Subbuffer<[Vertex]>> {
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
) -> Option<Subbuffer<[Vertex]>>
where
    Container: IntoIterator<Item = &'a Vec<Vertex>>,
    Vertex: Clone + BufferContents,
{
    let vertexes = objects
        .into_iter()
        .flat_map(|o| o.iter())
        .cloned()
        .collect::<Vec<Vertex>>();
    if vertexes.len() == 0 {
        return None;
    } else {
        let buffer = Buffer::from_iter(
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
            vertexes,
        )
        .unwrap();

        return Some(buffer);
    }
}
