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
    buffer::subbuffer::BufferContents,
    pipeline::{
        PipelineShaderStageCreateInfo,
        graphics::{
            color_blend::AttachmentBlend,
            rasterization::CullMode,
            vertex_input::{VertexBufferDescription, Vertex},
            depth_stencil::{CompareOp, DepthState, StencilState}
        }
    },
    shader::{EntryPoint, ShaderModule, SpecializedShaderModule},
    device::Device
};

use winit::window::{Window, Icon, WindowAttributes};

use window::InfoInit;
pub use window::{Rendering, PipelineInfo};

use game_object::*;

pub use object::{
    Object,
    ObjectVertex,
    game_object,
    resource_uploader::ResourceUploader
};

pub use solid_object::SolidObject;

pub use occluding_plane::{OccluderPoints, OccludingPlane};

pub use text_object::TextObject;
pub use text_factory::{TextInfo, TextBlocks, TextInfoBlock, TextOutline, TextCreateInfo, FontsContainer};

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
pub struct SimpleVertex
{
    #[format(R32G32B32A32_SFLOAT)]
    pub position: [f32; 4]
}

impl From<[f32; 4]> for SimpleVertex
{
    fn from(position: [f32; 4]) -> Self
    {
        Self{position}
    }
}

impl From<([f32; 4], [f32; 2])> for SimpleVertex
{
    fn from((position, _): ([f32; 4], [f32; 2])) -> Self
    {
        Self::from(position)
    }
}

pub trait YanyaApp
where
    Self: Sized
{
    type SetupInfo: Clone;
    type AppInfo: Default;

    fn init(info: InitPartialInfo<Self::SetupInfo>, app_info: Self::AppInfo) -> Self;

    fn input(&mut self, _control: Control) {}

    fn mouse_move(&mut self, _position: (f64, f64)) {}

    fn update(&mut self, _info: UpdateBuffersPartialInfo, _dt: f32) {}

    fn draw(&mut self, _info: DrawInfo) {}

    fn resize(&mut self, _aspect: f32) {}

    fn early_exit(&self) -> bool { false }

    fn swap_pipelines(&mut self, _resource_uploader: &ResourceUploader) {}

    fn render_pass_ended(&mut self, _builder: &mut CommandBuilderType) {}
}

#[derive(Default)]
pub struct AppOptions
{
    assets_paths: AssetsPaths
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
    pub per_vertex: Option<Vec<VertexBufferDescription>>,
    pub depth: Option<DepthState>,
    pub stencil: Option<StencilState>,
    pub cull: CullMode,
    pub blend: Option<AttachmentBlend>,
    pub subpass: u32
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
            stencil: None,
            cull: CullMode::Back,
            blend: Some(AttachmentBlend::alpha()),
            subpass: 0
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

impl Default for ShaderId
{
    fn default() -> Self
    {
        Self(0)
    }
}

impl ShaderId
{
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
    window_attributes: WindowAttributes,
    shaders: ShadersContainer,
    options: AppOptions,
    app_init: Option<UserApp::AppInfo>,
    rendering: Rendering<UserApp, T>,
    _user_app: PhantomData<UserApp>
}

impl<UserApp: YanyaApp + 'static> AppBuilder<UserApp, ()>
{
    pub fn with_clear_color(mut self, color: [f32; 3]) -> Self
    {
        self.rendering = Rendering::new_default([color[0], color[1], color[2], 1.0].into());

        self
    }
}

impl<UserApp: YanyaApp + 'static, T> AppBuilder<UserApp, T>
{
    pub fn with_title(mut self, title: &str) -> Self
    {
        self.window_attributes = self.window_attributes.with_title(title)
            .with_active(true);

        self
    }

    pub fn with_icon<P: AsRef<Path>>(mut self, icon_path: P) -> Self
    {
        let texture = image::open(icon_path).unwrap().into_rgba8();
        let (width, height) = (texture.width(), texture.height());

        let icon = Icon::from_rgba(texture.into_vec(), width, height).ok();

        self.window_attributes = self.window_attributes.with_window_icon(icon);

        self
    }

    pub fn with_app_init(mut self, app_init: UserApp::AppInfo) -> Self
    {
        self.app_init = Some(app_init);

        self
    }

    pub fn with_rendering<U>(self, rendering: Rendering<UserApp, U>) -> AppBuilder<UserApp, U>
    {
        AppBuilder{
            window_attributes: self.window_attributes,
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
        shaders: ShadersContainer
    ) -> Self
    {
        self.shaders = shaders;

        self
    }
}

impl<UserApp: YanyaApp + 'static> AppBuilder<UserApp, UserApp::SetupInfo>
{
    pub fn run(mut self)
    {
        if self.shaders.is_empty()
        {
            // load default shaders
            self.shaders.push(Shader{
                per_vertex: Some(vec![Object::per_vertex()]),
                depth: Some(DepthState{
                    write_enable: false,
                    compare_op: CompareOp::Less
                }),
                ..Default::default()
            });
        }

        window::run::<UserApp>(
            InfoInit{
                window_attributes: self.window_attributes,
                rendering: self.rendering,
                shaders: self.shaders,
                options: self.options,
            },
            self.app_init.unwrap_or_default()
        );
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
        AppBuilder{
            window_attributes: Window::default_attributes(),
            shaders: ShadersContainer::new(),
            options: AppOptions::default(),
            app_init: None,
            rendering: Rendering::new_default([0.0, 0.0, 0.0, 1.0].into()),
            _user_app: PhantomData
        }
    }
}
