use std::sync::Arc;

use vulkano::{
    pipeline::PipelineLayout,
	memory::allocator::StandardMemoryAllocator,
	image::sampler::Sampler,
	descriptor_set::allocator::StandardDescriptorSetAllocator,
	command_buffer::{
		AutoCommandBufferBuilder,
		PrimaryAutoCommandBuffer
	}
};


pub struct PipelineInfo<'a>
{
	pub allocator: &'a StandardDescriptorSetAllocator,
	pub layout: Arc<PipelineLayout>,
	pub sampler: Arc<Sampler>
}

impl<'a> PipelineInfo<'a>
{
    pub fn new(
        allocator: &'a StandardDescriptorSetAllocator,
        sampler: Arc<Sampler>,
        layout: Arc<PipelineLayout>
    ) -> Self
    {
        Self{allocator, layout, sampler}
    }
}

impl<'a> Clone for PipelineInfo<'a>
{
    fn clone(&self) -> Self
    {
        Self{
            allocator: self.allocator,
            layout: self.layout.clone(),
            sampler: self.sampler.clone()
        }
    }
}

pub struct ResourceUploader<'a>
{
	pub allocator: Arc<StandardMemoryAllocator>,
	pub builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
	pub pipeline_info: PipelineInfo<'a>
}
