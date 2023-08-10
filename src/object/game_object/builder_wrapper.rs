use super::CommandBuilderType;

use crate::{
    TextInfo,
    TextObject,
    object::resource_uploader::ResourceUploader,
    text_factory::{FontsContainer, TextFactory}
};


pub struct BuilderWrapper<'a>
{
    resource_uploader: ResourceUploader<'a>,
    fonts_info: &'a mut FontsContainer
}

impl<'a> BuilderWrapper<'a>
{
    pub fn new(
        resource_uploader: ResourceUploader<'a>,
        fonts_info: &'a mut FontsContainer
    ) -> Self
    {
        Self{resource_uploader, fonts_info}
    }

    pub fn builder<'b>(&'b mut self) -> &'b mut CommandBuilderType
    {
        self.resource_uploader.builder
    }

    pub fn text_factory<'b>(&'b mut self) -> TextFactory<'b, 'a>
    where
        'a: 'b
    {
        TextFactory::new(&mut self.resource_uploader, self.fonts_info)
    }

    pub fn create_text(&mut self, info: TextInfo) -> TextObject
    {
        self.text_factory().create(info)
    }
}
