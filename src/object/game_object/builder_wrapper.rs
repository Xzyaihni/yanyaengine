use std::sync::Arc;

use super::CommandBuilderType;

use crate::{
    TextInfo,
    TextObject,
    ObjectFactory,
    object::{Texture, texture::RgbaImage, resource_uploader::ResourceUploader},
    text_factory::{FontsContainer, TextFactory}
};


pub struct BuilderWrapper<'a>
{
    resource_uploader: ResourceUploader<'a>,
    object_factory: Arc<ObjectFactory>,
    fonts_info: &'a mut FontsContainer
}

impl<'a> BuilderWrapper<'a>
{
    pub fn new(
        resource_uploader: ResourceUploader<'a>,
        object_factory: Arc<ObjectFactory>,
        fonts_info: &'a mut FontsContainer
    ) -> Self
    {
        Self{resource_uploader, object_factory, fonts_info}
    }

    pub fn resource_uploader<'b>(&'b mut self) -> &'b mut ResourceUploader<'a>
    {
        &mut self.resource_uploader
    }

    pub fn builder(&mut self) -> &mut CommandBuilderType
    {
        self.resource_uploader.builder
    }

    pub fn text_factory<'b>(&'b mut self) -> TextFactory<'b, 'a>
    where
        'a: 'b
    {
        TextFactory::new(
            &mut self.resource_uploader,
            self.object_factory.clone(),
            self.fonts_info
        )
    }

    pub fn create_texture(&mut self, image: RgbaImage) -> Texture
    {
        Texture::new(&mut self.resource_uploader, image)
    }

    pub fn create_text(&mut self, info: TextInfo) -> TextObject
    {
        self.text_factory().create(info)
    }
}
