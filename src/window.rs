use std::{
    time::Instant,
    sync::Arc
};

use vulkano::{
    Validated,
    VulkanError,
    VulkanLibrary,
    format::{Format, NumericFormat, ClearValue},
    memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    shader::EntryPoint,
    sync::{
        GpuFuture,
        future::FenceSignalFuture
    },
    pipeline::{
        Pipeline,
        PipelineLayout,
        GraphicsPipeline,
        PipelineShaderStageCreateInfo,
        DynamicState,
        layout::PipelineDescriptorSetLayoutCreateInfo,
        graphics::{
            GraphicsPipelineCreateInfo,
            multisample::MultisampleState,
            depth_stencil::{DepthStencilState, DepthState, StencilState},
            color_blend::{ColorBlendState, ColorBlendAttachmentState, AttachmentBlend},
            rasterization::{CullMode, RasterizationState},
            input_assembly::InputAssemblyState,
            vertex_input::{VertexBufferDescription, VertexDefinition},
            viewport::{Scissor, Viewport, ViewportState}
        }
    },
    image::{
        ImageUsage,
        Image,
        ImageType,
        ImageCreateInfo,
        view::ImageView,
        sampler::{
            Filter,
            SamplerMipmapMode,
            Sampler,
            SamplerCreateInfo
        }
    },
    swapchain::{
        self,
        ColorSpace,
        Surface,
        SurfaceCapabilities,
        CompositeAlpha,
        PresentFuture,
        Swapchain,
        SwapchainAcquireFuture,
        SwapchainCreateInfo,
        SwapchainPresentInfo
    },
    device::{
        Device,
        DeviceExtensions,
        DeviceCreateInfo,
        QueueCreateInfo,
        QueueFlags,
        Queue,
        physical::{PhysicalDevice, PhysicalDeviceType}
    },
    render_pass::{
        Subpass,
        RenderPass,
        Framebuffer,
        FramebufferCreateInfo
    },
    command_buffer::{
        AutoCommandBufferBuilder,
        PrimaryAutoCommandBuffer,
        CommandBufferExecFuture,
        CommandBufferUsage,
        SubpassContents,
        SubpassBeginInfo,
        RenderPassBeginInfo,
        allocator::{CommandBufferAllocator, StandardCommandBufferAllocator}
    },
    instance::{Instance, InstanceCreateInfo}
};

use winit::{
    application::ApplicationHandler,
    dpi::{PhysicalPosition, PhysicalSize},
    window::{Window, WindowId, WindowAttributes},
    event::{
        WindowEvent,
        MouseScrollDelta
    },
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, DeviceEvents}
};

use crate::{
    YanyaApp,
    AppOptions,
    Control,
    ShadersGroup,
    ShadersContainer,
    engine::Engine,
    game_object::*,
    object::resource_uploader::ResourceUploader
};


pub struct PipelineInfo
{
    pub pipeline: Arc<GraphicsPipeline>,
    pub layout: Arc<PipelineLayout>
}

impl From<Arc<GraphicsPipeline>> for PipelineInfo
{
    fn from(value: Arc<GraphicsPipeline>) -> Self
    {
        Self{
            layout: value.layout().clone(),
            pipeline: value
        }
    }
}

pub struct PipelineCreateInfo
{
    pub stages: Vec<PipelineShaderStageCreateInfo>,
    pub per_vertex: VertexBufferDescription,
    pub shaders: ShadersGroup<EntryPoint>,
    pub layout: Arc<PipelineLayout>,
    pub depth: Option<DepthState>,
    pub stencil: Option<StencilState>
}

pub type AttachmentCreator<T> = Box<dyn Fn(T, Arc<StandardMemoryAllocator>, Arc<ImageView>) -> Vec<Arc<ImageView>>>;
pub type RenderPassCreator<T> = Box<dyn FnOnce(T, Arc<Device>, Format) -> Arc<RenderPass>>;

