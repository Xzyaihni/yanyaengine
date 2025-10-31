use std::{fs, borrow::Cow, rc::Rc};

use nalgebra::Vector2;

use ab_glyph::FontVec;

use serde::{Serialize, Deserialize};

use crate::{
    ObjectFactory,
    TextObject,
    text_object::CharsRasterizer,
    object::resource_uploader::ResourceUploader
};

pub use crate::text_object::TextCreateInfo;


pub struct FontsContainer
{
    font_textures: Vec<CharsRasterizer>
}

impl FontsContainer
{
    pub fn new() -> Self
    {
        let load_font = |name: &str|
        {
            FontVec::try_from_vec(fs::read(name).unwrap_or_else(|err|
            {
                panic!("couldnt load file `{name}` ({err})")
            })).unwrap()
        };

        let fonts = vec![load_font("fonts/Roboto-Bold.ttf")];

        Self::from_fonts(fonts.into_iter())
    }

    fn from_fonts(fonts: impl Iterator<Item=FontVec>) -> Self
    {
        let font_textures = fonts.map(|font|
        {
            CharsRasterizer::new(font)
        }).collect();

        Self{font_textures}
    }

    pub fn text_height(&self, font_size: u32, screen_height: f32) -> f32
    {
        TextObject::text_height(self.default_font(), font_size, screen_height)
    }

    pub fn calculate_bounds(&self, info: &TextInfo, size: &Vector2<f32>) -> Vector2<f32>
    {
        TextObject::calculate_bounds(info, self.default_font(), size)
    }

    pub fn default_font(&self) -> &CharsRasterizer
    {
        self.get(0)
    }

    pub fn get(&self, index: usize) -> &CharsRasterizer
    {
        &self.font_textures[index]
    }

    pub fn len(&self) -> usize
    {
        self.font_textures.len()
    }

    pub fn is_empty(&self) -> bool
    {
        self.font_textures.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextOutline
{
    pub color: [u8; 3],
    pub size: u8
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextInfoBlock<'a>
{
    pub color: [u8; 3],
    pub text: Cow<'a, str>
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextBlocks<'a>(pub Vec<TextInfoBlock<'a>>);

impl<'a> TextBlocks<'a>
{
    pub fn single(color: [u8; 3], text: Cow<'a, str>) -> Self
    {
        Self(vec![TextInfoBlock{color, text}])
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextInfo<'a>
{
    pub font_size: u32,
    pub text: TextBlocks<'a>,
    pub outline: Option<TextOutline>
}

impl<'a> Default for TextInfo<'a>
{
    fn default() -> Self
    {
        Self{
            font_size: 16,
            text: TextBlocks(Vec::new()),
            outline: None
        }
    }
}

impl<'a> TextInfo<'a>
{
    pub fn new_simple(font_size: u32, text: impl Into<Cow<'a, str>>) -> Self
    {
        Self{font_size, text: TextBlocks::single([255; 3], text.into()), outline: None}
    }
}

pub struct TextFactory<'a, 'b: 'a>
{
    resource_uploader: &'a mut ResourceUploader<'b>,
    object_factory: Rc<ObjectFactory>,
    size: Vector2<f32>,
    fonts_container: &'a FontsContainer
}

impl<'a, 'b: 'a> TextFactory<'a, 'b>
{
    pub fn new(
        resource_uploader: &'a mut ResourceUploader<'b>,
        object_factory: Rc<ObjectFactory>,
        size: Vector2<f32>,
        fonts_container: &'a FontsContainer
    ) -> Self
    {
        Self{resource_uploader, object_factory, size, fonts_container}
    }

    pub fn create(
        &mut self,
        info: TextCreateInfo
    ) -> TextObject
    {
        TextObject::new(
            self.resource_uploader,
            &self.object_factory,
            &self.size,
            info,
            self.fonts_container.default_font()
        )
    }
}
