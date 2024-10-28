use std::rc::Rc;

use nalgebra::Vector2;

use super::CommandBuilderType;

use crate::{
    TextInfo,
    TextCreateInfo,
    TextObject,
    ObjectFactory,
    UniformLocation,
    ShaderId,
    object::{Texture, texture::RgbaImage, resource_uploader::ResourceUploader},
    text_factory::{FontsContainer, TextFactory}
};


pub struct BuilderWrapper<'a>
{
    resource_uploader: ResourceUploader<'a>,
    object_factory: Rc<ObjectFactory>,
    fonts: Rc<FontsContainer>
}

impl<'a> BuilderWrapper<'a>
{
    pub fn new(
        resource_uploader: ResourceUploader<'a>,
        object_factory: Rc<ObjectFactory>,
        fonts: Rc<FontsContainer>
    ) -> Self
    {
        Self{resource_uploader, object_factory, fonts}
    }

    pub fn fonts(&self) -> &Rc<FontsContainer>
    {
        &self.fonts
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
            &self.fonts
        )
    }

    pub fn create_texture(
        &mut self,
        image: RgbaImage,
        location: UniformLocation,
        shader: ShaderId
    ) -> Texture
    {
        Texture::new(&mut self.resource_uploader, image, location, shader)
    }

    pub fn create_text(
        &mut self,
        info: TextCreateInfo,
        location: UniformLocation,
        shader: ShaderId
    ) -> TextObject
    {
        self.text_factory().create(location, shader, info)
    }

    pub fn text_bounds(
        &self,
        info: TextInfo
    ) -> Vector2<f32>
    {
        TextObject::calculate_bounds(info, &self.fonts)
    }
}