pub struct Rendering<T>
{
    pub setup: Box<dyn FnOnce(Arc<PhysicalDevice>) -> T>,
    pub attachments: AttachmentCreator<T>,
    pub render_pass: RenderPassCreator<T>,
    pub clear: Vec<Option<ClearValue>>
}

impl Rendering<()>
{
    pub fn new_default(
        clear_color: ClearValue
    ) -> Self
    {
        let attachments = Box::new(|_, allocator: Arc<StandardMemoryAllocator>, view: Arc<ImageView>|
        {
            let depth_image = Image::new(
                allocator,
                ImageCreateInfo{
                    image_type: ImageType::Dim2d,
                    format: Format::D16_UNORM,
                    extent: view.image().extent(),
                    usage: ImageUsage::TRANSIENT_ATTACHMENT | ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default()
            ).unwrap();

            let depth = ImageView::new_default(depth_image).unwrap();

            vec![view, depth]
        });

        let render_pass = Box::new(|_, device, image_format|
        {
            vulkano::single_pass_renderpass!(
                device,
                attachments: {
                    color: {
                        format: image_format,
                        samples: 1,
                        load_op: Clear,
                        store_op: Store
                    },
                    depth: {
                        format: Format::D16_UNORM,
                        samples: 1,
                        load_op: Clear,
                        store_op: DontCare
                    }
                },
                pass: {
                    color: [color],
                    depth_stencil: {depth}
                }
            ).unwrap()
        });

        let clear = vec![Some(clear_color), Some(1.0.into())];

        Self{
            setup: Box::new(|_| {}),
            attachments,
            render_pass,
            clear
        }
    }
}

// just put everything in 1 place who cares lmao
struct RenderInfo<T>
{
    pub device: Arc<Device>,
    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Box<[Arc<Framebuffer>]>,
    pub pipelines: Vec<PipelineInfo>,
    pub viewport: Viewport,
    pub surface: Arc<Surface>,
    pub render_pass: Arc<RenderPass>,
    pub sampler: Arc<Sampler>,
    pub clear_values: Vec<Option<ClearValue>>,
    pipeline_infos: Vec<PipelineCreateInfo>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_allocator: Arc<StandardDescriptorSetAllocator>,
    setup: T,
    attachment_creator: AttachmentCreator<T>
}

impl<T: Clone> RenderInfo<T>
{
    pub fn new(
        info: GraphicsInfo<T>,
        capabilities: SurfaceCapabilities,
        image_format: Format,
        composite_alpha: CompositeAlpha
    ) -> Self
    {
        let device = info.device;
        let surface = info.surface;
        let pipeline_infos = info.pipeline_infos;

        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo{
                mag_filter: Filter::Nearest,
                min_filter: Filter::Linear,
                mipmap_mode: SamplerMipmapMode::Linear,
                ..Default::default()
            }
        ).unwrap();

        let memory_allocator = Arc::new(StandardMemoryAllocator::new_default(device.clone()));

        let dimensions = Self::surface_size_associated(&surface);

        eprintln!("framebuffer format: {image_format:?}");

        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo{
                min_image_count: capabilities.min_image_count.max(2),
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                composite_alpha,
                ..Default::default()
            }
        ).unwrap();

        let setup = (info.rendering.setup)(info.physical_device.clone());
        let render_pass = (info.rendering.render_pass)(setup.clone(), device.clone(), image_format);

        let attachment_creator = info.rendering.attachments;

        let framebuffers = Self::framebuffers(
            memory_allocator.clone(),
            images.into_iter(),
            render_pass.clone(),
            &setup,
            &attachment_creator
        );

        let viewport = Viewport{
            offset: [0.0, 0.0],
            extent: dimensions.into(),
            depth_range: 0.0..=1.0
        };


        let pipelines = Self::generate_pipelines(
            viewport.clone(),
            render_pass.clone(),
            device.clone(),
            &pipeline_infos
        );

