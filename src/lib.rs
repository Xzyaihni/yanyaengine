#![allow(clippy::suspicious_else_formatting)]
#![allow(clippy::match_like_matches_macro)]
#![allow(clippy::new_without_default)]

use std::{
    fmt::Display,
    marker::PhantomData,
    path::{Path, PathBuf},
    sync::Arc
};

use vulkano::{
    VulkanLibrary,
    buffer::subbuffer::BufferContents,
    swapchain::Surface,
    pipeline::{
        PipelineLayout,
        PipelineShaderStageCreateInfo,
        graphics::{
            vertex_input::{VertexBufferDescription, Vertex},
            depth_stencil::{DepthState, StencilState}
        },
        layout::PipelineDescriptorSetLayoutCreateInfo
    },
    shader::{EntryPoint, ShaderModule, SpecializedShaderModule},
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
pub use window::{Rendering, PipelineInfo};

use game_object::*;

pub use object::{
    Object,
    game_object,
    resource_uploader::ResourceUploader
};

pub use solid_object::SolidObject;

pub use occluding_plane::OccludingPlane;

pub use text_object::{TextAlign, VerticalAlign, HorizontalAlign, TextObject};
pub use text_factory::{TextInfo, TextCreateInfo, FontStyle, FontsContainer};

pub use nalgebra::Vector3;
pub use winit::{
    keyboard::{PhysicalKey, Key, KeyCode, NamedKey},
    event::{ElementState, MouseButton}
};

pub use transform::{
    Transform,
    TransformContainer,
    OnTransformCallback
};

pub use allocators::UniformLocation;

pub use object_factory::{ObjectFactory, ObjectInfo};
pub use assets::*;

pub use control::{KeyCodeNamed, Control};

mod control;

pub mod allocators;

pub mod occluding_plane;
pub mod object;
pub mod solid_object;
pub mod camera;
pub mod transform;

mod object_factory;
pub mod text_factory;
pub mod assets;
pub mod engine;
pub mod text_object;

mod window;


pub const WINDING_MATTERS: bool = false;

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

#[derive(BufferContents, Vertex, Debug, Clone, Copy)]
#[repr(C)]
struct SimpleVertex
{
    #[format(R32G32B32A32_SFLOAT)]
    pub position: [f32; 4]
}

pub trait YanyaApp
where
    Self: Sized
{
    type AppInfo: Default;

    fn init(info: InitPartialInfo, app_info: Self::AppInfo) -> Self;

    fn input(&mut self, _control: Control) {}

    fn mouse_move(&mut self, _position: (f64, f64)) {}

    fn update(&mut self, _info: UpdateBuffersPartialInfo, _dt: f32) {}

    fn draw(&mut self, _info: DrawInfo) {}

    fn resize(&mut self, _aspect: f32) {}

    fn swap_pipelines(&mut self, _resource_uploader: &ResourceUploader) {}
}

pub struct AppOptions
{
    assets_paths: AssetsPaths,
    shaders_query: Option<ShadersQuery>
}

impl Default for AppOptions
{
    fn default() -> Self
    {
        Self{
            assets_paths: AssetsPaths::default(),
            shaders_query: None
        }
    }
}

#[derive(Default)]
pub struct AssetsPaths
{
    textures: Option<PathBuf>,
    models: Option<PathBuf>
}

type WrapperShaderFn = Box<dyn FnOnce(Arc<Device>) -> EntryPoint>;

pub trait ShaderWrappable
{
    fn entry_point(
        self,
        name: &str,
        device: Arc<Device>
    ) -> Option<EntryPoint>;
}

pub trait EntryPointable
{
    fn entry_point(self, name: &str) -> Option<EntryPoint>;
}

impl EntryPointable for Arc<SpecializedShaderModule>
{
    fn entry_point(self, name: &str) -> Option<EntryPoint>
    {
        SpecializedShaderModule::entry_point(&self, name)
    }
}

impl EntryPointable for Arc<ShaderModule>
{
    fn entry_point(self, name: &str) -> Option<EntryPoint>
    {
        ShaderModule::entry_point(&self, name)
    }
}

impl<T, E, F> ShaderWrappable for F
where
    E: Display,
    T: EntryPointable,
    F: FnOnce(Arc<Device>) -> Result<T, E>
{
    fn entry_point(
        self,
        name: &str,
        device: Arc<Device>
    ) -> Option<EntryPoint>
    {
        let err_and_quit = |err|
        {
            panic!("error loading {} shader: {}", name, err)
        };

        (self)(device).unwrap_or_else(err_and_quit).entry_point("main")
    }
}

#[derive(Clone)]
pub struct ShadersGroup<VT, FT=VT>
{
    vertex: VT,
    fragment: FT
}

impl<VT, FT> ShadersGroup<VT, FT>
{
    pub fn new_raw(vertex: VT, fragment: FT) -> Self
    {
        Self{vertex, fragment}
    }
}

impl ShadersGroup<WrapperShaderFn>
{
    pub fn new<A: ShaderWrappable + 'static, B: ShaderWrappable + 'static>(
        vertex: A,
        fragment: B
    ) -> Self
    {
        Self{
            vertex: Box::new(|device| vertex.entry_point("main", device).unwrap()),
            fragment: Box::new(|device| fragment.entry_point("main", device).unwrap())
        }
    }

    pub fn load(self, device: Arc<Device>) -> ShadersGroup<EntryPoint>
    {
        ShadersGroup{
            vertex: (self.vertex)(device.clone()),
            fragment: (self.fragment)(device),
        }
    }
}

impl ShadersGroup<EntryPoint>
{
    pub fn stages(self) -> [PipelineShaderStageCreateInfo; 2]
    {
        [
            PipelineShaderStageCreateInfo::new(self.vertex),
            PipelineShaderStageCreateInfo::new(self.fragment)
        ]
    }
}

pub struct Shader
{
    pub shader: ShadersGroup<WrapperShaderFn>,
    pub per_vertex: Option<VertexBufferDescription>,
    pub depth: Option<DepthState>,
    pub stencil: Option<StencilState>
}

impl Default for Shader
{
    fn default() -> Self
    {
        Self{
            shader: ShadersGroup::new(
                default_vertex::load,
                default_fragment::load
            ),
            per_vertex: None,
            depth: None,
            stencil: None
        }
    }
}

pub struct ShadersContainer
{
    shaders: Vec<Shader>
}

impl IntoIterator for ShadersContainer
{
    type IntoIter = <Vec<Shader> as IntoIterator>::IntoIter;
    type Item = Shader;

    fn into_iter(self) -> Self::IntoIter
    {
        self.shaders.into_iter()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ShaderId(usize);

impl ShaderId
{
    pub fn into_always_query(self) -> ShadersQuery
    {
        Box::new(move |_| self)
    }

    pub fn get_raw(&self) -> usize
    {
        self.0
    }
}

impl ShadersContainer
{
    pub fn new() -> Self
    {
        Self{shaders: Vec::new()}
    }

    pub fn push(&mut self, value: Shader) -> ShaderId
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

pub struct AppBuilder<UserApp: YanyaApp, T>
{
    instance: Arc<Instance>,
    window_builder: WindowBuilder,
    event_loop: EventLoop<()>,
    shaders: ShadersContainer,
    options: AppOptions,
    app_init: Option<UserApp::AppInfo>,
    rendering: Rendering<T>,
    _user_app: PhantomData<UserApp>
}

impl<UserApp: YanyaApp + 'static, T> AppBuilder<UserApp, T>
{
    pub fn with_title(mut self, title: &str) -> Self
    {
        self.window_builder = self.window_builder.with_title(title)
            .with_active(true);

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

    pub fn with_app_init(mut self, app_init: UserApp::AppInfo) -> Self
    {
        self.app_init = Some(app_init);

        self
    }

    pub fn with_clear_color(mut self, color: [f32; 3]) -> Self
    {
        self.rendering.clear[0] = Some([color[0], color[1], color[2], 1.0].into());

        self
    }

    pub fn with_rendering<U>(self, rendering: Rendering<U>) -> AppBuilder<UserApp, U>
    {
        AppBuilder{
            instance: self.instance,
            window_builder: self.window_builder,
            event_loop: self.event_loop,
            shaders: self.shaders,
            options: self.options,
            app_init: self.app_init,
            rendering,
            _user_app: PhantomData
        }
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

    pub fn with_shaders(
        mut self,
        shaders: ShadersContainer,
        shaders_query: ShadersQuery
    ) -> Self
    {
        self.shaders = shaders;
        self.options.shaders_query = Some(shaders_query);

        self
    }

    pub fn run(mut self)
    where
        T: Clone
    {
        if self.shaders.is_empty()
        {
            // load default shaders
            let id = self.shaders.push(Shader::default());

            self.options.shaders_query = Some(Box::new(move |_| id));
        }

        let window = Arc::new(self.window_builder.build(&self.event_loop).unwrap());

        let surface = Surface::from_window(self.instance.clone(), window)
            .unwrap();

        let (physical_device, (device, queues)) =
            Self::create_device(surface.clone(), self.instance);

        let pipeline_infos = self.shaders.into_iter().enumerate().map(|(index, shader_item)|
        {
            let shader = shader_item.shader.load(device.clone());

            let stages = ShadersGroup::from(shader.clone()).stages();

            let info = PipelineDescriptorSetLayoutCreateInfo::from_stages(&stages)
                .into_pipeline_layout_create_info(device.clone())
                .unwrap();

            let layout = PipelineLayout::new(device.clone(), info).unwrap();

            let per_vertex = shader_item.per_vertex.unwrap_or_else(||
            {
                panic!("per_vertex must be provided for shader #{index}")
            });

            PipelineCreateInfo{
                stages: stages.into(),
                shaders: shader,
                per_vertex,
                layout,
                depth: shader_item.depth,
                stencil: shader_item.stencil
            }
        }).collect();

        let graphics_info = GraphicsInfo{
            surface,
            physical_device,
            device,
            pipeline_infos,
            queues: queues.collect(),
            rendering: self.rendering
        };

        window::run::<UserApp, T>(
            graphics_info,
            self.event_loop,
            self.options,
            self.app_init.unwrap_or_default()
        );
    }

    fn get_physical(
        surface: Arc<Surface>,
        instance: Arc<Instance>,
        device_extensions: &DeviceExtensions
    ) -> (Arc<PhysicalDevice>, u32)
    {
        instance.enumerate_physical_devices()
            .expect("no devices that support vulkan found :(")
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
            }).expect("no viable device for rendering :(")
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
    pub fn new() -> AppBuilder<UserApp, ()>
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
            app_init: None,
            rendering: Rendering::new_default([0.0, 0.0, 0.0, 1.0].into()),
            _user_app: PhantomData
        }
    }
}
