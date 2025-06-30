use std::{
    rc::Rc,
    sync::Arc
};

use nalgebra::Vector2;

use parking_lot::Mutex;

use vulkano::{
    device::Device,
    buffer::BufferUsage,
    memory::allocator::StandardMemoryAllocator
};

use crate::{
    ObjectFactory,
    AssetsPaths,
    Assets,
    allocators::{UniformAllocator, ObjectAllocator},
    text_factory::FontsContainer,
    game_object::*,
    object::resource_uploader::ResourceUploader
};


pub struct Engine
{
    fonts_info: Rc<FontsContainer>,
    object_factory: Rc<ObjectFactory>,
    uniform_allocator: Rc<UniformAllocator>,
    assets: Arc<Mutex<Assets>>
}

impl Engine
{
    pub fn new(
        assets_paths: &AssetsPaths,
        mut resource_uploader: ResourceUploader,
        device: Arc<Device>
    ) -> Self
    {
        let assets = Assets::new(
            &mut resource_uploader,
            assets_paths.textures.as_ref(),
            assets_paths.models.as_ref()
        );

        let assets = Arc::new(Mutex::new(assets));

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device));

        let vertex_allocator = ObjectAllocator::new(
            memory_allocator.clone(),
            BufferUsage::VERTEX_BUFFER | BufferUsage::TRANSFER_DST
        );

        let index_allocator = ObjectAllocator::new(
            memory_allocator.clone(),
            BufferUsage::INDEX_BUFFER | BufferUsage::TRANSFER_DST
        );

        let uniform_allocator = Rc::new(UniformAllocator::new(memory_allocator));

        let object_factory = ObjectFactory::new(vertex_allocator, index_allocator);
        let object_factory = Rc::new(object_factory);

        let fonts_info = Rc::new(FontsContainer::new());

        Self{fonts_info, object_factory, uniform_allocator, assets}
    }

    #[allow(unused_variables)]
    pub fn object_create_partial_info<'a>(
        &'a mut self,
        resource_uploader: ResourceUploader<'a>,
        size: [f32; 2],
        frame_parity: bool
    ) -> ObjectCreatePartialInfo<'a>
    {
        let builder_wrapper = BuilderWrapper::new(
            resource_uploader,
            self.object_factory.clone(),
            Vector2::new(size[0], size[1]),
            self.fonts_info.clone()
        );

        ObjectCreatePartialInfo{
            builder_wrapper,
            assets: self.assets.clone(),
            object_factory: self.object_factory.clone(),
            uniform_allocator: self.uniform_allocator.clone(),
            size,
            #[cfg(debug_assertions)]
            frame_parity
        }
    }

    pub fn init_partial_info<'a>(
        &'a mut self,
        resource_uploader: ResourceUploader<'a>,
        size: [f32; 2]
    ) -> InitPartialInfo<'a>
    {
        self.object_create_partial_info(resource_uploader, size, false)
    }

    pub fn swap_pipelines(&mut self)
    {
        self.assets.lock().swap_pipelines();
    }
}
