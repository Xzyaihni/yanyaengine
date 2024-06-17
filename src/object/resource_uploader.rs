use std::sync::Arc;

use vulkano::{
	memory::allocator::StandardMemoryAllocator,
	image::sampler::Sampler,
	descriptor_set::allocator::StandardDescriptorSetAllocator,
	command_buffer::{
		AutoCommandBufferBuilder,
		PrimaryAutoCommandBuffer
	}
};

use crate::PipelineInfo;


pub struct ResourceUploader<'a>
{
	pub allocator: Arc<StandardMemoryAllocator>,
	pub descriptor_allocator: Arc<StandardDescriptorSetAllocator>,
	pub sampler: Arc<Sampler>,
	pub builder: &'a mut AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>,
	pub pipeline_infos: &'a [PipelineInfo]
}
