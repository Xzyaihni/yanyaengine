use std::sync::Arc;

use parking_lot::RwLock;

use font_kit::{
    hinting::HintingOptions,
    font::Font,
    canvas::{RasterizationOptions, Format, Canvas}
};

use pathfinder_geometry::{
    transform2d::Transform2F,
    vector::{Vector2I, Vector2F}
};

use crate::{
    Object,
    ObjectFactory,
    TextInfo,
    ObjectInfo,
    text_factory::FontsContainer,
    transform::{TransformContainer, Transform},
    game_object::*,
    object::{
        resource_uploader::ResourceUploader,
        model::Model,
        texture::{Texture, Color, SimpleImage}
    }
};


pub struct FontsPicker<'a>
{
    font_textures: &'a mut FontsContainer,
    current: usize
}

impl<'a> FontsPicker<'a>
{
    pub fn new(font_textures: &'a mut FontsContainer) -> Self
    {
        Self{
            font_textures,
            current: 0
        }
    }

    pub fn current_font(&mut self) -> Option<&mut CharsRasterizer>
    {
        self.font_textures.get_mut(self.current)
    }

    pub fn cycle_next(&mut self, _resource_uploader: &mut ResourceUploader, _c: char)
    {
        self.current += 1;

        // i could do fallback fonts here but im tired
    }

    pub fn reset_cycle(&mut self)
    {
        self.current = 0;
    }
}

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
    height: i32,
    x: i32,
    y: u32
}

impl BoundsCalculator
{
    pub fn new() -> Self
    {
        Self{
            width: 0,
            height: 0,
            x: 0,
            y: 0
        }
    }

    pub fn process_character(&mut self, info: BoundsInfo) -> i32
    {
        self.width = self.x + info.origin.x + info.width as i32;
        self.height = self.height.max(info.origin.y + info.height as i32);

        let this_x = self.x + info.origin.x;

        self.x += info.advance;

        this_x
    }
}

#[derive(Debug)]
pub struct GlyphInfo
{
    pub offset: OriginOffset,
    pub width: u32,
    pub height: u32
}

pub struct TextObject
{
    pub object: Option<Object>,
    pub aspect: f32
}

impl TextObject
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        object_factory: &ObjectFactory,
        info: TextInfo,
        font_textures: &mut FontsContainer
    ) -> Self
    {
        let mut full_bounds = BoundsCalculator::new();

        let mut fonts_picker = FontsPicker::new(font_textures);

        let current_font = fonts_picker.current_font().expect("must have a font");

        let positions: Vec<_> = info.text.chars().map(|c|
        {
            Self::with_font(
                current_font,
                &mut full_bounds,
                info.font_size,
                c
            ).0
        }).collect();

        let aspect = full_bounds.width as f32 / full_bounds.height as f32;

        let width = full_bounds.width;
        let height = full_bounds.height;

        if width == 0 || height == 0
        {
            return Self{
                object: None,
                aspect: 1.0
            };
        }

        let mut text_canvas = Canvas::new(
            Vector2I::new(width as i32, height as i32),
            Format::A8
        );

        positions.into_iter().zip(info.text.chars()).for_each(|(char_x, c)|
        {
            let is_empty = false;//bounds.width == 0.0 || bounds.height == 0.0;

            if !is_empty
            {
                current_font.render_glyph(
                    &mut text_canvas,
                    info.font_size,
                    char_x,
                    c
                );
            }
        });

        let (x, y) = if aspect < 1.0
        {
            (aspect, 1.0)
        } else
        {
            (1.0, aspect.recip())
        };

        let object = object_factory.create(ObjectInfo{
            model: Arc::new(RwLock::new(Model::rectangle(x, y))),
            texture: Self::canvas_to_texture(resource_uploader, text_canvas),
            transform: info.transform
        });

        Self{object: Some(object), aspect}
    }

    fn canvas_to_texture(
        resource_uploader: &mut ResourceUploader,
        canvas: Canvas
    ) -> Arc<RwLock<Texture>>
    {
        let colors = canvas.pixels.into_iter().map(|value|
        {
            Color::new(u8::MAX, u8::MAX, u8::MAX, value)
        }).collect::<Vec<_>>();

        let image = SimpleImage::new(colors, canvas.size.x() as usize, canvas.size.y() as usize);
        let texture = Texture::new(resource_uploader, image.into());

        Arc::new(RwLock::new(texture))
    }

    fn with_font(
        rasterizer: &mut CharsRasterizer,
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

    pub fn advance(&self, c: char) -> f32
    {
        const DEFAULT_ADVANCE: f32 = 0.0;

        let id = match self.font.glyph_for_char(c)
        {
            Some(id) => id,
            None =>
            {
                eprintln!("couldnt get the advance of {c}, returning {DEFAULT_ADVANCE}");
                return DEFAULT_ADVANCE
            }
        };
        
        let units_per_em = self.font.metrics().units_per_em;

        let advance = match self.font.advance(id)
        {
            Ok(id) => id,
            Err(err) =>
            {
                eprintln!("couldnt get the advance of {c} ({err}), returning {DEFAULT_ADVANCE}");
                return DEFAULT_ADVANCE
            }
        };

        advance.x() / units_per_em as f32
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
        font_size: u32,
        char_x: i32,
        c: char
    ) -> Option<()>
    {
        let small = self.render_small(font_size, canvas.size.y(), c)?;

        let start_x = char_x.max(0) as usize;

        let big_width = canvas.size.x() as usize;

        let width = small.size.x() as usize;
        let height = small.size.y() as usize;

        for y in 0..height
        {
            for x in 0..width
            {
                let offset_x = start_x + x;
                if offset_x >= big_width
                {
                    continue;
                }

                let small_y = y * width;
                let big_y = y * big_width as usize;

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

        let hinting = HintingOptions::Full(point_size);
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
