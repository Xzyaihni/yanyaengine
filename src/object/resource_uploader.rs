use std::sync::Arc;

use vulkano::{
    pipeline::PipelineLayout,
	memory::allocator::StandardMemoryAllocator,
	image::{SampleCount, sampler::Sampler},
	descriptor_set::{
		allocator::StandardDescriptorSetAllocator,
		layout::DescriptorSetLayout
	},
	command_buffer::{
		AutoCommandBufferBuilder,
		PrimaryAutoCommandBuffer
	}
};


pub struct PipelineInfo<'a>
{
	pub allocator: &'a StandardDescriptorSetAllocator,
	pub layout: Arc<DescriptorSetLayout>,
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
        let descriptor_layout = layout.set_layouts().get(0).unwrap().clone();

        Self{allocator, layout: descriptor_layout, sampler}
    }
}

impl<'a> Clone for PipelineInfo<'a>
{
    fn clone(&self) -> Self
    {
        Self{
            allocator: &self.allocator,
            layout: self.layout.clone(),
            sampler: self.sampler.clone()
        }
    }
}

pub struct ResourceUploader<'a>
{
	pub allocator: Arc<StandardMemoryAllocator>,
	pub builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
    pub samples: SampleCount,
	pub pipeline_info: PipelineInfo<'a>
}
