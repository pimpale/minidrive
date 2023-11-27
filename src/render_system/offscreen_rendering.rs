use std::marker::PhantomData;

pub struct Renderer<T> {
    phantom: PhantomData<T>,
}