use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc
};

use vulkano::{
    VulkanLibrary,
    format::ClearValue,
    swapchain::Surface,
    device::{
        Device,
        DeviceCreateInfo,
        DeviceExtensions,
        Queue,
        QueueFlags,
        QueueCreateInfo,
        physical::{
            PhysicalDevice,
            PhysicalDeviceType
        }
    },
    instance::{Instance, InstanceCreateInfo}
};

use vulkano_win::VkSurfaceBuild;

use winit::{
    window::{Icon, WindowBuilder},
    event_loop::{DeviceEventFilter, EventLoop}
};

use window::GraphicsInfo;

use game_object::*;

pub use object::{
    Object,
    game_object,
    object_allocator::ObjectAllocator,
    resource_uploader::PipelineInfo
};

pub use text_object::TextObject;
pub use text_factory::TextInfo;

pub use nalgebra::Vector3;

pub use object_factory::{ObjectFactory, ObjectInfo};
pub use assets::*;

pub use control::Control;

mod control;

pub mod object;
pub mod camera;
pub mod transform;

mod object_factory;
mod text_factory;
pub mod assets;
pub mod engine;
pub mod text_object;

mod window;


pub trait YanyaApp
where
    Self: Sized
{
    fn init<'a>(info: InitPartialInfo<'a>) -> Self;

    fn input(&mut self, _control: Control) {}

    fn mouse_move(&mut self, _position: (f64, f64)) {}

    fn update(&mut self, _dt: f32) {}

    fn draw<'a>(&mut self, _info: DrawInfo<'a>) {}

    fn resize(&mut self, _aspect: f32) {}

    fn update_buffers<'a>(&mut self, _info: UpdateBuffersPartialInfo<'a>) {}

    fn swap_pipeline(&mut self, _info: PipelineInfo) {}
}

pub struct AppOptions
{
    clear_color: ClearValue,
    assets_paths: AssetsPaths
}

impl Default for AppOptions
{
    fn default() -> Self
    {
        Self{
            clear_color: [0.0, 0.0, 0.0, 1.0].into(),
            assets_paths: AssetsPaths::default()
        }
    }
}

pub struct AssetsPaths
{
    textures: Option<PathBuf>,
    models: Option<PathBuf>
}

impl Default for AssetsPaths
{
    fn default() -> Self
    {
        Self{
            textures: None,
            models: None
        }
    }
}

pub struct AppBuilder<UserApp>
{
    instance: Arc<Instance>,
    surface: WindowBuilder,
    options: AppOptions,
    _user_app: PhantomData<UserApp>
}

impl<UserApp: YanyaApp + 'static> AppBuilder<UserApp>
{
    pub fn with_title(mut self, title: &str) -> Self
    {
        self.surface = self.surface.with_title(title);

        self
    }

    pub fn with_icon<P: AsRef<Path>>(mut self, icon_path: P) -> Self
    {
        let texture = image::open(icon_path).unwrap().into_rgba8();
        let (width, height) = (texture.width(), texture.height());

        let icon = Icon::from_rgba(texture.into_vec(), width, height).ok();

        self.surface = self.surface.with_window_icon(icon);

        self
    }

    pub fn with_clear_color(mut self, color: [f32; 3]) -> Self
    {
        self.options.clear_color = [color[0], color[1], color[2], 1.0].into();

        self
    }

    pub fn with_textures_path<P: Into<PathBuf>>(mut self, path: P) -> Self
    {
        self.options.assets_paths.textures = Some(path.into());

        self
    }

    pub fn with_models_path<P: Into<PathBuf>>(mut self, path: P) -> Self
    {
        self.options.assets_paths.models = Some(path.into());

        self
    }

    pub fn run(self)
    {
        let app = App::<UserApp>{
            instance: self.instance,
            surface: self.surface,
            options: self.options,
            _user_app: PhantomData
        };

        app.run()
    }
}

pub struct App<UserApp>
{
    instance: Arc<Instance>,
    surface: WindowBuilder,
    options: AppOptions,
    _user_app: PhantomData<UserApp>
}

impl<UserApp: YanyaApp + 'static> App<UserApp>
{
    pub fn new() -> AppBuilder<UserApp>
    {
        let library = VulkanLibrary::new().expect("nyo vulkan? ;-;");

        let enabled_extensions = vulkano_win::required_extensions(&library);
        let instance = Instance::new(
            library,
            InstanceCreateInfo{
                enabled_extensions,
                ..Default::default()
            }
        ).expect("cant create vulkan instance..");

        let surface = WindowBuilder::new();

        let options = AppOptions::default();

        AppBuilder{instance, surface, options, _user_app: PhantomData}
    }

    pub fn run(self)
    {
        let event_loop = EventLoop::new();
        event_loop.set_device_event_filter(DeviceEventFilter::Unfocused);

        let surface = self.surface.build_vk_surface(&event_loop, self.instance.clone()).unwrap();

        let (physical_device, (device, queues)) =
            Self::create_device(surface.clone(), self.instance);

        let graphics_info = GraphicsInfo{
            surface,
            event_loop,
            physical_device,
            device,
            queues: queues.collect()
        };

        window::run::<UserApp>(graphics_info, self.options);
    }

    fn get_physical(
        surface: Arc<Surface>,
        instance: Arc<Instance>,
        device_extensions: &DeviceExtensions
    ) -> (Arc<PhysicalDevice>, u32)
    {
        instance.enumerate_physical_devices()
            .expect("cant enumerate devices,,,,")
            .filter(|device| device.supported_extensions().contains(device_extensions))
            .filter_map(|device|
            {
                device.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(index, queue)|
                    {
                        queue.queue_flags.contains(QueueFlags::GRAPHICS)
                            && device.surface_support(index as u32, &surface).unwrap_or(false)
                    })
                    .map(|index| (device, index as u32))
            }).min_by_key(|(device, _)|
            {
                match device.properties().device_type
                {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    _ => 4
                }
            }).expect("nyo rendering device...")
    }

    fn create_device(
        surface: Arc<Surface>,
        instance: Arc<Instance>
    ) -> (Arc<PhysicalDevice>, (Arc<Device>, impl Iterator<Item=Arc<Queue>> + ExactSizeIterator))
    {
        let device_extensions = DeviceExtensions{
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (physical_device, queue_family_index) =
            Self::get_physical(surface, instance, &device_extensions);

        eprintln!("using {}", physical_device.properties().device_name);

        (physical_device.clone(), Device::new(
            physical_device,
            DeviceCreateInfo{
                queue_create_infos: vec![QueueCreateInfo{
                    queue_family_index,
                    ..Default::default()
                }],
                enabled_extensions: device_extensions,
                ..Default::default()
            }).expect("couldnt create device...."))
    }
}
