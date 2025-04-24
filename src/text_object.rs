use std::{
    num::FpCategory,
    sync::Arc
};

use parking_lot::RwLock;

use font_kit::{
    metrics::Metrics,
    hinting::HintingOptions,
    font::Font,
    canvas::{RasterizationOptions, Format, Canvas}
};

use pathfinder_geometry::{
    transform2d::Transform2F,
    vector::{Vector2I, Vector2F}
};

use nalgebra::{Vector2, Vector3};

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


#[allow(dead_code)]
#[derive(Debug, Clone)]
struct BoundsInfo
{
    origin: OriginOffset,
    width: u32,
    height: u32,
    advance: i32
}

#[allow(dead_code)]
struct BoundsCalculator
{
    width: i32,
    x: i32,
    y: u32
}

impl BoundsCalculator
{
    pub fn new() -> Self
    {
        Self{
            width: 0,
            x: 0,
            y: 0
        }
    }

    pub fn process_character(&mut self, info: BoundsInfo) -> i32
    {
        self.width = self.width.max(self.x + info.origin.x + info.width as i32);

        let this_x = self.x + info.origin.x;

        self.x += info.advance;

        this_x
    }

    pub fn return_carriage(&mut self)
    {
        self.x = 0;
    }
}

#[derive(Debug)]
pub struct GlyphInfo
{
    pub offset: OriginOffset,
    pub width: u32,
    pub height: u32
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HorizontalAlign
{
    Left,
    Middle,
    Right
}

impl HorizontalAlign
{
    pub fn sign(self) -> f32
    {
        match self
        {
            Self::Left => -1.0,
            Self::Middle => 0.0,
            Self::Right => 1.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerticalAlign
{
    Top,
    Middle,
    Bottom
}

impl VerticalAlign
{
    pub fn sign(self) -> f32
    {
        match self
        {
            Self::Top => -1.0,
            Self::Middle => 0.0,
            Self::Bottom => 1.0
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextAlign
{
    pub horizontal: HorizontalAlign,
    pub vertical: VerticalAlign
}

impl Default for TextAlign
{
    fn default() -> Self
    {
        Self{
            horizontal: HorizontalAlign::Left,
            vertical: VerticalAlign::Top
        }
    }
}

impl TextAlign
{
    pub fn centered() -> Self
    {
        Self{
            horizontal: HorizontalAlign::Middle,
            vertical: VerticalAlign::Middle
        }
    }
}

pub struct TextCreateInfo<'a>
{
    pub transform: Transform,
    pub inner: TextInfo<'a>
}

#[derive(Debug)]
pub struct TextObject
{
    pub object: Option<Object>,
    align: Option<TextAlign>,
    size: Vector2<f32>
}

impl TextObject
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        object_factory: &ObjectFactory,
        screen_size: &Vector2<f32>,
        info: TextCreateInfo,
        fonts: &FontsContainer,
        location: UniformLocation,
        shader: ShaderId
    ) -> Self
    {
        let current_font = fonts.get(info.inner.font).expect("style must exist");

        let align = info.inner.align.clone();
        let font_size = info.inner.font_size;

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

        this
    }

    pub fn calculate_bounds_pixels(
        info: TextInfo,
        fonts: &FontsContainer
    ) -> (Vec<(i32, usize, char)>, Vector2<i32>, i32)
    {
        let current_font = fonts.get(info.font).expect("style must exist");

        let mut full_bounds = BoundsCalculator::new();

        let lines_count = info.text.lines().count();
        let chars_info: Vec<_> = info.text.lines().enumerate().flat_map(|(y, line)|
        {
            full_bounds.return_carriage();
            // i dunno how to not collect >_<
            line.chars().into_iter().map(|c|
            {
                let x = Self::with_font(
                    current_font,
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
    }

    fn bounds_to_global(size: &Vector2<f32>, bounds: Vector2<i32>) -> Vector2<f32>
    {
        let v: Vector2<f32> = bounds.cast();

        v.component_div(size)
    }

    pub fn calculate_bounds(
        info: TextInfo,
        fonts: &FontsContainer,
        screen_size: &Vector2<f32>
    ) -> Vector2<f32>
    {
        Self::bounds_to_global(screen_size, Self::calculate_bounds_pixels(info, fonts).1)
    }

    pub fn text_size(&self) -> Vector2<f32>
    {
        self.size
    }

    pub fn update_scale(&mut self)
    {
        if let Some(object) = self.object.as_mut()
        {
            let align = if let Some(x) = self.align.as_ref() { x } else { return; };

            let scale = object.scale();

            let model_size = if scale.x.classify() == FpCategory::Zero || scale.y.classify() == FpCategory::Zero
            {
                Vector2::zeros()
            } else
            {
                self.size.component_div(&scale.xy())
            };

            let shift = (Vector2::repeat(1.0) - model_size.xy()) / 2.0;

            let mut model = Model::rectangle(model_size.x, model_size.y);
            model.shift(Vector3::new(
                shift.x * align.horizontal.sign(),
                shift.y * align.vertical.sign(),
                0.0
            ));

            object.set_inplace_model_same_sized(model);
        }
    }

    fn canvas_to_texture(
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

#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct OriginOffset
{
    pub x: i32,
    pub y: i32
}

pub struct CharsRasterizer
{
    font: Font
}

impl CharsRasterizer
{
    pub fn new(font: Font) -> Self
    {
        Self{font}
    }

    pub fn metrics(&self) -> Metrics
    {
        self.font.metrics()
    }

    pub fn units_per_em(&self) -> f32
    {
        self.metrics().units_per_em as f32
    }

    pub fn advance(&self, c: char) -> f32
    {
        const DEFAULT_ADVANCE: f32 = 0.0;

        let id = match self.font.glyph_for_char(c)
        {
            Some(id) => id,
            None =>
            {
                // eprintln!("couldnt get the advance of {c}, returning {DEFAULT_ADVANCE}");
                return DEFAULT_ADVANCE
            }
        };

        let advance = match self.font.advance(id)
        {
            Ok(id) => id,
            Err(_err) =>
            {
                // eprintln!("couldnt get the advance of {c} ({err}), returning {DEFAULT_ADVANCE}");
                return DEFAULT_ADVANCE
            }
        };

        advance.x() / self.units_per_em()
    }

    fn glyph_info(
        &self,
        font_size: u32,
        c: char
    ) -> GlyphInfo
    {
        let id = match self.font.glyph_for_char(c)
        {
            Some(id) => id,
            None =>
            {
                eprintln!("couldnt get the offset of {c}");
                return GlyphInfo{
                    offset: OriginOffset{
                        x: 0,
                        y: 0
                    },
                    width: 0,
                    height: 0
                };
            }
        };

        let font_size_f = font_size as f32;
        let bounds = self.font.raster_bounds(
            id,
            font_size_f,
            Transform2F::from_translation(Vector2F::new(0.0, font_size_f)),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa
        ).unwrap();

        let offset = OriginOffset{
            x: bounds.origin().x(),
            y: bounds.origin().y()
        };

        GlyphInfo{
            offset,
            width: bounds.size().x() as u32,
            height: bounds.size().y() as u32
        }
    }

    pub fn render_glyph(
        &self,
        canvas: &mut Canvas,
        height: i32,
        font_size: u32,
        char_x: i32,
        char_y: usize,
        c: char
    ) -> Option<()>
    {
        let small = self.render_small(font_size, height, c)?;

        let start_x = char_x.max(0) as usize;

        let big_width = canvas.size.x() as usize;

        let width = small.size.x() as usize;

        let y_offset = char_y * height as usize;

        for y in 0..height as usize
        {
            for x in 0..width
            {
                let offset_x = start_x + x;
                if offset_x >= big_width
                {
                    continue;
                }

                let small_y = y * width;
                let big_y = (y + y_offset) * big_width;

                let this_pixel = &mut canvas.pixels[big_y + offset_x];
                *this_pixel = this_pixel.saturating_add(small.pixels[small_y + x]);
            }
        }

        Some(())
    }

    fn render_small(
        &self,
        font_size: u32,
        canvas_height: i32,
        c: char
    ) -> Option<Canvas>
    {
        let id = self.font.glyph_for_char(c)?;

        let point_size = font_size as f32;

        let hinting = HintingOptions::None;
        let options = RasterizationOptions::GrayscaleAa;

        let bounds = self.font.raster_bounds(
            id,
            point_size,
            Transform2F::from_translation(Vector2F::new(0.0, 0.0)),
            hinting,
            options
        ).ok()?;

        let mut canvas = Canvas::new(Vector2I::new(font_size as i32, canvas_height), Format::A8);

        let origin = bounds.origin();
        let offset = Vector2F::new(
            -origin.x() as f32,
            font_size as f32
        );

        self.font.rasterize_glyph(
            &mut canvas,
            id,
            point_size,
            Transform2F::from_translation(offset),
            hinting,
            options
        ).ok()?;

        Some(canvas)
    }
}
