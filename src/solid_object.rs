#[allow(unused_imports)]
use std::{
    fmt,
    cell::RefCell,
    sync::Arc
};

use parking_lot::RwLock;

use vulkano::{
    buffer::Subbuffer,
    pipeline::graphics::vertex_input::{VertexBufferDescription, Vertex}
};

use nalgebra::{Vector3, Vector4, Matrix4};

use crate::{
    game_object::*,
    SimpleVertex,
    object::{impl_updated_check, NormalGraphicalObject, ObjectTransform, Model},
    allocators::ObjectAllocator,
    transform::{Transform, OnTransformCallback, TransformContainer}
};


pub struct SolidObject
{
    model: Arc<RwLock<Model>>,
    transform: ObjectTransform,
    subbuffer: Subbuffer<[SimpleVertex]>,
    #[cfg(debug_assertions)]
    updated_buffers: bool
}

impl NormalGraphicalObject<SimpleVertex> for SolidObject
{
    fn subbuffer(&self) -> Subbuffer<[SimpleVertex]>
    {
        self.subbuffer.clone()
    }

    fn vertices(&self, projection_view: Matrix4<f32>) -> Box<[SimpleVertex]>
    {
        self.calculate_vertices(projection_view)
    }

    impl_updated_check!{}
}

#[allow(dead_code)]
impl SolidObject
{
    pub fn new_default(
        model: Arc<RwLock<Model>>,
        allocator: &ObjectAllocator
    ) -> Self
    {
        let transform = ObjectTransform::new_default();

        Self::new(model, transform, allocator)
    }

    pub fn new(
        model: Arc<RwLock<Model>>,
        transform: ObjectTransform,
        allocator: &ObjectAllocator
    ) -> Self
    {
        let subbuffer = allocator.subbuffer(model.read().vertices.len() as u64);

        Self{
            model,
            transform,
            subbuffer,
            #[cfg(debug_assertions)]
            updated_buffers: false
        }
    }

    fn calculate_vertices(&self, projection_view: Matrix4<f32>) -> Box<[SimpleVertex]>
    {
        let transform = self.transform.matrix();

        let model = self.model.read();

        model.vertices.iter().map(move |vertex|
        {
            let vertex = Vector4::new(vertex[0], vertex[1], vertex[2], 1.0);

            let vertex = projection_view * transform * vertex;

            SimpleVertex{position: vertex.into()}
        }).collect::<Box<[_]>>()
    }

    pub fn set_origin(&mut self, origin: Vector3<f32>)
    {
        self.transform.set_origin(origin);
    }

    fn needs_draw(&self) -> bool
    {
        !self.model.read().vertices.is_empty()
    }

    pub fn per_vertex() -> VertexBufferDescription
    {
        SimpleVertex::per_vertex()
    }
}

impl GameObject for SolidObject
{
    fn update_buffers(&mut self, info: &mut UpdateBuffersInfo)
    {
        self.normal_update_buffers(info);
    }

    fn draw(&self, info: &mut DrawInfo)
    {
        if !self.needs_draw()
        {
            return;
        }

        self.assert_updated(&info.object_info);

        let size = self.model.read().vertices.len() as u32;

        info.object_info.builder_wrapper.builder()
            .bind_vertex_buffers(0, self.subbuffer.clone())
            .unwrap()
            .draw(size, 1, 0, 0)
            .unwrap();
    }
}

impl OnTransformCallback for SolidObject
{
    fn callback(&mut self)
    {
        self.transform.callback();
    }
}

impl TransformContainer for SolidObject
{
    fn transform_ref(&self) -> &Transform
    {
        self.transform.transform_ref()
    }

    fn transform_mut(&mut self) -> &mut Transform
    {
        self.transform.transform_mut()
    }
}

impl fmt::Debug for SolidObject
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("Object")
            .field("model", &self.model)
            .field("transform", &self.transform)
            .finish()
    }
}
