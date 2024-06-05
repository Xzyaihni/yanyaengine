use std::{
    fmt,
    path::Path,
    sync::Arc
};

use nalgebra::Vector2;

use vulkano::{
    format::Format,
    buffer::{Buffer, BufferUsage, BufferCreateInfo},
    command_buffer::CopyBufferToImageInfo,
    memory::allocator::{MemoryTypeFilter, AllocationCreateInfo},
    image::{
        max_mip_levels,
        Image,
        ImageType,
        ImageUsage,
        ImageCreateInfo,
        view::ImageView
    },
    descriptor_set::{
        PersistentDescriptorSet,
        WriteDescriptorSet
    }
};

use image::{
    ColorType,
    DynamicImage,
    error::ImageError
};

use crate::UniformLocation;
use super::resource_uploader::{PipelineInfo, ResourceUploader};


#[derive(Debug, Clone, Copy)]
pub struct Color
{
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8
}

impl Color
{
    pub fn new(r: u8, g: u8, b: u8, a: u8) -> Self
    {
        Self{r, g, b, a}
    }

    pub fn blend(self, other: Self) -> Self
    {
        if self.a == 0
        {
            return other;
        } else if other.a == 0
        {
            return self;
        }

        let to_f = |x|
        {
            x as f32 / 255.0
        };

        let from_f = |x|
        {
            (x * 255.0) as u8
        };

        // or u could express this as lerp(self.alpha, 1.0, other.alpha)
        let alpha = (to_f(other.a) + to_f(self.a) * (1.0 - to_f(other.a))).clamp(0.0, 1.0);

        fn lerp(a: f32, b: f32, t: f32) -> f32
        {
            a * (1.0 - t) + b * t
        }

        let mix = |a, b|
        {
            let mixed = lerp(to_f(a) * to_f(self.a), to_f(b), to_f(other.a)) / alpha;

            from_f(mixed)
        };

        Self{
            r: mix(self.r, other.r),
            g: mix(self.g, other.g),
            b: mix(self.b, other.b),
            a: from_f(alpha)
        }
    }
}

#[derive(Debug, Clone)]
pub struct SimpleImage
{
    pub colors: Vec<Color>,
    pub width: usize,
    pub height: usize
}

#[allow(dead_code)]
impl SimpleImage
{
    pub fn new(colors: Vec<Color>, width: usize, height: usize) -> Self
    {
        Self{colors,  width, height}
    }

    pub fn load(filepath: impl AsRef<Path>) -> Result<Self, ImageError>
    {
        let image = image::open(filepath)?;

        Self::try_from(image)
    }

    pub fn map<F>(&mut self, mut f: F)
    where
        F: FnMut(Color) -> Color
    {
        self.colors.iter_mut().for_each(|color|
        {
            *color = f(*color);
        });
    }

    pub fn get_pixel(&self, x: usize, y: usize) -> Color
    {
        self.colors[self.index_of(x, y)]
    }

    pub fn set_pixel(&mut self, color: Color, x: usize, y: usize)
    {
        let index = self.index_of(x, y);
        self.colors[index] = color;
    }

    pub fn maybe_set_pixel(&mut self, color: Color, x: usize, y: usize)
    {
        if x >= self.width || y >= self.height
        {
            return;
        }

        let index = self.index_of(x, y);
        self.colors[index] = color;
    }

    pub fn maybe_blend_pixel(&mut self, color: Color, x: usize, y: usize)
    {
        if x >= self.width || y >= self.height
        {
            return;
        }

        let index = self.index_of(x, y);

        self.colors[index] = self.colors[index].blend(color);
    }

    pub fn flipped_horizontal(&self) -> Self
    {
        let mut flipped = self.clone();

        (0..self.height).for_each(|y|
        {
            (0..self.width).for_each(|x|
            {
                let this = self.get_pixel(self.width - x - 1, y);
                flipped.set_pixel(this, x, y);
            });
        });

        flipped
    }

    pub fn blit(&mut self, other: &Self, origin_x: usize, origin_y: usize)
    {
        self.blit_inner(other, origin_x, origin_y, |this, p, x, y|
        {
            this.maybe_set_pixel(p, x, y);
        });
    }

    pub fn blit_blend(&mut self, other: &Self, origin_x: usize, origin_y: usize)
    {
        self.blit_inner(other, origin_x, origin_y, |this, p, x, y|
        {
            this.maybe_blend_pixel(p, x, y);
        });
    }

    fn blit_inner<F>(&mut self, other: &Self, origin_x: usize, origin_y: usize, mut op: F)
    where
        F: FnMut(&mut Self, Color, usize, usize)
    {
        for y in 0..other.height
        {
            for x in 0..other.width
            {
                let other_pixel = other.get_pixel(x, y);

                op(self, other_pixel, origin_x + x, origin_y + y);
            }
        }
    }

