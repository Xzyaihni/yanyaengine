#[allow(unused_imports)]
use std::{
    fmt,
    cell::RefCell,
    sync::Arc
};

use parking_lot::RwLock;

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
        fn set_updated(&mut self, object_info: &crate::object::game_object::ObjectCreatePartialInfo)
        {
            #[cfg(debug_assertions)]
            {
                self.updated_buffers = object_info.frame_parity;
            }
        }

        #[allow(unused_variables)]
        fn assert_updated(&self, object_info: &crate::object::game_object::ObjectCreatePartialInfo)
        {
            #[cfg(debug_assertions)]
            {
                assert!(
                    self.updated_buffers == object_info.frame_parity,
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

#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
struct ObjectVertex
{
    #[format(R32G32B32A32_SFLOAT)]
    pub position: [f32; 4],

    #[format(R32G32_SFLOAT)]
    pub uv: [f32; 2]
}

pub struct Object
{
    model: Arc<RwLock<Model>>,
    texture: Arc<RwLock<Texture>>,
    transform: ObjectTransform,
    subbuffer: Subbuffer<[ObjectVertex]>,
    #[cfg(debug_assertions)]
    updated_buffers: bool
}

#[allow(dead_code)]
impl Object
{
    pub fn new_default(
        model: Arc<RwLock<Model>>,
        texture: Arc<RwLock<Texture>>,
        allocator: &ObjectAllocator
    ) -> Self
    {
        let transform = ObjectTransform::new_default();

        Self::new(model, texture, transform, allocator)
    }

    pub fn new(
        model: Arc<RwLock<Model>>,
        texture: Arc<RwLock<Texture>>,
        transform: ObjectTransform,
        allocator: &ObjectAllocator
    ) -> Self
    {
        let subbuffer = allocator.subbuffer(model.read().vertices.len() as u64);

        Self{
            model,
            texture,
            transform,
            subbuffer,
            #[cfg(debug_assertions)]
            updated_buffers: false
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

            ObjectVertex{position: vertex.into(), uv: *uv}
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

        *current_model = model;
    }

    pub fn set_texture(&mut self, texture: Arc<RwLock<Texture>>)
    {
        self.texture = texture;
    }

    pub fn set_inplace_texture(&mut self, texture: Texture)
    {
        *self.texture.write() = texture;
    }

    pub fn texture(&self) -> &Arc<RwLock<Texture>>
    {
        &self.texture
    }

    fn needs_draw(&self) -> bool
    {
        !self.model.read().vertices.is_empty()
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

        self.assert_updated(&info.object_info);

        let size = self.model.read().vertices.len() as u32;

        let layout = info.current_layout();

        unsafe{
            info.object_info.builder_wrapper.builder()
                .bind_descriptor_sets(
                    PipelineBindPoint::Graphics,
                    layout,
                    0,
                    self.texture.read().descriptor_set()
                )
                .unwrap()
                .bind_vertex_buffers(0, self.subbuffer.clone())
                .unwrap()
                .draw(size, 1, 0, 0)
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
