use std::{
    collections::{HashMap, hash_map::Entry},
    sync::Arc
};

use parking_lot::RwLock;

use nalgebra::Matrix4;

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
    ObjectInfo,
    ObjectFactory,
    TextInfo,
    text_factory::FontsContainer,
    transform::Transform,
    game_object::*,
    object::{
        resource_uploader::ResourceUploader,
        model::Model,
        texture::{Texture, Color, SimpleImage, RgbaImage}
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

    pub fn current_font(&mut self) -> Option<&mut CharsCreator>
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

#[derive(Debug, Clone)]
pub struct TextBounds
{
    pub right: f32,
    pub top: f32,
    pub bottom: f32
}

#[derive(Debug)]
struct BoundsInfo
{
    origin: OriginOffset,
    width: f32,
    height: f32,
    advance: f32
}

#[allow(dead_code)]
struct BoundsCalculator
{
    first_character: bool,
    bounds: TextBounds,
    x: f32,
    y: f32
}

impl BoundsCalculator
{
    pub fn new() -> Self
    {
        let bounds = TextBounds{
            right: 0.0,
            top: 0.0,
            bottom: 0.0
        };

        Self{first_character: true, bounds, x: 0.0, y: 0.0}
    }

    pub fn process_character(&mut self, info: BoundsInfo)
    {
        let this_x = self.x + info.origin.x + info.width;
        let this_y_top = info.origin.y;
        let this_y_bottom = self.y + info.origin.y + info.height;

        if this_x > self.bounds.right || self.first_character
        {
            self.bounds.right = this_x;
        }

        if this_y_top < self.bounds.top || self.first_character
        {
            self.bounds.top = this_y_top;
        }

        if this_y_bottom > self.bounds.bottom || self.first_character
        {
            self.bounds.bottom = this_y_bottom;
        }

        self.x += info.advance;
        self.first_character = false;
    }

    pub fn bounds(self) -> TextBounds
    {
        self.bounds
    }
}

#[derive(Debug)]
pub struct GlyphInfo
{
    pub offset: OriginOffset,
    pub width: f32,
    pub height: f32
}

pub struct TextObject
{
    objects: Vec<Object>,
    transform: Transform,
    bounds: TextBounds
}

impl TextObject
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        info: TextInfo,
        font_textures: &mut FontsContainer
    ) -> Self
    {
        let output_transform = info.transform.clone();

        let mut bounds_calculator = BoundsCalculator::new();

        let mut transform = info.transform;

        let mut text = info.text.chars().peekable();

        let mut fonts_picker = FontsPicker::new(font_textures);

        let mut objects = Vec::new();
        while let Some(&c) = text.peek()
        {
            let current_font = match fonts_picker.current_font()
            {
                Some(current_font) => current_font,
                None =>
                {
                    eprintln!("cant find any fonts to render {c}, skipping it");

                    fonts_picker.reset_cycle();
                    text.next();

                    continue;
                }
            };

            let object = Self::with_font(
                resource_uploader,
                current_font,
                &mut bounds_calculator,
                &mut transform,
                info.projection_view,
                c
            );

            let object = match object
            {
                Some(object) => object,
                None =>
                {
                    // cant find the character in the font, try next font
                    fonts_picker.cycle_next(resource_uploader, c);

                    continue;
                }
            };

            fonts_picker.reset_cycle();

            text.next();

            objects.push(object);
        }

        Self{objects, transform: output_transform, bounds: bounds_calculator.bounds()}
    }

    pub fn transform(&self) -> Transform
    {
        self.transform.clone()
    }

    pub fn bounds(&self) -> TextBounds
    {
        TextBounds{
            right: self.bounds.right * self.transform.scale.x,
            top: self.bounds.top * self.transform.scale.y,
            bottom: self.bounds.bottom * self.transform.scale.y
        }
    }

    fn with_font(
        resource_uploader: &mut ResourceUploader,
        font_texture: &mut CharsCreator,
        bounds_calculator: &mut BoundsCalculator,
        original_transform: &mut Transform,
        projection_view: Matrix4<f32>,
        c: char
    ) -> Option<Object>
    {
        let GlyphInfo{offset, width, height} = font_texture.glyph_info(c);

        let mut transform = original_transform.clone();
        transform.position.y += offset.y * transform.scale.y;
        transform.position.x += offset.x * transform.scale.x;

        let object = font_texture.create_char(
            resource_uploader,
            transform,
            projection_view,
            c
        )?;

        let advance = font_texture.advance(c);
        original_transform.position.x += advance * original_transform.scale.x;

        let info = BoundsInfo{
            origin: offset,
            width,
            height,
            advance
        };

        bounds_calculator.process_character(info);

        Some(object)
    }
}

