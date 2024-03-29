use std::{
    fmt,
    path::Path,
    ops::Deref,
    sync::Arc
};

use vulkano::{
    format::Format,
    buffer::{Buffer, BufferUsage, BufferCreateInfo},
    command_buffer::CopyBufferToImageInfo,
    memory::allocator::{MemoryTypeFilter, AllocationCreateInfo},
    image::{
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

    pub fn blit(&mut self, other: &Self, origin_x: usize, origin_y: usize)
    {
        for y in 0..other.height
        {
            for x in 0..other.width
            {
                let other_pixel = other.get_pixel(x, y);

                self.set_pixel(other_pixel, origin_x + x, origin_y + y);
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
    image: RgbaImage,
    view: Arc<ImageView>,
    descriptor_set: Arc<PersistentDescriptorSet>
}

impl Texture
{
    pub fn new(
        resource_uploader: &mut ResourceUploader,
        image: RgbaImage
    ) -> Self
    {
        let view = Self::calculate_descriptor_set(resource_uploader, &image);
        let descriptor_set = Self::calculate_persistent_set(
            view.clone(),
            &resource_uploader.pipeline_info
        );

        Self{image, view, descriptor_set}
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

        let image = Image::new(
            resource_uploader.allocator.clone(),
            ImageCreateInfo{
                image_type: ImageType::Dim2d,
                format: Format::R8G8B8A8_UNORM,
                extent: [image.width, image.height, 1],
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

    pub fn swap_pipeline(&mut self, info: &PipelineInfo)
    {
        self.descriptor_set = Self::calculate_persistent_set(self.view.clone(), info);
    }

    fn calculate_persistent_set(
        view: Arc<ImageView>,
        info: &PipelineInfo
    ) -> Arc<PersistentDescriptorSet>
    {
        // TODO change this when im gonna add support for multiple shaders
        PersistentDescriptorSet::new(
            info.allocator,
            info.layout.clone(),
            [
                WriteDescriptorSet::image_view_sampler(
                    0, view, info.sampler.clone()
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

impl Deref for Texture
{
    type Target = RgbaImage;

    fn deref(&self) -> &Self::Target
    {
        &self.image
    }
}

impl fmt::Debug for Texture
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result
    {
        f.debug_struct("Texture")
            .field("image", &self.image)
            .finish()
    }
}
