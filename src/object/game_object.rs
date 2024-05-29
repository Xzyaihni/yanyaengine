use std::{
    rc::Rc,
    sync::Arc
};

use nalgebra::Matrix4;

use parking_lot::Mutex;

use vulkano::{
    pipeline::PipelineLayout,
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
};

use crate::{
    Assets,
    ObjectFactory,
    camera::Camera
};

pub use builder_wrapper::BuilderWrapper;

mod builder_wrapper;


pub type CommandBuilderType = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;
type LayoutType = Arc<PipelineLayout>;

pub struct ObjectCreatePartialInfo<'a>
{
    pub builder_wrapper: BuilderWrapper<'a>,
    pub assets: Arc<Mutex<Assets>>,
    pub object_factory: Rc<ObjectFactory>,
    pub size: [f32; 2],
    pub image_index: usize
}

impl ObjectCreatePartialInfo<'_>
{
    pub fn aspect(&self) -> f32
    {
        let [x, y] = self.size;

        x / y
    }
}

pub struct ObjectCreateInfo<'a>
{
    pub partial: ObjectCreatePartialInfo<'a>,
    pub projection_view: Matrix4<f32>
}

impl<'a> ObjectCreateInfo<'a>
{
    pub fn new(partial_info: ObjectCreatePartialInfo<'a>, camera: &Camera) -> Self
    {
        let projection_view = camera.projection_view();

        Self{partial: partial_info, projection_view}
    }
}

pub type InitPartialInfo<'a> = ObjectCreatePartialInfo<'a>;

pub struct InitInfo<'a>
{
    pub object_info: ObjectCreateInfo<'a>
}

impl<'a> InitInfo<'a>
{
    pub fn new(partial_info: InitPartialInfo<'a>, camera: &Camera) -> Self
    {
        Self{
            object_info: ObjectCreateInfo::new(partial_info, camera)
        }
    }
}

pub struct DrawInfo<'a>
{
    pub object_info: ObjectCreatePartialInfo<'a>,
    pub layout: LayoutType
}

pub type UpdateBuffersPartialInfo<'a> = ObjectCreatePartialInfo<'a>;

pub struct UpdateBuffersInfo<'a>
{
    pub object_info: ObjectCreateInfo<'a>
}

impl<'a> UpdateBuffersInfo<'a>
{
    pub fn new(partial_info: UpdateBuffersPartialInfo<'a>, camera: &Camera) -> Self
    {
        let object_info = ObjectCreateInfo::new(partial_info, camera);

        Self{object_info}
    }
}

pub trait GameObject
{
	fn update_buffers(&mut self, info: &mut UpdateBuffersInfo);
	fn draw(&self, info: &mut DrawInfo);
}
