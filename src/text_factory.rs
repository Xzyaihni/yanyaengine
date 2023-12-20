use std::sync::Arc;

use nalgebra::Matrix4;

use font_kit::{
    font::Font,
    source::SystemSource,
    properties::Properties,
    family_name::FamilyName
};

use crate::{
    ObjectFactory,
    TextObject,
    transform::Transform,
    text_object::CharsCreator,
    object::resource_uploader::ResourceUploader
};


pub struct FontsContainer
{
    font_textures: Vec<CharsCreator>
}

impl FontsContainer
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        object_factory: Arc<ObjectFactory>
    ) -> Self
    {
        let default_font = SystemSource::new()
            .select_best_match(&[FamilyName::SansSerif], &Properties::new())
            .unwrap()
            .load()
            .unwrap();

        Self::from_fonts(resource_uploader, object_factory, vec![default_font].into_iter())
    }

    fn from_fonts(
        resource_uploader: &mut ResourceUploader,
        object_factory: Arc<ObjectFactory>,
        fonts: impl Iterator<Item=Font>
    ) -> Self
    {
        let font_textures = fonts.map(|font|
        {
            CharsCreator::new(resource_uploader, object_factory.clone(), font)
        }).collect();

        Self{font_textures}
    }

    pub fn len(&self) -> usize
    {
        self.font_textures.len()
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut CharsCreator>
    {
        self.font_textures.get_mut(index)
    }
}

pub struct TextInfo<'a>
{
    pub transform: Transform,
    pub projection_view: Matrix4<f32>,
    pub text: &'a str
}

pub struct TextFactory<'a, 'b: 'a>
{
    resource_uploader: &'a mut ResourceUploader<'b>,
    fonts_container: &'a mut FontsContainer
}

impl<'a, 'b: 'a> TextFactory<'a, 'b>
{
    pub fn new(
        resource_uploader: &'a mut ResourceUploader<'b>,
        fonts_container: &'a mut FontsContainer
    ) -> Self
    {
        Self{resource_uploader, fonts_container}
    }

    pub fn create(&mut self, info: TextInfo) -> TextObject
    {
        TextObject::new(
            self.resource_uploader,
            info,
            self.fonts_container
        )
    }
}
