use std::{
    fmt,
    sync::Arc
};

use parking_lot::RwLock;

use vulkano::{
    buffer::{BufferContents, Subbuffer},
    pipeline::{
        PipelineBindPoint,
        graphics::vertex_input::Vertex
    }
};

use nalgebra::{Vector3, Vector4, Matrix4};

use crate::{
    allocators::ObjectAllocator,
    transform::{Transform, OnTransformCallback, TransformContainer}
};

pub use object_transform::ObjectTransform;

use game_object::*;
pub use model::Model;
pub use texture::Texture;

mod object_transform;

pub mod game_object;
pub mod resource_uploader;
pub mod model;
pub mod texture;


#[derive(BufferContents, Vertex, Clone, Copy)]
#[repr(C)]
pub struct ObjectVertex
{
    #[format(R32G32B32_SFLOAT)]
    position: [f32; 3],

    #[format(R32G32_SFLOAT)]
    uv: [f32; 2]
}

pub trait DrawableEntity
{
    fn texture(&self) -> &str;
}

pub struct Object
{
    model: Arc<RwLock<Model>>,
    texture: Arc<RwLock<Texture>>,
    transform: ObjectTransform,
    subbuffers: Box<[Subbuffer<[ObjectVertex]>]>
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
        let subbuffers = allocator.subbuffers(&model.read());

        let mut this = Self{
            model,
            texture,
            transform,
            subbuffers
        };

        (0..allocator.subbuffers_amount()).for_each(|index|
        {
            this.create_buffer(index);
        });

        this
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

    fn create_buffer(&mut self, index: usize)
    {
        let vertices = self.calculate_vertices(Matrix4::zeros());

        self.subbuffers[index].write().unwrap().copy_from_slice(&vertices);
    }

    pub fn set_origin(&mut self, origin: Vector3<f32>)
    {
        self.transform.set_origin(origin);
    }

    pub fn set_inplace_model(&mut self, model: Model)
    {
        *self.model.write() = model;
    }

    pub fn set_texture(&mut self, texture: Arc<RwLock<Texture>>)
    {
        self.texture = texture;
    }

    pub fn set_inplace_texture(&mut self, texture: Texture)
    {
        *self.texture.write() = texture;
    }

    fn needs_draw(&self) -> bool
    {
        !self.model.read().vertices.is_empty()
    }
}

impl GameObject for Object
{
    fn update_buffers(&mut self, info: &mut UpdateBuffersInfo)
    {
        if !self.needs_draw()
        {
            return;
        }

        info.object_info.partial.builder_wrapper.builder()
            .update_buffer(
                self.subbuffers[info.object_info.partial.image_index].clone(),
                self.calculate_vertices(info.object_info.projection_view)
            ).unwrap();
    }

    fn draw(&self, info: &mut DrawInfo)
    {
        if !self.needs_draw()
        {
            return;
        }

        let size = self.model.read().vertices.len() as u32;

        info.object_info.builder_wrapper.builder()
            .bind_descriptor_sets(
                PipelineBindPoint::Graphics,
                info.layout.clone(),
                0,
                self.texture.read().descriptor_set()
            )
            .unwrap()
            .bind_vertex_buffers(0, self.subbuffers[info.object_info.image_index].clone())
            .unwrap()
            .draw(size, 1, 0, 0)
            .unwrap();
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