impl GameObject for TextObject
{
    fn update_buffers(&mut self, info: &mut UpdateBuffersInfo)
    {
        self.objects.iter_mut().for_each(|object| object.update_buffers(info));
    }

    fn draw(&self, info: &mut DrawInfo)
    {
        self.objects.iter().for_each(|object| object.draw(info));
    }
}

const ASCII_START: u8 = 0x20;
const ASCII_END: u8 = 0x7e;

// adding 1 cuz inclusive
const CHARS_AMOUNT: u8 = ASCII_END - ASCII_START + 1;

#[allow(dead_code)]
#[derive(Debug)]
pub struct OriginOffset
{
    pub x: f32,
    pub y: f32
}

struct CharsRasterizer
{
    font: Font,
    font_size: u32
}

impl CharsRasterizer
{
    pub fn new(font: Font, font_size: u32) -> Self
    {
        Self{font, font_size}
    }

    #[allow(dead_code)]
    pub fn font_size(&self) -> u32
    {
        self.font_size
    }

    pub fn ascii_charmap(&self, resource_uploader: &mut ResourceUploader) -> Arc<RwLock<Texture>>
    {
        let total_width = self.font_size * CHARS_AMOUNT as u32;
        let total_height = self.font_size;

        let default_background = Color::new(u8::MAX, u8::MAX, u8::MAX, 0);

        let combined_texture =
            vec![default_background; (total_width * total_height) as usize];
    
        let mut combined_texture = SimpleImage::new(
            combined_texture,
            total_width as usize,
            total_height as usize
        );

        (ASCII_START..=ASCII_END).enumerate().for_each(|(i, c)|
        {
            let c = char::from_u32(c as u32).expect("char must be in valid ascii range");

            let glyph_image = self.glyph_image(c)
                .expect("default font must contain all ascii characters");

            let x = i * self.font_size as usize;
            let y = 0;

            combined_texture.blit(&glyph_image, x, y);
        });

        Self::image_to_texture(resource_uploader, combined_texture)
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
                eprintln!("couldnt get the advance of {c} ({err:?}), returning {DEFAULT_ADVANCE}");
                return DEFAULT_ADVANCE
            }
        };
        let advance = advance.x() / units_per_em as f32;

        advance
    }

    pub fn non_ascii_texture(
        &self,
        resource_uploader: &mut ResourceUploader,
        c: char
    ) -> Option<Arc<RwLock<Texture>>>
    {
        match self.glyph_image(c)
        {
            Some(image) => Some(Self::image_to_texture(resource_uploader, image)),
            None => None
        }
    }

    fn image_to_texture(
        resource_uploader: &mut ResourceUploader,
        image: SimpleImage
    ) -> Arc<RwLock<Texture>>
    {
        let image = RgbaImage::from(image);
        let texture = Texture::new(resource_uploader, image);

        Arc::new(RwLock::new(texture))
    }

    fn glyph_info(&self, c: char) -> GlyphInfo
    {
        let id = match self.font.glyph_for_char(c)
        {
            Some(id) => id,
            None =>
            {
                eprintln!("couldnt get the offset of {c}");
                return GlyphInfo{
                    offset: OriginOffset{
                        x: 0.0,
                        y: 0.0
                    },
                    width: 0.0,
                    height: 0.0
                };
            }
        };

        let bounds = self.font.raster_bounds(
            id,
            self.font_size as f32,
            Transform2F::from_translation(Vector2F::new(0.0, 0.0)),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa
        ).unwrap();

        let font_size = self.font_size as f32;
        let offset = OriginOffset{
            x: (self.font_size as i32 + bounds.origin().x()) as f32 / font_size - 1.0,
            y: (self.font_size as i32 + bounds.origin().y()) as f32 / font_size
        };

        GlyphInfo{
            offset,
            width: bounds.size().x() as f32 / font_size,
            height: bounds.size().y() as f32 / font_size
        }
    }

    fn glyph_image(&self, c: char) -> Option<SimpleImage>
    {
        let id = self.font.glyph_for_char(c)?;

        let mut canvas = Canvas::new(Vector2I::splat(self.font_size as i32), Format::A8);

        let point_size = self.font_size as f32;

        let bounds = self.font.raster_bounds(
            id,
            point_size,
            Transform2F::from_translation(Vector2F::new(0.0, 0.0)),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa
        ).ok()?;

        let offset = Vector2F::new(
            -(bounds.origin().x() as f32),
            -(bounds.origin().y() as f32)
        );

        self.font.rasterize_glyph(
            &mut canvas,
            id,
            point_size,
            Transform2F::from_translation(offset),
            HintingOptions::None,
            RasterizationOptions::GrayscaleAa
        ).ok()?;

        let colors = canvas.pixels.into_iter().map(|value|
        {
            Color::new(u8::MAX, u8::MAX, u8::MAX, value)
        }).collect::<Vec<_>>();

        let image = SimpleImage::new(colors, self.font_size as usize, self.font_size as usize);

        Some(image)
    }
}

