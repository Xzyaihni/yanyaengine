use std::sync::Arc;

use parking_lot::RwLock;

use nalgebra::Vector2;

use ab_glyph::{Font, ScaleFont, FontVec, PxScaleFont, Glyph, Point};

use crate::{
    Object,
    ObjectFactory,
    TextInfo,
    ObjectInfo,
    UniformLocation,
    ShaderId,
    transform::{TransformContainer, Transform},
    game_object::*,
    object::{
        resource_uploader::ResourceUploader,
        model::Model,
        texture::{Texture, Color, SimpleImage}
    }
};


pub struct TextCreateInfo<'a>
{
    pub transform: Transform,
    pub inner: TextInfo<'a>
}

struct BoundsInfo<'a>
{
    advance: f32,
    glyph: &'a Glyph
}

struct BoundsCalculator
{
    line_gap: f32,
    position: Vector2<f32>,
    width: f32,
    height: f32
}

impl BoundsCalculator
{
    fn new(line_gap: f32) -> Self
    {
        Self{
            line_gap,
            position: Vector2::zeros(),
            width: 0.0,
            height: 0.0
        }
    }

    fn process(&mut self, bounds: BoundsInfo) -> Vector2<f32>
    {
        let this_position = self.position;

        self.position.x += bounds.advance;

        self.width = self.width.max(self.position.x);
        self.height = self.height.max(self.position.y + bounds.glyph.scale.y);

        this_position
    }

    fn return_carriage(&mut self)
    {
        self.position.x = 0.0;
        self.position.y += self.line_gap;
    }
}

struct CharInfo
{
    glyph: Glyph
}

struct ProcessedInfo
{
    chars: Vec<CharInfo>,
    bounds: Vector2<f32>
}

#[derive(Debug)]
pub struct TextObject
{
    pub object: Option<Object>,
    size: Vector2<f32>
}

impl TextObject
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        object_factory: &ObjectFactory,
        screen_size: &Vector2<f32>,
        info: TextCreateInfo,
        font: &CharsRasterizer,
        location: UniformLocation,
        shader: ShaderId
    ) -> Self
    {
        let font = font.with_font_size(info.inner.font_size);
        let ProcessedInfo{chars: chars_info, bounds} = Self::process_text(info.inner, &font);

        let global_size = Self::bounds_to_global(screen_size, bounds);

        if bounds.x <= 0.0 || bounds.y <= 0.0
        {
            return Self{
                object: None,
                size: global_size
            };
        }

        let mut image = SimpleImage::filled(
            Color{r: 255, g: 255, b: 255, a: 0},
            bounds.x.ceil() as usize,
            bounds.y.ceil() as usize
        );

        chars_info.into_iter().for_each(|info|
        {
            font.render(&mut image, info);
        });

        let texture = Texture::new(resource_uploader, image.into(), location, shader);

        let object = object_factory.create(ObjectInfo{
            model: Arc::new(RwLock::new(Model::square(1.0))),
            texture: Arc::new(RwLock::new(texture)),
            transform: info.transform
        });

        Self{
            object: Some(object),
            size: global_size
        }
    }

    fn process_text(
        info: TextInfo,
        font: &CharsRasterizerScaled
    ) -> ProcessedInfo
    {
        let mut full_bounds = BoundsCalculator::new(font.font.height() + font.font.line_gap());

        let chars: Vec<_> = info.text.lines().enumerate().flat_map(|(index, line)|
        {
            if index != 0
            {
                full_bounds.return_carriage();
            }

            // i dunno how to not collect >_<
            line.chars().map(|c|
            {
                font.bounds(&mut full_bounds, c)
            }).collect::<Vec<_>>()
        }).collect();

        let height = full_bounds.height;
        let width = full_bounds.width;

        ProcessedInfo{chars, bounds: Vector2::new(width, height)}
    }

    fn bounds_to_global(size: &Vector2<f32>, bounds: Vector2<f32>) -> Vector2<f32>
    {
        bounds.component_div(size)
    }

    pub fn text_height(
        font: &CharsRasterizer,
        font_size: u32,
        screen_height: f32
    ) -> f32
    {
        font.with_font_size(font_size).height() / screen_height
    }

    pub fn calculate_bounds(
        info: TextInfo,
        font: &CharsRasterizer,
        screen_size: &Vector2<f32>
    ) -> Vector2<f32>
    {
        let font = font.with_font_size(info.font_size);
        Self::bounds_to_global(screen_size, Self::process_text(info, &font).bounds)
    }

    pub fn text_size(&self) -> Vector2<f32>
    {
        self.size
    }

    pub fn texture(&self) -> Option<&Arc<RwLock<Texture>>>
    {
        self.object.as_ref().map(|x| x.texture())
    }

    pub fn transform(&self) -> Option<&Transform>
    {
        self.object.as_ref().map(|object| object.transform_ref())
    }
}

impl GameObject for TextObject
{
    fn update_buffers(&mut self, info: &mut UpdateBuffersInfo)
    {
        if let Some(object) = self.object.as_mut()
        {
            object.update_buffers(info);
        }
    }

    fn draw(&self, info: &mut DrawInfo)
    {
        if let Some(object) = self.object.as_ref()
        {
            object.draw(info);
        }
    }
}

pub struct CharsRasterizer
{
    font: FontVec
}

impl CharsRasterizer
{
    pub fn new(font: FontVec) -> Self
    {
        Self{font}
    }

    fn with_font_size(&self, font_size: u32) -> CharsRasterizerScaled
    {
        let pixel_scale = self.font.pt_to_px_scale(font_size as f32).unwrap();

        CharsRasterizerScaled{font: self.font.as_scaled(pixel_scale)}
    }
}

struct CharsRasterizerScaled<'a>
{
    pub font: PxScaleFont<&'a FontVec>
}

impl CharsRasterizerScaled<'_>
{
    fn bounds(&self, bounds_calculator: &mut BoundsCalculator, c: char) -> CharInfo
    {
        let glyph_id = self.font.glyph_id(c);
        let mut glyph = self.font.scaled_glyph(c);

        let offset = bounds_calculator.process(BoundsInfo{
            advance: self.font.h_advance(glyph_id),
            glyph: &glyph
        });

        glyph.position = Point{x: offset.x, y: offset.y};

        CharInfo{glyph}
    }

    fn height(&self) -> f32
    {
        self.font.scale.y
    }

    fn render(&self, image: &mut SimpleImage, info: CharInfo)
    {
        let position = info.glyph.position;
        let ascent = self.font.ascent();

        if let Some(outlined) = self.font.outline_glyph(info.glyph)
        {
            let px_bounds = outlined.px_bounds();

            outlined.draw(|x, y, amount|
            {
                let x = (x as f32 + position.x) as usize;
                let y = (y as f32 + ascent + px_bounds.min.y) as usize;

                if !((0..image.width).contains(&x) && (0..image.height).contains(&y))
                {
                    return;
                }

                let color = Color{r: 255, g: 255, b: 255, a: (amount * 255.0) as u8};
                image.set_pixel(color, x, y);
            });
        }
    }
}
