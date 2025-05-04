use std::{fs, rc::Rc};

use nalgebra::Vector2;

use ab_glyph::FontVec;

use crate::{
    ObjectFactory,
    TextObject,
    UniformLocation,
    ShaderId,
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

    pub fn calculate_bounds(&self, info: TextInfo, size: &Vector2<f32>) -> Vector2<f32>
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

pub struct TextInfo<'a>
{
    pub font_size: u32,
    pub text: &'a str
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
        location: UniformLocation,
        shader: ShaderId,
        info: TextCreateInfo
    ) -> TextObject
    {
        TextObject::new(
            self.resource_uploader,
            &self.object_factory,
            &self.size,
            info,
            self.fonts_container.default_font(),
            location,
            shader
        )
    }
}
