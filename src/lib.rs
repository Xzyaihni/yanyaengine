#![allow(clippy::suspicious_else_formatting)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::new_without_default)]

use std::{
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc
};

use vulkano::{
    VulkanLibrary,
    Validated,
    VulkanError,
    format::ClearValue,
    swapchain::Surface,
    image::SampleCount,
    pipeline::{
        PipelineLayout,
        PipelineShaderStageCreateInfo,
        layout::PipelineDescriptorSetLayoutCreateInfo
    },
    shader::{EntryPoint, ShaderModule},
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

use winit::{
    window::{Icon, WindowBuilder},
    event_loop::{DeviceEvents, EventLoop}
};

use window::{GraphicsInfo, PipelineCreateInfo};

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
pub use winit::{
    keyboard::{PhysicalKey, KeyCode},
    event::{ElementState, MouseButton}
};

pub use transform::{
    Transform,
    TransformContainer,
    OnTransformCallback
};

pub use object_factory::{ObjectFactory, ObjectInfo};
pub use assets::*;

pub use control::Control;

mod control;

pub mod object;
pub mod camera;
pub mod transform;

mod object_factory;
pub mod text_factory;
pub mod assets;
pub mod engine;
pub mod text_object;

mod window;


mod default_vertex
{
    vulkano_shaders::shader!
    {
        ty: "vertex",
        path: "shaders/default.vert"
    }
}

mod default_fragment
{
    vulkano_shaders::shader!
    {
        ty: "fragment",
        path: "shaders/default.frag"
    }
}

pub trait YanyaApp
where
    Self: Sized
{
    fn init(info: InitPartialInfo) -> Self;

    fn input(&mut self, _control: Control) {}

    fn mouse_move(&mut self, _position: (f64, f64)) {}

    fn update(&mut self, _dt: f32) {}

    fn draw(&mut self, _info: DrawInfo) {}

    fn resize(&mut self, _aspect: f32) {}

    fn update_buffers(&mut self, _info: UpdateBuffersPartialInfo) {}

    fn swap_pipeline(&mut self, _info: PipelineInfo) {}
}

pub struct AppOptions
{
    clear_color: ClearValue,
    assets_paths: AssetsPaths,
    samples: SampleCount
}

impl Default for AppOptions
{
    fn default() -> Self
    {
        Self{
            clear_color: [0.0, 0.0, 0.0, 1.0].into(),
            assets_paths: AssetsPaths::default(),
            samples: SampleCount::Sample2
        }
    }
}

#[derive(Default)]
pub struct AssetsPaths
{
    textures: Option<PathBuf>,
    models: Option<PathBuf>
}

type ShaderLoadResult = Result<Arc<ShaderModule>, Validated<VulkanError>>;

type ShaderFn = fn(Arc<Device>) -> ShaderLoadResult;

#[derive(Clone)]
pub struct ShadersInfo<VT, FT=VT>
{
    vertex: VT,
    fragment: FT
}

impl<VT, FT> ShadersInfo<VT, FT>
{
    pub fn new_raw(vertex: VT, fragment: FT) -> Self
    {
        Self{vertex, fragment}
    }
}

impl ShadersInfo<ShaderFn>
{
    pub fn new(vertex: ShaderFn, fragment: ShaderFn) -> Self
    {
        Self{
            vertex,
            fragment
        }
    }

    pub fn load(self, device: Arc<Device>) -> ShadersInfo<Arc<ShaderModule>>
    {
        let err_and_quit = |name, err|
        {
            panic!("error loading {} shader: {}", name, err)
        };

        let vertex = (self.vertex)(device.clone())
            .unwrap_or_else(|err| err_and_quit("vertex", err));

        let fragment = (self.fragment)(device)
            .unwrap_or_else(|err| err_and_quit("fragment", err));

        ShadersInfo{
            vertex,
            fragment,
        }
    }
}

impl ShadersInfo<EntryPoint>
{
    pub fn stages(self) -> [PipelineShaderStageCreateInfo; 2]
    {
        [
            PipelineShaderStageCreateInfo::new(self.vertex),
            PipelineShaderStageCreateInfo::new(self.fragment)
        ]
    }
}

impl From<&ShadersInfo<Arc<ShaderModule>>> for ShadersInfo<EntryPoint>
{
    fn from(value: &ShadersInfo<Arc<ShaderModule>>) -> Self
    {
        Self{
            vertex: value.vertex_entry(),
            fragment: value.fragment_entry()
        }
    }
}

impl ShadersInfo<Arc<ShaderModule>>
{
    pub fn vertex_entry(&self) -> EntryPoint
    {
        self.vertex.entry_point("main").unwrap()
    }

    pub fn fragment_entry(&self) -> EntryPoint
    {
        self.fragment.entry_point("main").unwrap()
    }
}

pub struct ShadersContainer
{
    shaders: Vec<ShadersInfo<ShaderFn>>
}

impl IntoIterator for ShadersContainer
{
    type IntoIter = <Vec<ShadersInfo<ShaderFn>> as IntoIterator>::IntoIter;
    type Item = ShadersInfo<ShaderFn>;

    fn into_iter(self) -> Self::IntoIter
    {
        self.shaders.into_iter()
    }
}

#[allow(dead_code)]
pub struct ShaderId(usize);

impl ShadersContainer
{
    pub fn new() -> Self
    {
        Self{shaders: Vec::new()}
    }

    pub fn push(&mut self, value: ShadersInfo<ShaderFn>) -> ShaderId
    {
        let id = ShaderId(self.shaders.len());

        self.shaders.push(value);

        id
    }

    pub fn is_empty(&self) -> bool
    {
        self.shaders.is_empty()
    }
}

pub struct AppBuilder<UserApp>
{
    instance: Arc<Instance>,
    window_builder: WindowBuilder,
    event_loop: EventLoop<()>,
    shaders: ShadersContainer,
    options: AppOptions,
    _user_app: PhantomData<UserApp>
}

impl<UserApp: YanyaApp + 'static> AppBuilder<UserApp>
{
    pub fn with_title(mut self, title: &str) -> Self
    {
        self.window_builder = self.window_builder.with_title(title);

        self
    }

    pub fn with_icon<P: AsRef<Path>>(mut self, icon_path: P) -> Self
    {
        let texture = image::open(icon_path).unwrap().into_rgba8();
        let (width, height) = (texture.width(), texture.height());

        let icon = Icon::from_rgba(texture.into_vec(), width, height).ok();

        self.window_builder = self.window_builder.with_window_icon(icon);

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

    pub fn without_multisampling(mut self) -> Self
    {
        self.options.samples = SampleCount::Sample1;

        self
    }

    pub fn with_models_path<P: Into<PathBuf>>(mut self, path: P) -> Self
    {
        self.options.assets_paths.models = Some(path.into());

        self
    }

    pub fn with_shaders(mut self, shaders: ShadersContainer) -> Self
    {
        self.shaders = shaders;

        self
    }

    pub fn run(mut self)
    {
        if self.shaders.is_empty()
        {
            // load default shaders
            self.shaders.push(ShadersInfo::new(default_vertex::load, default_fragment::load));
        }

        let window = Arc::new(self.window_builder.build(&self.event_loop).unwrap());

        let surface = Surface::from_window(self.instance.clone(), window)
            .unwrap();

        let (physical_device, (device, queues)) =
            Self::create_device(surface.clone(), self.instance);

        let pipeline_infos = self.shaders.into_iter().map(|shader_item|
        {
            let shader = shader_item.load(device.clone());

            let stages = ShadersInfo::from(&shader).stages();

            let info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap();

            let layout = PipelineLayout::new(device.clone(), info).unwrap();

            PipelineCreateInfo::new(stages.into(), (&shader).into(), layout)
        }).collect();

        let graphics_info = GraphicsInfo{
            surface,
            event_loop: self.event_loop,
            physical_device,
            device,
            pipeline_infos,
            samples: self.options.samples,
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
    ) -> (Arc<PhysicalDevice>, (Arc<Device>, impl ExactSizeIterator<Item=Arc<Queue>>))
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

pub struct App<UserApp>
{
    _user_app: PhantomData<UserApp>
}

impl<UserApp: YanyaApp + 'static> App<UserApp>
{
    #[allow(clippy::new_ret_no_self)]
    pub fn new() -> AppBuilder<UserApp>
    {
        let library = VulkanLibrary::new().expect("nyo vulkan? ;-;");

        let event_loop = EventLoop::new().unwrap();
        event_loop.listen_device_events(DeviceEvents::WhenFocused);

        let enabled_extensions = Surface::required_extensions(&event_loop);
        let instance = Instance::new(
            library,
            InstanceCreateInfo{
                enabled_extensions,
                ..Default::default()
            }
        ).expect("cant create vulkan instance..");

        AppBuilder{
            instance,
            window_builder: WindowBuilder::new(),
            event_loop,
            shaders: ShadersContainer::new(),
            options: AppOptions::default(),
            _user_app: PhantomData
        }
    }
}