        let descriptor_allocator = Arc::new(StandardDescriptorSetAllocator::new(
            device.clone(),
            Default::default()
        ));

        Self{
            device,
            swapchain,
            framebuffers,
            pipelines,
            viewport,
            surface,
            render_pass,
            sampler,
            clear_values: info.rendering.clear,
            pipeline_infos,
            memory_allocator,
            descriptor_allocator,
            setup,
            attachment_creator
        }
    }

    pub fn framebuffers(
        memory_allocator: Arc<StandardMemoryAllocator>,
        images: impl Iterator<Item=Arc<Image>>,
        render_pass: Arc<RenderPass>,
        setup: &T,
        attachments: &AttachmentCreator<T>
    ) -> Box<[Arc<Framebuffer>]>
    {
        images.map(|image|
        {
            let view = ImageView::new_default(image).unwrap();

            let attachments = attachments(setup.clone(), memory_allocator.clone(), view);

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo{
                    attachments,
                    ..Default::default()
                }
            ).unwrap()
        }).collect()
    }

    fn generate_pipeline(
        shader: &PipelineCreateInfo,
        viewport: Viewport,
        subpass: Subpass,
        device: Arc<Device>
    ) -> PipelineInfo
    {
        let mut dynamic_state = foldhash::HashSet::default();
        dynamic_state.insert(DynamicState::Scissor);

        let pipeline = GraphicsPipeline::new(
            device,
            None,
            GraphicsPipelineCreateInfo{
                stages: shader.stages.iter().cloned().collect(),
                vertex_input_state: Some(shader.per_vertex
                    .definition(&shader.shaders.vertex)
                    .unwrap()
                ),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState{
                    viewports: [viewport].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState{
                    cull_mode: CullMode::None,
                    ..Default::default()
                }),
                multisample_state: Some(MultisampleState{
                    rasterization_samples: subpass.num_samples().unwrap(),
                    ..Default::default()
                }),
                color_blend_state: Some(ColorBlendState::with_attachment_states(
                    subpass.num_color_attachments(),
                    ColorBlendAttachmentState{
                        blend: Some(AttachmentBlend::alpha()),
                        ..Default::default()
                    }
                )),
                depth_stencil_state: Some(DepthStencilState{
                    depth: shader.depth,
                    stencil: shader.stencil.clone(),
                    ..Default::default()
                }),
                dynamic_state,
                subpass: Some(subpass.into()),
                ..GraphicsPipelineCreateInfo::layout(shader.layout.clone())
            }
        ).unwrap();

        pipeline.into()
    }

    fn generate_pipelines(
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
        device: Arc<Device>,
        pipeline_infos: &[PipelineCreateInfo]
    ) -> Vec<PipelineInfo>
    {
        let subpass = Subpass::from(render_pass, 0).unwrap();

        pipeline_infos.iter().map(|shader|
        {
            Self::generate_pipeline(
                shader,
                viewport.clone(),
                subpass.clone(),
                device.clone()
            )
        }).collect()
    }

    pub fn resource_uploader<'a>(
        &'a self,
        builder: &'a mut CommandBuilderType
    ) -> ResourceUploader<'a>
    {
        ResourceUploader{
            allocator: self.memory_allocator.clone(),
            descriptor_allocator: self.descriptor_allocator.clone(),
            sampler: self.sampler.clone(),
            builder,
            pipeline_infos: &self.pipelines
        }
    }

    pub fn recreate(
        &mut self,
        redraw_window: bool
    ) -> Result<(), Validated<VulkanError>>
    {
        let dimensions = self.surface_size();

        let (new_swapchain, new_images) = self.swapchain.recreate(SwapchainCreateInfo{
            image_extent: dimensions.into(),
            ..self.swapchain.create_info()
        })?;

        self.swapchain = new_swapchain;
        self.framebuffers = Self::framebuffers(
            self.memory_allocator.clone(),
            new_images.into_iter(),
            self.render_pass.clone(),
            &self.setup,
            &self.attachment_creator
        );

        if redraw_window
        {
            self.viewport.extent = dimensions.into();

            self.pipelines = Self::generate_pipelines(
                self.viewport.clone(),
                self.render_pass.clone(),
                self.device.clone(),
                &self.pipeline_infos
            );
        }

        Ok(())
    }

    pub fn size(&self) -> [f32; 2]
    {
        self.surface_size().into()
    }

    pub fn aspect(&self) -> f32
    {
        let [x, y] = self.size();

        x / y
    }

    pub fn surface_size(&self) -> PhysicalSize<u32>
    {
        Self::surface_size_associated(&self.surface)
    }

    pub fn window(&self) -> &Window
    {
        Self::window_associated(&self.surface)
    }

    fn window_associated(surface: &Arc<Surface>) -> &Window
    {
        surface.object().unwrap().downcast_ref::<Window>().unwrap()
    }

    fn surface_size_associated(surface: &Arc<Surface>) -> PhysicalSize<u32>
    {
        Self::window_associated(surface).inner_size()
    }
}

