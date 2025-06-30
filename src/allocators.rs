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


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UniformLocation
{
    pub set: u32,
    pub binding: u32
}

type ThisMemoryAllocator = GenericMemoryAllocator<FreeListAllocator>;

#[derive(Debug)]
pub struct ObjectAllocator
{
	allocator: SubbufferAllocator
}

impl ObjectAllocator
{
	pub fn new(allocator: Arc<ThisMemoryAllocator>, buffer_usage: BufferUsage) -> Self
	{
		let allocator = SubbufferAllocator::new(
			allocator,
			SubbufferAllocatorCreateInfo{
				buffer_usage,
                memory_type_filter: MemoryTypeFilter::PREFER_DEVICE
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
				..Default::default()
			}
		);

		Self{allocator}
	}

	pub fn subbuffer<T: BufferContents>(&self, size: u64) -> Subbuffer<[T]>
	{
		self.allocator.allocate_slice(size).unwrap()
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
