use std::{
    rc::Rc,
    sync::Arc
};

use parking_lot::Mutex;

use vulkano::{
    device::Device,
    memory::allocator::StandardMemoryAllocator
};

use crate::{
    ObjectFactory,
    AssetsPaths,
    ShadersQuery,
    Assets,
    UniformLocation,
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
        device: Arc<Device>,
        shaders_query: ShadersQuery
    ) -> Self
    {
        let assets = Assets::new(
            &mut resource_uploader,
            assets_paths.textures.as_ref(),
            assets_paths.models.as_ref(),
            UniformLocation{set: 0, binding: 0},
            shaders_query
        );

        let assets = Arc::new(Mutex::new(assets));

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device));
        let allocator = ObjectAllocator::new(memory_allocator.clone());
        let uniform_allocator = Rc::new(UniformAllocator::new(memory_allocator));

        let object_factory = ObjectFactory::new(allocator);
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

    pub fn swap_pipelines(&mut self, resource_uploader: &ResourceUploader)
    {
        self.assets.lock().swap_pipelines(resource_uploader);
    }
}