pub struct GraphicsInfo<T>
{
    pub surface: Arc<Surface>,
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    pub pipeline_infos: Vec<PipelineCreateInfo>,
    pub queues: Vec<Arc<Queue>>,
    pub rendering: Rendering<T>
}

pub type ThisCommandBufferAllocator = Arc<(dyn CommandBufferAllocator + 'static)>;

// stupid code duplication but im lazy wutever
struct HandleEventInfoRaw<T>
{
    command_allocator: ThisCommandBufferAllocator,
    queue: Arc<Queue>,
    fence: FutureType,
    device: Arc<Device>,
    render_info: RenderInfo<T>,
    options: AppOptions
}

struct HandleEventInfo<UserApp, T>
{
    command_allocator: ThisCommandBufferAllocator,
    queue: Arc<Queue>,
    fence: FutureType,
    device: Arc<Device>,
    render_info: RenderInfo<T>,
    options: AppOptions,
    engine: Option<Engine>,
    user_app: Option<UserApp>,
    previous_time: Instant,
    frame_parity: bool,
    initialized: bool,
    recreate_swapchain: bool,
    window_resized: bool
}

impl<UserApp, T> From<HandleEventInfoRaw<T>> for HandleEventInfo<UserApp, T>
{
    fn from(value: HandleEventInfoRaw<T>) -> Self
    {
        Self{
            command_allocator: value.command_allocator,
            queue: value.queue,
            fence: value.fence,
            device: value.device,
            render_info: value.render_info,
            options: value.options,
            engine: None,
            user_app: None,
            previous_time: Instant::now(),
            frame_parity: false,
            initialized: false,
            recreate_swapchain: false,
            window_resized: false
        }
    }
}

pub struct InfoInit<T>
{
    pub window_attributes: WindowAttributes,
    pub rendering: Rendering<T>,
    pub shaders: ShadersContainer,
    pub options: AppOptions
}

impl<T: Clone> InfoInit<T>
{
    fn initialize<UserApp: YanyaApp + 'static>(self, event_loop: &ActiveEventLoop) -> HandleEventInfo<UserApp, T>
    {
        let library = VulkanLibrary::new().expect("nyo vulkan? ;-;");

        let enabled_extensions = Surface::required_extensions(&event_loop).expect("cant get required extensions");
        let instance = Instance::new(
            library,
            InstanceCreateInfo{
                enabled_extensions,
                ..Default::default()
            }
        ).expect("cant create vulkan instance..");

        let window = Arc::new(event_loop.create_window(self.window_attributes).unwrap());

        let surface = Surface::from_window(instance.clone(), window)
            .unwrap();

        let (physical_device, (device, queues)) = create_device(surface.clone(), instance);

        let pipeline_infos = self.shaders.into_iter().enumerate().map(|(index, shader_item)|
        {
            let shader = shader_item.shader.load(device.clone());

            let stages = shader.clone().stages();

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

        let info = GraphicsInfo{
            surface,
            physical_device,
            device,
            pipeline_infos,
            queues: queues.collect(),
            rendering: self.rendering
        };

        let capabilities = info.physical_device
            .surface_capabilities(&info.surface, Default::default())
            .unwrap();

        let composite_alpha =
        {
            let supported = capabilities.supported_composite_alpha;

            let preferred = CompositeAlpha::Opaque;
            let supports_preferred = supported.contains_enum(preferred);

            if supports_preferred
            {
                preferred
            } else
            {
                supported.into_iter().next().unwrap()
            }
        };

        let formats = info.physical_device
            .surface_formats(&info.surface, Default::default())
            .unwrap();

        let image_format = formats.iter().find(|(format, colorspace)|
        {
            format.numeric_format_color() == Some(NumericFormat::SRGB)
                && *colorspace == ColorSpace::SrgbNonLinear
        }).unwrap_or_else(|| &formats[0]).0;

        let device = info.device.clone();
        let queue = info.queues[0].clone();

        let render_info = RenderInfo::new(
            info,
            capabilities,
            image_format,
            composite_alpha
        );

        HandleEventInfo::from(
            HandleEventInfoRaw{
                fence: None,
                command_allocator: Arc::new(StandardCommandBufferAllocator::new(
                    device.clone(),
                    Default::default()
                )),
                queue,
                render_info,
                device,
                options: self.options
            }
        )
    }
}

pub fn run<UserApp: YanyaApp + 'static, T: Clone>(
    info_init: InfoInit<T>,
    app_init: UserApp::AppInfo
)
{
    let mut app: WindowEventHandler<UserApp, UserApp::AppInfo, T> = WindowEventHandler{
        info_init: Some(info_init),
        info: None,
        app_init: Some(app_init)
    };

    let event_loop = EventLoop::new().unwrap();

    event_loop.listen_device_events(DeviceEvents::WhenFocused);
    event_loop.set_control_flow(ControlFlow::Poll);
    event_loop.run_app(&mut app).unwrap();
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

    let (physical_device, queue_family_index) = get_physical(surface, instance, &device_extensions);

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

struct WindowEventHandler<UserApp, Init, T>
{
    info_init: Option<InfoInit<T>>,
    info: Option<HandleEventInfo<UserApp, T>>,
    app_init: Option<Init>
}

impl<UserApp, Init, T> WindowEventHandler<UserApp, Init, T>
{
    fn info(&self) -> &HandleEventInfo<UserApp, T>
    {
        self.info.as_ref().unwrap()
    }

    fn info_mut(&mut self) -> &mut HandleEventInfo<UserApp, T>
    {
        self.info.as_mut().unwrap()
    }
}

impl<UserApp: YanyaApp + 'static, T: Clone> ApplicationHandler for WindowEventHandler<UserApp, UserApp::AppInfo, T>
{
    fn resumed(&mut self, event_loop: &ActiveEventLoop)
    {
        if self.info_init.is_none()
        {
            return;
        }

        self.info = Some(self.info_init.take().unwrap().initialize(event_loop));
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _window_id: WindowId, event: WindowEvent)
    {
        if self.info.is_none()
        {
            return;
        }

        match event
        {
            WindowEvent::CloseRequested =>
            {
                drop(self.info_mut().user_app.take());

                event_loop.exit()
            },
            WindowEvent::RedrawRequested =>
            {
                let [x, y]: [u32; 2] = self.info().render_info.surface_size().into();

                if x == 0 || y == 0
                {
                    return;
                }

                handle_redraw(self.info.as_mut().unwrap(), &mut self.app_init);

                self.info().render_info.window().request_redraw();
            },
            WindowEvent::Resized(_) => self.info_mut().window_resized = true,
            WindowEvent::CursorMoved{position, ..} =>
            {
                if !self.info().initialized
                {
                    return;
                }

                let (width, height): (f64, f64) = self.info().render_info.surface_size().into();

                if width == 0.0 || height == 0.0
                {
                    return;
                }

                let position = ((position.x / width).clamp(0.0, 1.0), (position.y / height).clamp(0.0, 1.0));

                if let Some(app) = self.info_mut().user_app.as_mut()
                {
                    app.mouse_move(position);
                }
            },
            WindowEvent::MouseInput{
                button,
                state,
                ..
            } =>
            {
                if !self.info().initialized
                {
                    return;
                }

                let control = Control::Mouse{button, state};
                if let Some(app) = self.info_mut().user_app.as_mut()
                {
                    app.input(control);
                }
            },
            WindowEvent::MouseWheel{delta, ..} =>
            {
                if !self.info().initialized
                {
                    return;
                }

                let (x, y) = match delta
                {
                    MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64),
                    MouseScrollDelta::PixelDelta(PhysicalPosition{x, y}) => (x, y)
                };

                let control = Control::Scroll{x, y};
                if let Some(app) = self.info_mut().user_app.as_mut()
                {
                    app.input(control);
                }
            },
            WindowEvent::KeyboardInput{event, ..} =>
            {
                if !self.info().initialized
                {
                    return;
                }

                let control = Control::Keyboard{
                    logical: event.logical_key,
                    keycode: event.physical_key,
                    state: event.state
                };

                if let Some(app) = self.info_mut().user_app.as_mut()
                {
                    app.input(control);
                }
            },
            _ => ()
        }
    }
}

fn handle_redraw<UserApp: YanyaApp + 'static, T: Clone>(
    info: &mut HandleEventInfo<UserApp, T>,
    app_init: &mut Option<UserApp::AppInfo>
)
{
    let mut builder = AutoCommandBufferBuilder::primary(
        info.command_allocator.clone(),
        info.queue.queue_family_index(),
        CommandBufferUsage::OneTimeSubmit
    ).unwrap();

    if info.recreate_swapchain || (info.initialized && info.window_resized)
    {
        info.recreate_swapchain = false;

        match info.render_info.recreate(info.window_resized)
        {
            Ok(_) => (),
            Err(e) => panic!("couldnt recreate swapchain ; -; ({e})")
        }

        if !info.initialized
        {
            return;
        }

        let resource_uploader = info.render_info.resource_uploader(&mut builder);
        info.engine.as_mut().unwrap().swap_pipelines(&resource_uploader);
        info.user_app.as_mut().unwrap().swap_pipelines(&resource_uploader);

        if info.window_resized
        {
            info.user_app.as_mut().unwrap().resize(info.render_info.aspect());
        }

        info.window_resized = false;
    }

    builder.set_scissor(0, vec![Scissor::default()].into()).unwrap();

    let acquired =
        match swapchain::acquire_next_image(info.render_info.swapchain.clone(), None)
        {
            Ok(x) => Some(x),
            Err(Validated::Error(VulkanError::OutOfDate)) =>
            {
                None
            },
            Err(e) =>
            {
                let e = match e
                {
                    Validated::Error(x) => format!("{x}"),
                    Validated::ValidationError(x) => format!("error validating {x}")
                };

                panic!("error getting next image: ({e})")
            }
        };

    if let Some((image_index, suboptimal, acquire_future)) = acquired
    {
        if !info.initialized
        {
            info.initialized = true;

            info.engine = Some(Engine::new(
                &info.options.assets_paths,
                info.render_info.resource_uploader(&mut builder),
                info.device.clone(),
                info.options.shaders_query.take().unwrap()
            ));

            info.user_app = {
                let init_info = info.engine
                    .as_mut()
                    .unwrap()
                    .init_partial_info(
                        info.render_info.resource_uploader(&mut builder),
                        info.render_info.size()
                    );

                let app_init = app_init.take().unwrap();
                Some(UserApp::init(init_info, app_init))
            };
        } else if info.user_app.is_none()
        {
            return;
        }

        let run_frame_info = RunFrameInfo
        {
            engine: info.engine.as_mut().unwrap(),
            builder,
            image_index: image_index as usize,
            render_info: &mut info.render_info,
            previous_time: &mut info.previous_time,
            frame_parity: info.frame_parity
        };

        info.frame_parity = !info.frame_parity;

        let command_buffer = run_frame(
            run_frame_info,
            info.user_app.as_mut().unwrap()
        );

        if let Some(fence) = info.fence.as_mut()
        {
            fence.cleanup_finished();
        }

        info.recreate_swapchain |= suboptimal;
        info.recreate_swapchain |= execute_builder(
            info.queue.clone(),
            info.render_info.swapchain.clone(),
            &mut info.fence,
            FrameData{
                command_buffer,
                acquire_future,
                image_index
            }
        );
    }
}

type FutureInner = PresentFuture<CommandBufferExecFuture<SwapchainAcquireFuture>>;
type FutureType = Option<Arc<FenceSignalFuture<FutureInner>>>;

struct FrameData
{
    command_buffer: Arc<PrimaryAutoCommandBuffer>,
    acquire_future: SwapchainAcquireFuture,
    image_index: u32
}

struct RunFrameInfo<'a, T>
{
    engine: &'a mut Engine,
    image_index: usize,
    builder: CommandBuilderType,
    render_info: &'a mut RenderInfo<T>,
    previous_time: &'a mut Instant,
    frame_parity: bool
}

fn run_frame<UserApp: YanyaApp, T: Clone>(
    mut frame_info: RunFrameInfo<T>,
    user_app: &mut UserApp
) -> Arc<PrimaryAutoCommandBuffer>
{
    let delta_time = frame_info.previous_time.elapsed().as_secs_f32();
    *frame_info.previous_time = Instant::now();

    {
        let object_create_info = frame_info.engine
            .object_create_partial_info(
                frame_info.render_info.resource_uploader(&mut frame_info.builder),
                frame_info.render_info.size(),
                frame_info.frame_parity
            );

        user_app.update(object_create_info, delta_time);
    }

    frame_info.builder
        .begin_render_pass(
            RenderPassBeginInfo{
                clear_values: frame_info.render_info.clear_values.clone(),
                ..RenderPassBeginInfo::framebuffer(
                    frame_info.render_info.framebuffers[frame_info.image_index].clone()
                )
            },
            SubpassBeginInfo{
                contents: SubpassContents::Inline,
                ..Default::default()
            }
        )
        .unwrap();

    {
        let object_create_info = frame_info.engine
            .object_create_partial_info(
                frame_info.render_info.resource_uploader(&mut frame_info.builder),
                frame_info.render_info.size(),
                frame_info.frame_parity
            );

        let draw_info = DrawInfo::new(
            object_create_info,
            &frame_info.render_info.pipelines
        );

        user_app.draw(draw_info);
    }

    frame_info.builder.end_render_pass(Default::default()).unwrap();
    frame_info.builder.build().unwrap()
}

fn execute_builder(
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    fence: &mut FutureType,
    frame_data: FrameData
) -> bool
{
    let FrameData{
        command_buffer,
        acquire_future,
        image_index
    } = frame_data;

    fence.take();

    let current_fence = acquire_future
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_swapchain_present(
            queue,
            SwapchainPresentInfo::swapchain_image_index(swapchain, image_index)
        ).then_signal_fence_and_flush();

    let mut recreate_swapchain = false;
    *fence = match current_fence
    {
        #[allow(clippy::arc_with_non_send_sync)]
        Ok(fence) => Some(Arc::new(fence)),
        Err(Validated::Error(VulkanError::OutOfDate)) =>
        {
            recreate_swapchain = true;
            None
        },
        Err(e) =>
        {
            let e = match e
            {
                Validated::Error(x) => format!("{x}"),
                Validated::ValidationError(x) => format!("error validating {x}")
            };

            panic!("error flushing future: {e}")
        }
    };

    recreate_swapchain
}
