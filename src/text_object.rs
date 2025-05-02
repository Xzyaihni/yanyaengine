use std::{
    num::FpCategory,
    sync::Arc
};

use parking_lot::RwLock;

use nalgebra::{Vector2, Vector3};

use ab_glyph::FontVec;

use serde::{Serialize, Deserialize};

use crate::{
    Object,
    ObjectFactory,
    TextInfo,
    FontsContainer,
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
        /*let font_size = info.inner.font_size;

        let (chars_info, size, height_single) = Self::calculate_bounds_pixels(info.inner, fonts);

        let global_size = Self::bounds_to_global(screen_size, size);

        if size.x == 0 || size.y == 0
        {
            return Self{
                object: None,
                align,
                size: global_size
            };
        }

        let mut text_canvas = Canvas::new(
            Vector2I::new(size.x, size.y),
            Format::A8
        );

        chars_info.into_iter().for_each(|(x, y, c)|
        {
            current_font.render_glyph(
                &mut text_canvas,
                height_single,
                font_size,
                x,
                y,
                c
            );
        });

        let object = object_factory.create(ObjectInfo{
            model: Arc::new(RwLock::new(Model::square(1.0))),
            texture: Self::canvas_to_texture(resource_uploader, text_canvas, location, shader),
            transform: info.transform
        });

        let mut this = Self{
            object: Some(object),
            align,
            size: global_size
        };

        this.update_scale();

        this*/todo!()
    }

    /*pub fn calculate_bounds_pixels(
        info: TextInfo,
        font: &CharsRasterizer
    ) -> (Vec<(i32, usize, char)>, Vector2<i32>, i32)
    {
        let mut full_bounds = BoundsCalculator::new();

        let lines_count = info.text.lines().count();
        let chars_info: Vec<_> = info.text.lines().enumerate().flat_map(|(y, line)|
        {
            full_bounds.return_carriage();
            // i dunno how to not collect >_<
            line.chars().into_iter().map(|c|
            {
                let x = font.bounds(
                    &mut full_bounds,
                    info.font_size,
                    c
                ).0;

                (x, y, c)
            }).collect::<Vec<_>>()
        }).collect();

        let metrics = current_font.metrics();

        let height_font = metrics.ascent + metrics.descent.abs();

        let height_single = (height_font / metrics.units_per_em as f32 * info.font_size as f32)
            .round() as i32;

        let height = height_single * lines_count as i32;
        let width = full_bounds.width;

        (chars_info, Vector2::new(width, height), height_single)
    }*/

    fn bounds_to_global(size: &Vector2<f32>, bounds: Vector2<i32>) -> Vector2<f32>
    {
        let v: Vector2<f32> = bounds.cast();

        v.component_div(size)
    }

    pub fn calculate_bounds(
        info: TextInfo,
        font: &CharsRasterizer,
        screen_size: &Vector2<f32>
    ) -> Vector2<f32>
    {
        Vector2::zeros()
        // Self::bounds_to_global(screen_size, Self::calculate_bounds_pixels(info, font).1)
    }

    pub fn text_size(&self) -> Vector2<f32>
    {
        self.size
    }

    /*fn canvas_to_texture(
        resource_uploader: &mut ResourceUploader,
        canvas: Canvas,
        location: UniformLocation,
        shader: ShaderId
    ) -> Arc<RwLock<Texture>>
    {
        let colors = canvas.pixels.into_iter().map(|value|
        {
            Color::new(u8::MAX, u8::MAX, u8::MAX, value)
        }).collect::<Vec<_>>();

        let image = SimpleImage::new(colors, canvas.size.x() as usize, canvas.size.y() as usize);
        let texture = Texture::new(resource_uploader, image.into(), location, shader);

        Arc::new(RwLock::new(texture))
    }*/

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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct OriginOffset
{
    pub x: i32,
    pub y: i32
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

    /*pub fn metrics(&self) -> Metrics
    {
        self.font.metrics()
    }

    fn with_font(
        rasterizer: &CharsRasterizer,
        bounds_calculator: &mut BoundsCalculator,
        font_size: u32,
        c: char
    ) -> (i32, BoundsInfo)
    {
        let GlyphInfo{offset, width, height} = rasterizer.glyph_info(font_size, c);

        let advance = (rasterizer.advance(c) * font_size as f32).round() as i32;

        let info = BoundsInfo{
            origin: offset,
            width,
            height,
            advance
        };

        let x = bounds_calculator.process_character(info.clone());

        (x, info)
    }*/
}
