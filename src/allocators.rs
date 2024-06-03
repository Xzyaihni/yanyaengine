use std::sync::Arc;

use vulkano::{
	buffer::{
        BufferContents,
		BufferUsage,
		Subbuffer,
		allocator::{SubbufferAllocator, SubbufferAllocatorCreateInfo}
	},
	memory::allocator::{
        GenericMemoryAllocator,
        FreeListAllocator,
        MemoryTypeFilter
    }
};

use crate::object::{
    ObjectVertex,
    Model
};


type ThisMemoryAllocator = GenericMemoryAllocator<FreeListAllocator>;

#[derive(Debug)]
pub struct ObjectAllocator
{
	allocator: SubbufferAllocator,
	frames: usize
}

impl ObjectAllocator
{
	pub fn new(
        allocator: Arc<ThisMemoryAllocator>,
        frames: usize
    ) -> Self
	{
		let allocator = SubbufferAllocator::new(
			allocator,
			SubbufferAllocatorCreateInfo{
				buffer_usage: BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
				..Default::default()
			}
		);

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

#[derive(Debug)]
pub struct UniformAllocator
{
	allocator: SubbufferAllocator
}

impl UniformAllocator
{
	pub fn new(allocator: Arc<ThisMemoryAllocator>) -> Self
	{
		let allocator = SubbufferAllocator::new(
			allocator,
			SubbufferAllocatorCreateInfo{
				buffer_usage: BufferUsage::UNIFORM_BUFFER,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
				..Default::default()
			}
		);

		Self{allocator}
	}

	pub fn allocate_sized<T: BufferContents>(&self) -> Subbuffer<T>
	{
        self.allocator.allocate_sized().unwrap()
	}
}
