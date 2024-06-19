use std::{
    mem,
    time::Instant,
    sync::Arc
};

use vulkano::{
    Validated,
    VulkanError,
    format::Format,
    memory::allocator::{AllocationCreateInfo, StandardMemoryAllocator},
    descriptor_set::allocator::StandardDescriptorSetAllocator,
    shader::EntryPoint,
    sync::{
        GpuFuture,
        future::{JoinFuture, FenceSignalFuture}
    },
    pipeline::{
        Pipeline,
        PipelineLayout,
        GraphicsPipeline,
        PipelineShaderStageCreateInfo,
        DynamicState,
        graphics::{
            GraphicsPipelineCreateInfo,
            multisample::MultisampleState,
            depth_stencil::{DepthStencilState, DepthState},
            color_blend::{ColorBlendState, ColorBlendAttachmentState, AttachmentBlend},
            rasterization::{CullMode, RasterizationState},
            input_assembly::InputAssemblyState,
            vertex_input::{VertexDefinition, Vertex},
            viewport::{Scissor, Viewport, ViewportState}
        }
    },
    image::{
        ImageUsage,
        Image,
        ImageType,
        ImageCreateInfo,
        SampleCount,
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
        physical::PhysicalDevice,
        Queue
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
        allocator::StandardCommandBufferAllocator
    }
};

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
    event::{
        Event,
        WindowEvent,
        MouseScrollDelta
    },
    event_loop::{ControlFlow, EventLoop, EventLoopWindowTarget}
};

use crate::{
    YanyaApp,
    AppOptions,
    Control,
    ShadersInfo,
    engine::Engine,
    game_object::*,
    object::{
        ObjectVertex,
        resource_uploader::ResourceUploader
    }
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
    stages: Vec<PipelineShaderStageCreateInfo>,
    shaders: ShadersInfo<EntryPoint>,
    layout: Arc<PipelineLayout>
}

impl PipelineCreateInfo
{
    pub fn new(
        stages: Vec<PipelineShaderStageCreateInfo>,
        shaders: ShadersInfo<EntryPoint>,
        layout: Arc<PipelineLayout>
    ) -> Self
    {
        Self{stages, shaders, layout}
    }
}

// just put everything in 1 place who cares lmao
struct RenderInfo
{
    pub device: Arc<Device>,
    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub pipelines: Vec<PipelineInfo>,
    pub viewport: Viewport,
    pub surface: Arc<Surface>,
    pub render_pass: Arc<RenderPass>,
    pub sampler: Arc<Sampler>,
    pub samples: SampleCount,
    pipeline_infos: Vec<PipelineCreateInfo>,
    pub memory_allocator: Arc<StandardMemoryAllocator>,
    descriptor_allocator: Arc<StandardDescriptorSetAllocator>
}

