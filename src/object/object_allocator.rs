use std::sync::Arc;

use vulkano::{
	device::Device,
	buffer::{
		BufferUsage,
		Subbuffer,
		allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo}
	},
	memory::allocator::StandardMemoryAllocator
};

use super::{
    ObjectVertex,
    Model
};


#[derive(Debug, Clone)]
pub struct ObjectAllocator
{
	allocator: Arc<SubbufferAllocator>,
	frames: usize
}

impl ObjectAllocator
{
	pub fn new(device: Arc<Device>, frames: usize) -> Self
	{
		let allocator = StandardMemoryAllocator::new_default(device);
		let allocator = SubbufferAllocator::new(
			Arc::new(allocator),
			SubbufferAllocatorCreateInfo{
				buffer_usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
				..Default::default()
			}
		);

		let allocator = Arc::new(allocator);

		Self{allocator, frames}
	}

	pub fn subbuffers(&self, model: &Model) -> Box<[Subbuffer<[ObjectVertex]>]>
	{
		(0..self.frames).map(|_|
		{
			self.allocator.allocate_slice(model.vertices.len() as u64).unwrap()
		}).collect::<Vec<_>>().into_boxed_slice()
	}

	pub fn subbuffers_amount(&self) -> usize
	{
		self.frames
	}
}
