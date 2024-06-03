use std::{
    rc::Rc,
    sync::Arc
};

use parking_lot::Mutex;

use vulkano::device::Device;

use crate::{
    PipelineInfo,
    ObjectFactory,
    AssetsPaths,
    Assets,
    allocators::ObjectAllocator,
    text_factory::FontsContainer,
    game_object::*,
    object::resource_uploader::ResourceUploader
};


pub struct Engine
{
    fonts_info: FontsContainer,
    object_factory: Rc<ObjectFactory>,
    assets: Arc<Mutex<Assets>>
}

impl Engine
{
    pub fn new(
        assets_paths: &AssetsPaths,
        mut resource_uploader: ResourceUploader,
        device: Arc<Device>,
        frames: usize
    ) -> Self
    {
        let assets = Assets::new(
            &mut resource_uploader,
            assets_paths.textures.as_ref(),
            assets_paths.models.as_ref()
        );

        let assets = Arc::new(Mutex::new(assets));

        let allocator = ObjectAllocator::new(device, frames);

        let object_factory = ObjectFactory::new(allocator);
        let object_factory = Rc::new(object_factory);

        let fonts_info = FontsContainer::new();

        Self{fonts_info, object_factory, assets}
    }

    pub fn object_create_partial_info<'a>(
        &'a mut self,
        resource_uploader: ResourceUploader<'a>,
        size: [f32; 2],
        image_index: usize
    ) -> ObjectCreatePartialInfo<'a>
    {
        let builder_wrapper = BuilderWrapper::new(
            resource_uploader,
            self.object_factory.clone(),
            &mut self.fonts_info
        );

        ObjectCreatePartialInfo{
            builder_wrapper,
            assets: self.assets.clone(),
            object_factory: self.object_factory.clone(),
            size,
            image_index
        }
    }

    pub fn init_partial_info<'a>(
        &'a mut self,
        resource_uploader: ResourceUploader<'a>,
        size: [f32; 2],
        image_index: usize
    ) -> InitPartialInfo<'a>
    {
        self.object_create_partial_info(resource_uploader, size, image_index)
    }

    pub fn swap_pipeline(&mut self, info: &PipelineInfo)
    {
        self.assets.lock().swap_pipeline(info);
    }
}
