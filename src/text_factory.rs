use std::sync::Arc;

use parking_lot::Mutex;

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
    object_factory: Arc<Mutex<ObjectFactory>>,
    font_textures: Vec<CharsCreator>
}

impl FontsContainer
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        object_factory: Arc<Mutex<ObjectFactory>>
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
        object_factory: Arc<Mutex<ObjectFactory>>,
        fonts: impl Iterator<Item=Font>
    ) -> Self
    {
        let font_textures = fonts.map(|font|
        {
            CharsCreator::new(resource_uploader, object_factory.clone(), font)
        }).collect();

        Self{object_factory, font_textures}
    }

    pub fn add_fitting(&mut self, resource_uploader: &mut ResourceUploader, c: char)
    {
        if let Ok(all_fonts) = SystemSource::new().all_fonts()
        {
            let matched_font =  all_fonts.into_iter().filter_map(|font|
            {
                font.load().ok()
            }).find(|font|
            {
                font.glyph_for_char(c).is_some()
            });

            if let Some(font) = matched_font
            {
                let chars_creator = CharsCreator::new(
                    resource_uploader,
                    self.object_factory.clone(),
                    font
                );

                self.font_textures.push(chars_creator);
            }
        }
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
            &mut self.fonts_container
        )
    }
}
