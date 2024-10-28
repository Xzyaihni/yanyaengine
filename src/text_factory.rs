use std::rc::Rc;

use font_kit::{
    font::Font,
    source::SystemSource,
    properties::{Properties, Weight},
    family_name::FamilyName
};

use serde::{Serialize, Deserialize};

use nalgebra::Vector2;

use crate::{
    ObjectFactory,
    TextObject,
    TextAlign,
    UniformLocation,
    ShaderId,
    text_object::CharsRasterizer,
    object::resource_uploader::ResourceUploader
};

pub use crate::text_object::TextCreateInfo;


#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FontStyle
{
    Sans = 0,
    Serif,
    Bold
}

pub struct FontsContainer
{
    font_textures: Vec<CharsRasterizer>
}

impl FontsContainer
{
    pub fn new() -> Self
    {
        let load_family = |family, properties: Properties|
        {
            SystemSource::new()
                .select_best_match(&[family], &properties)
                .unwrap()
                .load()
                .unwrap()
        };

        let fonts = vec![
            load_family(FamilyName::SansSerif, Properties::new()),
            load_family(FamilyName::Serif, Properties::new()),
            load_family(FamilyName::SansSerif, *Properties::new().weight(Weight::BOLD))
        ];

        Self::from_fonts(fonts.into_iter())
    }

    fn from_fonts(fonts: impl Iterator<Item=Font>) -> Self
    {
        let font_textures = fonts.map(|font|
        {
            CharsRasterizer::new(font)
        }).collect();

        Self{font_textures}
    }

    pub fn calculate_bounds(&self, info: TextInfo) -> Vector2<f32>
    {
        TextObject::calculate_bounds(info, self)
    }

    pub fn len(&self) -> usize
    {
        self.font_textures.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.font_textures.is_empty()
    }

    pub fn get(&self, font: FontStyle) -> Option<&CharsRasterizer>
    {
        self.font_textures.get(font as usize)
    }
}

pub struct TextInfo<'a>
{
    pub font_size: u32,
    pub font: FontStyle,
    pub align: TextAlign,
    pub text: &'a str
}

pub struct TextFactory<'a, 'b: 'a>
{
    resource_uploader: &'a mut ResourceUploader<'b>,
    object_factory: Rc<ObjectFactory>,
    fonts_container: &'a FontsContainer
}

impl<'a, 'b: 'a> TextFactory<'a, 'b>
{
    pub fn new(
        resource_uploader: &'a mut ResourceUploader<'b>,
        object_factory: Rc<ObjectFactory>,
        fonts_container: &'a FontsContainer
    ) -> Self
    {
        Self{resource_uploader, object_factory, fonts_container}
    }

    pub fn create(
        &mut self,
        location: UniformLocation,
        shader: ShaderId,
        info: TextCreateInfo
    ) -> TextObject
    {
        TextObject::new(
            self.resource_uploader,
            &self.object_factory,
            info,
            self.fonts_container,
            location,
            shader
        )
    }
}
