#[allow(unused_imports)]
use std::{
    fmt,
    cell::RefCell,
    sync::Arc
};

use parking_lot::RwLock;

use vulkano::{
    buffer::Subbuffer,
    pipeline::{PipelineBindPoint, graphics::vertex_input::Vertex}
};

use nalgebra::{Vector3, Vector4, Matrix4};

use crate::{
    game_object::*,
    SimpleVertex,
    object::{impl_updated_check, NormalGraphicalObject, ObjectTransform, Model},
    allocators::ObjectAllocator,
    transform::{Transform, OnTransformCallback, TransformContainer}
};


pub struct SolidObject<VertexType=SimpleVertex>
{
    model: Arc<RwLock<Model>>,
    transform: ObjectTransform,
    subbuffer: Subbuffer<[VertexType]>,
    indices: Subbuffer<[u16]>,
    #[cfg(debug_assertions)]
    updated_buffers: Option<bool>
}

impl<VertexType: Vertex + From<([f32; 4], [f32; 2])> + fmt::Debug> NormalGraphicalObject<VertexType> for SolidObject<VertexType>
{
    fn subbuffer(&self) -> Subbuffer<[VertexType]>
    {
        self.subbuffer.clone()
    }

    fn vertices(&self, projection_view: Matrix4<f32>) -> Box<[VertexType]>
    {
        self.calculate_vertices(projection_view)
    }

    impl_updated_check!{}
}

#[allow(dead_code)]
impl<VertexType: Vertex + From<([f32; 4], [f32; 2])>> SolidObject<VertexType>
{
    pub fn new(
        model: Arc<RwLock<Model>>,
        transform: ObjectTransform,
        vertex_allocator: &ObjectAllocator,
        index_allocator: &ObjectAllocator
    ) -> Self
    {
        let subbuffer = vertex_allocator.subbuffer(model.read().vertices.len() as u64);

        let indices = {
            let model_indices = &model.read().indices;

            let indices = index_allocator.subbuffer(model_indices.len() as u64);
            indices.write().unwrap().copy_from_slice(model_indices.as_slice());

            indices
        };

        Self{
            model,
            transform,
            subbuffer,
            indices,
            #[cfg(debug_assertions)]
            updated_buffers: None
        }
    }

    fn calculate_vertices(&self, projection_view: Matrix4<f32>) -> Box<[VertexType]>
    {
        let transform = self.transform.matrix();

        let model = self.model.read();

        model.vertices.iter().zip(model.uvs.iter()).map(move |(vertex, uv)|
        {
            let vertex = Vector4::new(vertex[0], vertex[1], vertex[2], 1.0);

            let vertex = projection_view * transform * vertex;

            VertexType::from((vertex.into(), *uv))
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
}

impl<VertexType: Vertex + From<([f32; 4], [f32; 2])> + fmt::Debug> GameObject for SolidObject<VertexType>
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

        let size = self.model.read().indices.len() as u32;

        let layout = info.current_layout();

        unsafe{
            info.object_info.builder_wrapper.builder()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout,
                    0,
                    info.current_sets.clone()
                )
                .unwrap()
                .bind_index_buffer(self.indices.clone())
                .unwrap()
                .bind_vertex_buffers(0, self.subbuffer.clone())
                .unwrap()
                .draw_indexed(size, 1, 0, 0, 0)
                .unwrap();
        }
    }
}

impl<VertexType> OnTransformCallback for SolidObject<VertexType>
{
    fn callback(&mut self)
    {
        self.transform.callback();
    }
}

impl<VertexType> TransformContainer for SolidObject<VertexType>
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

impl<VertexType: fmt::Debug> fmt::Debug for SolidObject<VertexType>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("Object")
            .field("model", &self.model)
            .field("transform", &self.transform)
            .finish()
    }
}