    fn index_of(&self, x: usize, y: usize) -> usize
    {
        y * self.width + x
    }
}

impl TryFrom<DynamicImage> for SimpleImage
{
    type Error = ImageError;

    fn try_from(other: DynamicImage) -> Result<Self, Self::Error>
    {
        let width = other.width() as usize;
        let height = other.height() as usize;

        let colors = other.into_rgba8().into_raw().chunks(4).map(|bytes: &[u8]|
        {
            Color::new(bytes[0], bytes[1], bytes[2], bytes[3])
        }).collect();

        Ok(Self{colors, width, height})
    }
}

#[derive(Clone)]
pub struct RgbaImage
{
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32
}

#[allow(dead_code)]
impl RgbaImage
{
    pub fn new(data: Vec<u8>, width: u32, height: u32) -> Self
    {
        Self{data, width, height}
    }

    pub fn load(filepath: impl AsRef<Path>) -> Result<Self, ImageError>
    {
        let image = image::open(filepath)?;

        let width = image.width();
        let height = image.height();

        let data = image.into_rgba8().into_raw();

        Ok(Self{data, width, height})
    }

    pub fn save(&self, filename: impl AsRef<Path>) -> Result<(), ImageError>
    {
        image::save_buffer(filename, &self.data, self.width, self.height, ColorType::Rgba8)
    }
}

impl From<SimpleImage> for RgbaImage
{
    fn from(other: SimpleImage) -> Self
    {
        let data = other.colors.into_iter().flat_map(|color| [color.r, color.g, color.b, color.a])
            .collect();

        Self::new(data, other.width as u32, other.height as u32)
    }
}

impl fmt::Debug for RgbaImage
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("RgbaImage")
            .field("width", &self.width)
            .field("height", &self.height)
            .finish()
    }
}

#[derive(Clone)]
pub struct Texture
{
    view: Arc<ImageView>,
    descriptor_set: Arc<PersistentDescriptorSet>,
    location: UniformLocation
}

impl Texture
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        image: RgbaImage,
        location: UniformLocation
    ) -> Self
    {
        let view = Self::calculate_descriptor_set(resource_uploader, &image);
        let descriptor_set = Self::calculate_persistent_set(
            view.clone(),
            &resource_uploader.pipeline_info,
            location
        );

        Self{view, descriptor_set, location}
    }

    fn calculate_descriptor_set(
        resource_uploader: &mut ResourceUploader,
        image: &RgbaImage
    ) -> Arc<ImageView>
    {
        let buffer = Buffer::from_iter(
            resource_uploader.allocator.clone(),
            BufferCreateInfo{
                usage: BufferUsage::TRANSFER_SRC,
                ..Default::default()
            },
            AllocationCreateInfo{
                memory_type_filter: MemoryTypeFilter::PREFER_HOST
                    | MemoryTypeFilter::HOST_SEQUENTIAL_WRITE,
                ..Default::default()
            },
            image.data.iter().copied()
        ).unwrap();

        let extent = [image.width, image.height, 1];

        let image = Image::new(
            resource_uploader.allocator.clone(),
            ImageCreateInfo{
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent,
                mip_levels: max_mip_levels(extent),
                usage: ImageUsage::SAMPLED | ImageUsage::TRANSFER_DST,
                ..Default::default()
            },
            AllocationCreateInfo::default()
        ).unwrap();

        resource_uploader.builder
            .copy_buffer_to_image(CopyBufferToImageInfo::buffer_image(buffer, image.clone()))
            .unwrap();

        ImageView::new_default(image).unwrap()
    }

    pub fn image(&self) -> &Arc<Image>
    {
        self.view.image()
    }

    pub fn aspect_min(&self) -> Vector2<f32>
    {
        let [x, y, _z] = self.view.image().extent();

        let x = x as f32;
        let y = y as f32;

        let max_size = x.max(y);

        Vector2::new(x, y) / max_size
    }

    pub fn swap_pipeline(&mut self, info: &PipelineInfo)
    {
        self.descriptor_set = Self::calculate_persistent_set(
            self.view.clone(),
            info,
            self.location
        );
    }

    fn calculate_persistent_set(
        view: Arc<ImageView>,
        info: &PipelineInfo,
        location: UniformLocation
    ) -> Arc<PersistentDescriptorSet>
    {
        let descriptor_layout = info.layout.set_layouts().get(location.set as usize)
            .unwrap()
            .clone();

        // TODO change this when im gonna add support for multiple shaders
        PersistentDescriptorSet::new(
            info.allocator,
            descriptor_layout,
            [
                WriteDescriptorSet::image_view_sampler(
                    location.binding, view, info.sampler.clone()
                )
            ],
            []
        ).unwrap()
    }

    pub fn descriptor_set(&self) -> Arc<PersistentDescriptorSet>
    {
        self.descriptor_set.clone()
    }
}

impl fmt::Debug for Texture
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("Texture")
            .finish()
    }
}