impl RenderInfo
{
    pub fn new(
        device: Arc<Device>,
        surface: Arc<Surface>,
        pipeline_infos: Vec<PipelineCreateInfo>,
        samples: SampleCount,
        capabilities: SurfaceCapabilities,
        image_format: Format,
        composite_alpha: CompositeAlpha
    ) -> Self
    {
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

        let image_count = capabilities.min_image_count.max(2);
        let min_image_count = match capabilities.max_image_count
        {
            None => image_count,
            Some(max_images) => image_count.min(max_images)
        };

        let (swapchain, images) = Swapchain::new(
            device.clone(),
            surface.clone(),
            SwapchainCreateInfo{
                min_image_count,
                image_format,
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSFER_DST,
                composite_alpha,
                ..Default::default()
            }
        ).unwrap();

        let render_pass = if let SampleCount::Sample1 = samples
        {
            vulkano::single_pass_renderpass!(
                device.clone(),
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
            )
        } else
        {
            vulkano::single_pass_renderpass!(
                device.clone(),
                attachments: {
                    multisampled: {
                        format: image_format,
                        samples: samples as u32,
                        load_op: Clear,
                        store_op: DontCare
                    },
                    color: {
                        format: image_format,
                        samples: 1,
                        load_op: DontCare,
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
                    color: [multisampled],
                    color_resolve: [color],
                    depth_stencil: {depth}
                }
            )
        }.unwrap();

        let framebuffers = Self::framebuffers(
            memory_allocator.clone(),
            samples,
            images.into_iter(),
            render_pass.clone()
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
            samples,
            sampler,
            pipeline_infos,
            memory_allocator,
            descriptor_allocator
        }
    }

    pub fn framebuffers(
        memory_allocator: Arc<StandardMemoryAllocator>,
        samples: SampleCount,
        images: impl Iterator<Item=Arc<Image>>,
        render_pass: Arc<RenderPass>
    ) -> Vec<Arc<Framebuffer>>
    {
        images.map(|image|
        {
            let format = image.format();
            let extent = image.extent();

            let view = ImageView::new_default(image).unwrap();

            let depth_image = Image::new(
                memory_allocator.clone(),
                ImageCreateInfo{
                    image_type: ImageType::Dim2d,
                    format: Format::D16_UNORM,
                    extent,
                    usage: ImageUsage::TRANSIENT_ATTACHMENT | ImageUsage::DEPTH_STENCIL_ATTACHMENT,
                    ..Default::default()
                },
                AllocationCreateInfo::default()
            ).unwrap();

            let depth = ImageView::new_default(depth_image).unwrap();

            let attachments = if let SampleCount::Sample1 = samples
            {
                vec![view, depth]
            } else
            {
                // im not sure if i need one for each swapchain image or if they can share??
                let multisampled_image = Image::new(
                    memory_allocator.clone(),
                    ImageCreateInfo{
                        image_type: ImageType::Dim2d,
                        format,
                        extent,
                        usage: ImageUsage::COLOR_ATTACHMENT | ImageUsage::TRANSIENT_ATTACHMENT,
                        samples,
                        ..Default::default()
                    },
                    AllocationCreateInfo::default()
                ).unwrap();

                let multisampled = ImageView::new_default(multisampled_image).unwrap();

                vec![multisampled, view, depth]
            };

            Framebuffer::new(
                render_pass.clone(),
                FramebufferCreateInfo{
                    attachments,
                    ..Default::default()
                }
            ).unwrap()
        }).collect::<Vec<_>>()
    }


    fn generate_pipeline(
        shader: &PipelineCreateInfo,
        viewport: Viewport,
        subpass: Subpass,
        device: Arc<Device>
    ) -> PipelineInfo
    {
        let mut dynamic_state = ahash::HashSet::default();
        dynamic_state.insert(DynamicState::Scissor);
        dynamic_state.insert(DynamicState::DepthWriteEnable);

        let pipeline = GraphicsPipeline::new(
            device,
            None,
            GraphicsPipelineCreateInfo{
                stages: shader.stages.iter().cloned().collect(),
                vertex_input_state: Some(ObjectVertex::per_vertex()
                    .definition(&shader.shaders.vertex.info().input_interface)
                    .unwrap()
                ),
                input_assembly_state: Some(InputAssemblyState::default()),
                viewport_state: Some(ViewportState{
                    viewports: [viewport].into_iter().collect(),
                    ..Default::default()
                }),
                rasterization_state: Some(RasterizationState{
                    cull_mode: CullMode::Back,
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
                    depth: Some(DepthState::simple()),
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
            self.samples,
            new_images.into_iter(),
            self.render_pass.clone()
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

    fn surface_size_associated(surface: &Arc<Surface>) -> PhysicalSize<u32>
    {
        let window = surface.object().unwrap().downcast_ref::<Window>().unwrap();

        window.inner_size()
    }
}

pub struct GraphicsInfo
{
    pub surface: Arc<Surface>,
    pub event_loop: EventLoop<()>,
    pub physical_device: Arc<PhysicalDevice>,
    pub device: Arc<Device>,
    pub pipeline_infos: Vec<PipelineCreateInfo>,
    pub samples: SampleCount,
    pub queues: Vec<Arc<Queue>>
}

// peeling back an onion layer by layer and crying profusely
type FencesType = Box<[Option<FencesTypeInner>]>;
type FencesTypeInner = Arc<FenceSignalFuture<PresentFuture<FencesTypeFuture>>>;
type FencesTypeFuture = CommandBufferExecFuture<JoinFuture<Box<(dyn GpuFuture + 'static)>, SwapchainAcquireFuture>>;

// stupid code duplication but im lazy wutever
struct HandleEventInfoRaw
{
    command_allocator: StandardCommandBufferAllocator,
    queue: Arc<Queue>,
    fences: FencesType,
    fences_amount: usize,
    device: Arc<Device>,
    render_info: RenderInfo,
    options: AppOptions
}

struct HandleEventInfo<UserApp>
{
    command_allocator: StandardCommandBufferAllocator,
    queue: Arc<Queue>,
    fences: FencesType,
    fences_amount: usize,
    device: Arc<Device>,
    render_info: RenderInfo,
    options: AppOptions,
    engine: Option<Engine>,
    user_app: Option<UserApp>,
    previous_time: Instant,
    previous_frame_index: usize,
    initialized: bool,
    recreate_swapchain: bool,
    window_resized: bool
}

impl<UserApp> From<HandleEventInfoRaw> for HandleEventInfo<UserApp>
{
    fn from(value: HandleEventInfoRaw) -> Self
    {
        Self{
            command_allocator: value.command_allocator,
            queue: value.queue,
            fences: value.fences,
            fences_amount: value.fences_amount,
            device: value.device,
            render_info: value.render_info,
            options: value.options,
            engine: None,
            user_app: None,
            previous_time: Instant::now(),
            previous_frame_index: 0,
            initialized: false,
            recreate_swapchain: false,
            window_resized: false
        }
    }
}

pub fn run<UserApp: YanyaApp + 'static>(
    info: GraphicsInfo,
    options: AppOptions,
    app_init: UserApp::AppInfo
)
{
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

    let image_format = info.physical_device
        .surface_formats(&info.surface, Default::default())
        .unwrap()[0].0;

    let render_info = RenderInfo::new(
        info.device.clone(),
        info.surface.clone(),
        info.pipeline_infos,
        info.samples,
        capabilities,
        image_format,
        composite_alpha
    );

    let fences_amount = render_info.framebuffers.len();

    let mut handle_info: HandleEventInfo<UserApp> = HandleEventInfo::from(
        HandleEventInfoRaw{
            fences_amount,
            fences: vec![None; fences_amount].into_boxed_slice(),
            command_allocator: StandardCommandBufferAllocator::new(
                info.device.clone(),
                Default::default()
            ),
            queue: info.queues[0].clone(),
            render_info,
            device: info.device,
            options
        }
    );

    info.event_loop.set_control_flow(ControlFlow::Poll);

    let mut app_init: Option<_> = Some(app_init);
    info.event_loop.run(move |event, event_loop|
    {
        handle_event(&mut handle_info, event, event_loop, &mut app_init);
    }).unwrap();
}

fn handle_event<UserApp: YanyaApp + 'static>(
    info: &mut HandleEventInfo<UserApp>,
    event: Event<()>,
    event_loop: &EventLoopWindowTarget<()>,
    app_init: &mut Option<UserApp::AppInfo>
)
{
    match event
    {
        Event::WindowEvent{
            event,
            ..
        } =>
        {
            match event
            {
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::Resized(_) => info.window_resized = true,
                WindowEvent::CursorMoved{position, ..} =>
                {
                    if !info.initialized
                    {
                        return;
                    }

                    let (width, height): (f64, f64) = info.render_info.surface_size().into();

                    if width == 0.0 || height == 0.0
                    {
                        return;
                    }

                    let position = (position.x / width, position.y / height);

                    info.user_app.as_mut().unwrap().mouse_move(position);
                },
                WindowEvent::MouseInput{
                    button,
                    state,
                    ..
                } =>
                {
                    if !info.initialized
                    {
                        return;
                    }

                    let control = Control::Mouse{button, state};
                    info.user_app.as_mut().unwrap().input(control);
                },
                WindowEvent::MouseWheel{delta, ..} =>
                {
                    if !info.initialized
                    {
                        return;
                    }

                    let (x, y) = match delta
                    {
                        MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64),
                        MouseScrollDelta::PixelDelta(PhysicalPosition{x, y}) => (x, y)
                    };

                    let control = Control::Scroll{x, y};
                    info.user_app.as_mut().unwrap().input(control);
                },
                WindowEvent::KeyboardInput{event, ..} =>
                {
                    if !info.initialized
                    {
                        return;
                    }

                    let control = Control::Keyboard{
                        keycode: event.physical_key,
                        state: event.state
                    };

                    info.user_app.as_mut().unwrap().input(control);
                },
                _ => ()
            }
        },
        Event::AboutToWait =>
        {
            let [x, y]: [u32; 2] = info.render_info.surface_size().into();

            if x == 0 || y == 0
            {
                return;
            }

            handle_redraw(info, app_init);
        },
        _ => ()
    }
}

fn handle_redraw<UserApp: YanyaApp + 'static>(
    info: &mut HandleEventInfo<UserApp>,
    app_init: &mut Option<UserApp::AppInfo>
)
{
    let mut builder = AutoCommandBufferBuilder::primary(
        &info.command_allocator,
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
        let image_index = image_index as usize;

        if !info.initialized
        {
            info.initialized = true;

            info.engine = Some(Engine::new(
                &info.options.assets_paths,
                info.render_info.resource_uploader(&mut builder),
                info.device.clone(),
                info.fences_amount,
                info.options.default_shader.unwrap()
            ));

            info.user_app = {
                let init_info = info.engine
                    .as_mut()
                    .unwrap()
                    .init_partial_info(
                        info.render_info.resource_uploader(&mut builder),
                        info.render_info.size(),
                        image_index
                    );

                let app_init = mem::take(app_init).unwrap();
                Some(UserApp::init(init_info, app_init))
            };
        }

        let run_frame_info = RunFrameInfo
        {
            engine: info.engine.as_mut().unwrap(),
            builder,
            image_index,
            render_info: &mut info.render_info,
            previous_time: &mut info.previous_time
        };

        let command_buffer = run_frame(
            run_frame_info,
            info.user_app.as_mut().unwrap(),
            &info.options
        );

        info.recreate_swapchain |= suboptimal;
        info.recreate_swapchain |= execute_builder(
            info.device.clone(),
            info.queue.clone(),
            info.render_info.swapchain.clone(),
            &mut info.fences,
            FrameData{
                command_buffer,
                image_index,
                previous_frame_index: info.previous_frame_index,
                acquire_future
            }
        );

        info.previous_frame_index = image_index;
    }
}

type FutureInner = PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>;
type FutureType = Option<Arc<FenceSignalFuture<FutureInner>>>;

struct FrameData
{
    command_buffer: Arc<PrimaryAutoCommandBuffer>,
    image_index: usize,
    previous_frame_index: usize,
    acquire_future: SwapchainAcquireFuture
}

struct RunFrameInfo<'a>
{
    engine: &'a mut Engine,
    image_index: usize,
    builder: CommandBuilderType,
    render_info: &'a mut RenderInfo,
    previous_time: &'a mut Instant
}

fn run_frame<UserApp: YanyaApp>(
    mut frame_info: RunFrameInfo,
    user_app: &mut UserApp,
    options: &AppOptions
) -> Arc<PrimaryAutoCommandBuffer>
{
    let delta_time = frame_info.previous_time.elapsed().as_secs_f32();
    *frame_info.previous_time = Instant::now();

    user_app.update(delta_time);

    {
        let object_create_info = frame_info.engine
            .object_create_partial_info(
                frame_info.render_info.resource_uploader(&mut frame_info.builder),
                frame_info.render_info.size(),
                frame_info.image_index
            );

        user_app.update_buffers(object_create_info);
    }

    let clear_color = Some(options.clear_color);
    let depth_clear = Some(1.0.into());

    let clear_values = if let SampleCount::Sample1 = frame_info.render_info.samples
    {
        vec![clear_color, depth_clear]
    } else
    {
        vec![clear_color, None, depth_clear]
    };

    frame_info.builder
        .begin_render_pass(
            RenderPassBeginInfo{
                clear_values,
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
                frame_info.image_index
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
    device: Arc<Device>,
    queue: Arc<Queue>,
    swapchain: Arc<Swapchain>,
    fences: &mut [FutureType],
    frame_data: FrameData
) -> bool
{
    let FrameData{
        command_buffer,
        image_index,
        previous_frame_index,
        acquire_future
    } = frame_data;

    if let Some(fence) = &fences[image_index]
    {
        fence.wait(None).unwrap();
    }

    let previous_fence = match &fences[previous_frame_index]
    {
        Some(fence) => fence.clone().boxed(),
        None =>
        {
            let mut now = vulkano::sync::now(device);
            now.cleanup_finished();

            now.boxed()
        }
    };

    let fence = previous_fence
        .join(acquire_future)
        .then_execute(queue.clone(), command_buffer)
        .unwrap()
        .then_swapchain_present(
            queue,
            SwapchainPresentInfo::swapchain_image_index(
                swapchain,
                image_index as u32
            )
        ).then_signal_fence_and_flush();

    let mut recreate_swapchain = false;
    fences[image_index] = match fence
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

            eprintln!("error flushing future: {e}");

            None
        }
    };

    recreate_swapchain
}