struct CharInfo
{
    pub model: Arc<RwLock<Model>>,
    pub texture: Arc<RwLock<Texture>>
}

pub struct CharsCreator
{
    ascii_charmap: Arc<RwLock<Texture>>,
    non_ascii_textures: HashMap<char, Arc<RwLock<Texture>>>,
    rasterizer: CharsRasterizer,
    object_factory: Arc<ObjectFactory>
}

impl CharsCreator
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        object_factory: Arc<ObjectFactory>,
        font: Font
    ) -> Self
    {
        let rasterizer = CharsRasterizer::new(font, 32);

        let ascii_charmap = rasterizer.ascii_charmap(resource_uploader);
        let non_ascii_textures = HashMap::new();

        Self{ascii_charmap, non_ascii_textures, rasterizer, object_factory}
    }

    pub fn glyph_info(&self, c: char) -> GlyphInfo
    {
        self.rasterizer.glyph_info(c)
    }

    pub fn advance(&self, c: char) -> f32
    {
        self.rasterizer.advance(c)
    }

    fn is_visible_ascii(c: char) -> bool
    {
        (ASCII_START as u32..=ASCII_END as u32).contains(&(c as u32))
    }

    fn ascii_char_info(&self, c: char) -> CharInfo
    {
        let model_width = 1.0;
        let model_height = 1.0;

        let vertices = vec![
            [-model_width / 2.0, -model_height / 2.0, 0.0],
            [-model_width / 2.0, model_height / 2.0, 0.0],
            [model_width / 2.0, -model_height / 2.0, 0.0],
            [model_width / 2.0, -model_height / 2.0, 0.0],
            [-model_width / 2.0, model_height / 2.0, 0.0],
            [model_width / 2.0, model_height / 2.0, 0.0]
        ];

        let c = c as u8 - ASCII_START;

        let uv_width = 1.0 / CHARS_AMOUNT as f32;
        let uv_height = 1.0;
        
        let uv_start_x = c as f32 / CHARS_AMOUNT as f32;
        let uv_start_y = 0.0;

        let uvs = vec![
            [uv_start_x, uv_start_y],
            [uv_start_x, uv_start_y + uv_height],
            [uv_start_x + uv_width, uv_start_y],
            [uv_start_x + uv_width, uv_start_y],
            [uv_start_x, uv_start_y + uv_height],
            [uv_start_x + uv_width, uv_start_y + uv_height]
        ];

        let model = Model{
            uvs,
            vertices
        };

        let model = Arc::new(RwLock::new(model));

        CharInfo{
            model,
            texture: self.ascii_charmap.clone()
        }
    }

    fn non_ascii_char_info(
        &mut self,
        resource_uploader: &mut ResourceUploader,
        c: char
    ) -> Option<CharInfo>
    {
        let model = Arc::new(RwLock::new(Model::square(1.0)));

        let texture = match self.non_ascii_textures.entry(c)
        {
            Entry::Occupied(texture) => texture.get().clone(),
            Entry::Vacant(entry) =>
            {
                let texture = self.rasterizer.non_ascii_texture(resource_uploader, c)?;

                entry.insert(texture).clone()
            }
        };

        Some(CharInfo{
            model,
            texture
        })
    }

    fn char_info(&mut self, resource_uploader: &mut ResourceUploader, c: char) -> Option<CharInfo>
    {
        if Self::is_visible_ascii(c)
        {
            Some(self.ascii_char_info(c))
        } else
        {
            self.non_ascii_char_info(resource_uploader, c)
        }
    }

    pub fn create_char(
        &mut self,
        resource_uploader: &mut ResourceUploader,
        transform: Transform,
        projection_view: Matrix4<f32>,
        c: char
    ) -> Option<Object>
    {
        let CharInfo{model, texture} = self.char_info(resource_uploader, c)?;

        let object_info = ObjectInfo{
            model,
            texture,
            transform,
            projection_view
        };

        Some(self.object_factory.create(object_info))    
    }
}

