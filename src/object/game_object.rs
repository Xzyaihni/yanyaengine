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

#[cfg(debug_assertions)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameParity
{
    Even = 0,
    Odd = 1,
    Ignore = 2
}

pub struct ObjectCreatePartialInfo<'a>
{
    pub builder_wrapper: BuilderWrapper<'a>,
    pub assets: Arc<Mutex<Assets>>,
    pub object_factory: Rc<ObjectFactory>,
    pub uniform_allocator: Rc<UniformAllocator>,
    pub size: [f32; 2],
    #[cfg(debug_assertions)]
    pub frame_parity: FrameParity
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

    pub fn with_projection(&mut self, projection: Matrix4<f32>, mut f: impl FnMut(&mut Self))
    {
        let previous = self.projection_view;
        self.projection_view = projection;

        f(self);

        self.projection_view = previous;
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

        let builder = self.object_info.builder_wrapper.builder();

        #[cfg(debug_assertions)]
        {
            builder.bind_pipeline_graphics(pipeline).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            unsafe{ builder.bind_pipeline_graphics_unchecked(pipeline); }
        }
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
        let builder = self.object_info.builder_wrapper.builder();

        #[cfg(debug_assertions)]
        {
            builder.next_subpass(SubpassEndInfo::default(), SubpassBeginInfo::default()).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            unsafe{ builder.next_subpass_unchecked(SubpassEndInfo::default(), SubpassBeginInfo::default()); }
        }
    }

    #[allow(dead_code)]
    pub fn push_constants<T: BufferContents>(
        &mut self,
        constants: T
    )
    {
        let layout = self.current_layout();

        let builder = self.object_info.builder_wrapper.builder();

        #[cfg(debug_assertions)]
        {
            builder.push_constants(layout, 0, constants).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            unsafe{ builder.push_constants_unchecked(layout, 0, constants); }
        }
    }

    #[allow(dead_code)]
    pub fn push_uniform_buffer<T: BufferContents>(
        &mut self,
        location: UniformLocation,
        buffer: Subbuffer<T>
    )
    {
        let layout = self.current_layout();

        let builder = self.object_info.builder_wrapper.builder();

        let descriptor_sets = vec![WriteDescriptorSet::buffer(location.binding, buffer)].into();

        #[cfg(debug_assertions)]
        {
            builder.push_descriptor_set(PipelineBindPoint::Graphics, layout, location.set, descriptor_sets).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            unsafe{ builder.push_descriptor_set_unchecked(PipelineBindPoint::Graphics, layout, location.set, descriptor_sets); }
        }
    }

    pub fn set_depth_test(&mut self, state: bool)
    {
        let builder = self.object_info.builder_wrapper.builder();

        #[cfg(debug_assertions)]
        {
            builder.set_depth_test_enable(state).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            unsafe{ builder.set_depth_test_enable_unchecked(state); }
        }
    }

    pub fn set_depth_write(&mut self, state: bool)
    {
        let builder = self.object_info.builder_wrapper.builder();

        #[cfg(debug_assertions)]
        {
            builder.set_depth_write_enable(state).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            unsafe{ builder.set_depth_write_enable_unchecked(state); }
        }
    }

    pub fn set_scissor(&mut self, scissor: Scissor)
    {
        let builder = self.object_info.builder_wrapper.builder();

        #[cfg(debug_assertions)]
        {
            builder.set_scissor(0, vec![scissor].into()).unwrap();
        }

        #[cfg(not(debug_assertions))]
        {
            unsafe{ builder.set_scissor_unchecked(0, vec![scissor].into()); }
        }
    }

    pub fn reset_scissor(&mut self)
    {
        self.set_scissor(Scissor::default())
    }

    pub fn resource_uploader(&self) -> &ResourceUploader<'_>
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
