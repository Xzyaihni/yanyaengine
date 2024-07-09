use std::rc::Rc;

use font_kit::{
    font::Font,
    source::SystemSource,
    properties::{Properties, Weight},
    family_name::FamilyName
};

use serde::{Serialize, Deserialize};

use crate::{
    ObjectFactory,
    TextObject,
    UniformLocation,
    ShaderId,
    transform::Transform,
    text_object::CharsRasterizer,
    object::resource_uploader::ResourceUploader
};


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

    pub fn len(&self) -> usize
    {
        self.font_textures.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.font_textures.is_empty()
    }

    pub fn get_mut(&mut self, font: FontStyle) -> Option<&mut CharsRasterizer>
    {
        self.font_textures.get_mut(font as usize)
    }
}

pub struct TextInfo<'a>
{
    pub transform: Transform,
    pub font_size: u32,
    pub font: FontStyle,
    pub text: &'a str
}

pub struct TextFactory<'a, 'b: 'a>
{
    resource_uploader: &'a mut ResourceUploader<'b>,
    object_factory: Rc<ObjectFactory>,
    fonts_container: &'a mut FontsContainer
}

impl<'a, 'b: 'a> TextFactory<'a, 'b>
{
    pub fn new(
        resource_uploader: &'a mut ResourceUploader<'b>,
        object_factory: Rc<ObjectFactory>,
        fonts_container: &'a mut FontsContainer
    ) -> Self
    {
        Self{resource_uploader, object_factory, fonts_container}
    }

    pub fn create(
        &mut self,
        location: UniformLocation,
        shader: ShaderId,
        info: TextInfo
    ) -> TextObject
    {
        let style = info.font;

        TextObject::new(
            self.resource_uploader,
            &self.object_factory,
            info,
            self.fonts_container.get_mut(style).expect("all font styles must exist"),
            location,
            shader
        )
    }
}
