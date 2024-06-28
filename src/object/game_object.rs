use std::{
    rc::Rc,
    sync::Arc
};

use nalgebra::Matrix4;

use parking_lot::Mutex;

use vulkano::{
    pipeline::{PipelineBindPoint, PipelineLayout, graphics::viewport::Scissor},
    descriptor_set::WriteDescriptorSet,
    buffer::{
        Subbuffer,
        BufferContents
    },
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer},
};

use crate::{
    Assets,
    ObjectFactory,
    UniformLocation,
    ShaderId,
    PipelineInfo,
    allocators::UniformAllocator,
    camera::Camera
};

pub use builder_wrapper::BuilderWrapper;

mod builder_wrapper;


pub type CommandBuilderType = AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>;

pub struct ObjectCreatePartialInfo<'a>
{
    pub builder_wrapper: BuilderWrapper<'a>,
    pub assets: Arc<Mutex<Assets>>,
    pub object_factory: Rc<ObjectFactory>,
    pub uniform_allocator: Rc<UniformAllocator>,
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
    current_pipeline: Option<usize>,
    pipelines: &'a [PipelineInfo]
}

impl<'a> DrawInfo<'a>
{
    pub fn new(
        object_info: ObjectCreatePartialInfo<'a>,
        pipelines: &'a [PipelineInfo]
    ) -> Self
    {
        Self{
            object_info,
            current_pipeline: None,
            pipelines
        }
    }

    pub fn bind_pipeline(&mut self, shader: ShaderId)
    {
        self.current_pipeline = Some(shader.get_raw());

        let pipeline = self.current_pipeline().pipeline.clone();
        self.object_info.builder_wrapper.builder().bind_pipeline_graphics(
            pipeline
        ).unwrap();
    }

    pub fn current_pipeline_id(&self) -> Option<ShaderId>
    {
        self.current_pipeline.map(ShaderId)
    }

    pub fn current_pipeline(&self) -> &PipelineInfo
    {
        &self.pipelines[self.current_pipeline.expect("pipeline must be bound")]
    }

    pub fn current_layout(&self) -> Arc<PipelineLayout>
    {
        self.current_pipeline().layout.clone()
    }

    #[allow(dead_code)]
    pub fn push_constants<T: BufferContents>(
        &mut self,
        constants: T
    )
    {
        let layout = self.current_layout();
        self.object_info.builder_wrapper.builder().push_constants(
                layout,
                0,
                constants
            )
            .unwrap();
    }

    #[allow(dead_code)]
    pub fn push_uniform_buffer<T: BufferContents>(
        &mut self,
        location: UniformLocation,
        buffer: Subbuffer<T>
    )
    {
        let layout = self.current_layout();
        self.object_info.builder_wrapper.builder().push_descriptor_set(
                PipelineBindPoint::Graphics,
                layout,
                location.set,
                vec![WriteDescriptorSet::buffer(location.binding, buffer)].into()
            )
            .unwrap();
    }

    pub fn set_depth_write(&mut self, state: bool)
    {
        self.object_info.builder_wrapper.builder()
            .set_depth_write_enable(state)
            .unwrap();
    }

    pub fn set_scissor(&mut self, scissor: Scissor)
    {
        self.object_info.builder_wrapper.builder()
            .set_scissor(0, vec![scissor].into())
            .unwrap();
    }

    pub fn reset_scissor(&mut self)
    {
        self.object_info.builder_wrapper.builder()
            .set_scissor(0, vec![Scissor::default()].into())
            .unwrap();
    }
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
