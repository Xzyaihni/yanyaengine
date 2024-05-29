use std::{
    rc::Rc,
    sync::Arc
};

use vulkano::{
	device::Device,
	buffer::{
		BufferUsage,
		Subbuffer,
		allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo}
	},
	memory::allocator::{MemoryTypeFilter, StandardMemoryAllocator}
};

use super::{
    ObjectVertex,
    Model
};


#[derive(Debug, Clone)]
pub struct ObjectAllocator
{
	allocator: Rc<SubbufferAllocator>,
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
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
				..Default::default()
			}
		);

		let allocator = Rc::new(allocator);

		Self{allocator, frames}
	}

	pub fn subbuffers(&self, model: &Model) -> Box<[Subbuffer<[ObjectVertex]>]>
	{
		(0..self.frames).map(|_|
		{
			self.allocator.allocate_slice(model.vertices.len() as u64).unwrap()
		}).collect::<Box<[_]>>()
	}

	pub fn subbuffers_amount(&self) -> usize
	{
		self.frames
	}
}
