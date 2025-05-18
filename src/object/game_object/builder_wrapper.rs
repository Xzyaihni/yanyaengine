use std::rc::Rc;

use nalgebra::Vector2;

use super::CommandBuilderType;

use crate::{
    TextInfo,
    TextCreateInfo,
    TextObject,
    ObjectFactory,
    object::{Texture, texture::RgbaImage, resource_uploader::ResourceUploader},
    text_factory::{FontsContainer, TextFactory}
};


pub struct BuilderWrapper<'a>
{
    resource_uploader: ResourceUploader<'a>,
    object_factory: Rc<ObjectFactory>,
    size: Vector2<f32>,
    fonts: Rc<FontsContainer>
}

impl<'a> BuilderWrapper<'a>
{
    pub fn new(
        resource_uploader: ResourceUploader<'a>,
        object_factory: Rc<ObjectFactory>,
        size: Vector2<f32>,
        fonts: Rc<FontsContainer>
    ) -> Self
    {
        Self{resource_uploader, object_factory, size, fonts}
    }

    pub fn fonts(&self) -> &Rc<FontsContainer>
    {
        &self.fonts
    }

    pub fn resource_uploader_mut<'b>(&'b mut self) -> &'b mut ResourceUploader<'a>
    {
        &mut self.resource_uploader
    }

    pub fn resource_uploader(&self) -> &ResourceUploader
    {
        &self.resource_uploader
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
            self.size,
            &self.fonts
        )
    }

    pub fn create_texture(
        &mut self,
        image: RgbaImage
    ) -> Texture
    {
        Texture::new(&mut self.resource_uploader, image)
    }

    pub fn create_text(
        &mut self,
        info: TextCreateInfo
    ) -> TextObject
    {
        self.text_factory().create(info)
    }

    pub fn text_bounds(
        &self,
        info: TextInfo
    ) -> Vector2<f32>
    {
        TextObject::calculate_bounds(info, self.fonts.default_font(), &self.size)
    }
}
