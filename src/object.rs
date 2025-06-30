#[allow(unused_imports)]
use std::{
    fmt,
    ops::DerefMut,
    cell::RefCell,
    sync::Arc
};

use parking_lot::{RwLock, Mutex};

use vulkano::{
    buffer::{BufferContents, Subbuffer},
    pipeline::{
        PipelineBindPoint,
        graphics::vertex_input::{VertexBufferDescription, Vertex}
    }
};

use nalgebra::{Vector3, Vector4, Matrix4};

use crate::{
    allocators::ObjectAllocator,
    transform::{Transform, OnTransformCallback, TransformContainer}
};

pub use crate::impl_updated_check;

pub use object_transform::ObjectTransform;

use game_object::*;
pub use model::Model;
pub use texture::Texture;

mod object_transform;

pub mod game_object;
pub mod resource_uploader;
pub mod model;
pub mod texture;


pub trait NormalGraphicalObject<T: BufferContents>
{
    fn subbuffer(&self) -> Subbuffer<[T]>;
    fn vertices(&self, projection_view: Matrix4<f32>) -> Box<[T]>;

    fn set_updated(&mut self, object_info: &ObjectCreatePartialInfo);
    fn assert_updated(&self, object_info: &ObjectCreatePartialInfo);

    fn normal_update_buffers(&mut self, info: &mut UpdateBuffersInfo)
    {
        let vertices = self.vertices(info.projection_view);
        if vertices.is_empty()
        {
            return;
        }

        self.set_updated(&info.partial);

        info.partial.builder_wrapper.builder()
            .update_buffer(
                self.subbuffer(),
                vertices
            ).unwrap();
    }
}

#[macro_export]
macro_rules! impl_updated_check
{
    () =>
    {
        #[allow(unused_variables)]
        fn set_updated(&mut self, object_info: &$crate::object::game_object::ObjectCreatePartialInfo)
        {
            #[cfg(debug_assertions)]
            {
                self.updated_buffers = Some(object_info.frame_parity);
            }
        }

        #[allow(unused_variables)]
        fn assert_updated(&self, object_info: &$crate::object::game_object::ObjectCreatePartialInfo)
        {
            #[cfg(debug_assertions)]
            {
                assert!(
                    self.updated_buffers == Some(object_info.frame_parity),
                    "update_buffers wasnt called on {self:#?}"
                );
            }
        }
    }
}

impl NormalGraphicalObject<ObjectVertex> for Object
{
    fn subbuffer(&self) -> Subbuffer<[ObjectVertex]>
    {
        self.subbuffer.clone()
    }

    fn vertices(&self, projection_view: Matrix4<f32>) -> Box<[ObjectVertex]>
    {
        self.calculate_vertices(projection_view)
    }

    impl_updated_check!{}
}

#[derive(BufferContents, Vertex, Debug, Clone, Copy)]
#[repr(C)]
pub struct ObjectVertex
{
    #[format(R32G32B32_SFLOAT)]
    pub position: [f32; 3],

    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2]
}

impl From<([f32; 4], [f32; 2])> for ObjectVertex
{
    fn from(([x, y, z, _w], uv): ([f32; 4], [f32; 2])) -> Self
    {
        Self{position: [x, y, z], uv}
    }
}

pub struct Object
{
    model: Arc<RwLock<Model>>,
    texture: Arc<Mutex<Texture>>,
    transform: ObjectTransform,
    subbuffer: Subbuffer<[ObjectVertex]>,
    indices: Subbuffer<[u16]>,
    #[cfg(debug_assertions)]
    updated_buffers: Option<bool>
}

#[allow(dead_code)]
impl Object
{
    pub fn new(
        model: Arc<RwLock<Model>>,
        texture: Arc<Mutex<Texture>>,
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
            texture,
            transform,
            subbuffer,
            indices,
            #[cfg(debug_assertions)]
            updated_buffers: None
        }
    }

    fn calculate_vertices(&self, projection_view: Matrix4<f32>) -> Box<[ObjectVertex]>
    {
        let transform = self.transform.matrix();

        let model = self.model.read();

        model.vertices.iter().zip(model.uvs.iter()).map(move |(vertex, uv)|
        {
            let vertex = Vector4::new(vertex[0], vertex[1], vertex[2], 1.0);

            let vertex = projection_view * transform * vertex;

            ObjectVertex{position: vertex.xyz().into(), uv: *uv}
        }).collect::<Box<[_]>>()
    }

    pub fn set_origin(&mut self, origin: Vector3<f32>)
    {
        self.transform.set_origin(origin);
    }

    pub fn set_inplace_model_same_sized(&mut self, model: Model)
    {
        let mut current_model = self.model.write();
        assert_eq!(current_model.vertices.len(), model.vertices.len());
        assert_eq!(current_model.indices.len(), model.indices.len());

        *current_model = model;
    }

    pub fn set_texture(&mut self, texture: Arc<Mutex<Texture>>)
    {
        self.texture = texture;
    }

    pub fn set_inplace_texture(&mut self, texture: Texture)
    {
        *self.texture.lock() = texture;
    }

    pub fn texture(&self) -> &Arc<Mutex<Texture>>
    {
        &self.texture
    }

    fn needs_draw(&self) -> bool
    {
        !self.model.read().indices.is_empty()
    }

    pub fn per_vertex() -> VertexBufferDescription
    {
        ObjectVertex::per_vertex()
    }
}

impl GameObject for Object
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

        let descriptor_set = self.texture.lock().descriptor_set(info);

        self.assert_updated(&info.object_info);

        let size = self.model.read().indices.len() as u32;

        let layout = info.current_layout();

        let mut sets = info.current_sets.clone();
        sets.push(descriptor_set);

        unsafe{
            info.object_info.builder_wrapper.builder()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout,
                    0,
                    sets
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

impl OnTransformCallback for Object
{
    fn callback(&mut self)
    {
        self.transform.callback();
    }
}

impl TransformContainer for Object
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

impl fmt::Debug for Object
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("Object")
            .field("model", &self.model)
            .field("texture", &self.texture)
            .field("transform", &self.transform)
            .finish()
    }
}
