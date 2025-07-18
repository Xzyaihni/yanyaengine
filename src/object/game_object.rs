use std::{
    rc::Rc,
    sync::Arc
};

use nalgebra::Matrix4;

use parking_lot::Mutex;

use vulkano::{
    image::view::ImageView,
    pipeline::{PipelineBindPoint, PipelineLayout, graphics::viewport::Scissor},
    descriptor_set::{WriteDescriptorSet, DescriptorSet},
    buffer::{
        Subbuffer,
        BufferContents
    },
    command_buffer::{AutoCommandBufferBuilder, PrimaryAutoCommandBuffer, SubpassEndInfo, SubpassBeginInfo}
};

use crate::{
    Assets,
    ObjectFactory,
    UniformLocation,
    ShaderId,
    PipelineInfo,
    ResourceUploader,
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
    #[cfg(debug_assertions)]
    pub frame_parity: bool
}

impl<'a> ObjectCreatePartialInfo<'a>
{
    pub fn aspect(&self) -> f32
    {
        let [x, y] = self.size;

        x / y
    }

    pub fn to_full(self, camera: &Camera) -> ObjectCreateInfo<'a>
    {
        let projection_view = camera.projection_view();

        ObjectCreateInfo{partial: self, projection_view}
    }
}

pub struct ObjectCreateInfo<'a>
{
    pub partial: ObjectCreatePartialInfo<'a>,
    pub projection_view: Matrix4<f32>
}

impl ObjectCreateInfo<'_>
{
    pub fn update_camera(&mut self, camera: &Camera)
    {
        self.projection_view = camera.projection_view();
    }
}

pub struct InitPartialInfo<'a, T>
{
    pub object_info: ObjectCreatePartialInfo<'a>,
    pub setup: &'a T
}

pub type InitInfo<'a> = ObjectCreateInfo<'a>;

pub struct DrawInfo<'a>
{
    pub object_info: ObjectCreatePartialInfo<'a>,
    pub current_sets: Vec<Arc<DescriptorSet>>,
    pub attachments: &'a [Arc<ImageView>],
    current_pipeline: Option<usize>,
    pipelines: &'a [PipelineInfo]
}

impl<'a> DrawInfo<'a>
{
    pub fn new(
        object_info: ObjectCreatePartialInfo<'a>,
        pipelines: &'a [PipelineInfo],
        attachments: &'a [Arc<ImageView>]
    ) -> Self
    {
        Self{
            object_info,
            current_sets: Vec::new(),
            attachments,
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

    pub fn next_subpass(&mut self)
    {
        self.object_info.builder_wrapper.builder()
            .next_subpass(SubpassEndInfo::default(), SubpassBeginInfo::default())
            .unwrap();
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

    pub fn set_depth_test(&mut self, state: bool)
    {
        self.object_info.builder_wrapper.builder()
            .set_depth_test_enable(state)
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

    pub fn resource_uploader(&self) -> &ResourceUploader
    {
        self.object_info.builder_wrapper.resource_uploader()
    }

    pub fn create_descriptor_set(
        &self,
        set: usize,
        writes: impl IntoIterator<Item=WriteDescriptorSet>
    ) -> Arc<DescriptorSet>
    {
        let resource_uploader = self.resource_uploader();

        let shader = self.current_pipeline_id().unwrap();

        let info = &resource_uploader.pipeline_infos[shader.get_raw()];
        let descriptor_layout = info.layout.set_layouts().get(set)
            .unwrap()
            .clone();

        DescriptorSet::new(
            resource_uploader.descriptor_allocator.clone(),
            descriptor_layout,
            writes,
            []
        ).unwrap()
    }
}

pub type UpdateBuffersPartialInfo<'a> = ObjectCreatePartialInfo<'a>;
pub type UpdateBuffersInfo<'a> = ObjectCreateInfo<'a>;

pub trait GameObject
{
	fn update_buffers(&mut self, info: &mut UpdateBuffersInfo);
	fn draw(&self, info: &mut DrawInfo);
}
