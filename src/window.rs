use std::{
    time::Instant,
    sync::Arc
};

use vulkano::{
    format::Format,
    memory::allocator::StandardMemoryAllocator,
    descriptor_set::allocator::StandardDescriptorSetAllocator,
	sampler::{
        Filter,
        Sampler,
        SamplerCreateInfo
    },
    sync::{
        FlushError,
        GpuFuture,
        future::{JoinFuture, FenceSignalFuture}
    },
    pipeline::{
        Pipeline,
        PipelineLayout,
        GraphicsPipeline,
        StateMode,
        graphics::{
            color_blend::ColorBlendState,
            rasterization::{CullMode, RasterizationState},
            input_assembly::InputAssemblyState,
            vertex_input::Vertex,
            viewport::{Viewport, ViewportState}
        }
    },
    image::{
        ImageUsage,
        SwapchainImage,
        view::ImageView
    },
    swapchain::{
        self,
        AcquireError,
        Surface,
        SurfaceCapabilities,
        CompositeAlpha,
        PresentFuture,
        Swapchain,
        SwapchainAcquireFuture,
        SwapchainCreateInfo,
        SwapchainCreationError,
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
        DeviceEvent,
        KeyboardInput,
        MouseScrollDelta
    },
    event_loop::{ControlFlow, EventLoop}
};

use crate::{
    YanyaApp,
    AppOptions,
    Control,
    PipelineInfo,
    ShadersInfo,
    engine::Engine,
    game_object::*,
    object::{
        ObjectVertex,
        resource_uploader::ResourceUploader
    }
};


pub fn framebuffers(
    images: impl Iterator<Item=Arc<SwapchainImage>>,
    render_pass: Arc<RenderPass>
) -> Vec<Arc<Framebuffer>>
{
    images.map(|image|
    {
        let view = ImageView::new_default(image).unwrap();
        Framebuffer::new(
            render_pass.clone(),
            FramebufferCreateInfo{
                attachments: vec![view],
                ..Default::default()
            }
        ).unwrap()
    }).collect::<Vec<_>>()
}

pub fn default_builder(
    allocator: &StandardCommandBufferAllocator,
    queue_family_index: u32
) -> AutoCommandBufferBuilder<PrimaryAutoCommandBuffer>
{
    AutoCommandBufferBuilder::primary(
        allocator,
        queue_family_index,
        CommandBufferUsage::OneTimeSubmit
    ).unwrap()
}

struct PipelineInfoRaw
{
    pipeline: Arc<GraphicsPipeline>,
    layout: Arc<PipelineLayout>
}

// just put everything in 1 place who cares lmao
struct RenderInfo
{
    pub device: Arc<Device>,
    pub swapchain: Arc<Swapchain>,
    pub framebuffers: Vec<Arc<Framebuffer>>,
    pub pipelines: Vec<PipelineInfoRaw>,
    pub viewport: Viewport,
    pub surface: Arc<Surface>,
    pub render_pass: Arc<RenderPass>,
    pub sampler: Arc<Sampler>,
    shaders: Vec<ShadersInfo>,
    pub descriptor_set_allocator: StandardDescriptorSetAllocator,
    pub memory_allocator: StandardMemoryAllocator
}

impl RenderInfo
{
    pub fn new(
        device: Arc<Device>,
        surface: Arc<Surface>,
        shaders: Vec<ShadersInfo>,
        capabilities: SurfaceCapabilities,
        image_format: Format,
        composite_alpha: CompositeAlpha
    ) -> Self
    {
        let descriptor_set_allocator = StandardDescriptorSetAllocator::new(device.clone());
        let sampler = Sampler::new(
            device.clone(),
            SamplerCreateInfo{
                mag_filter: Filter::Nearest,
                min_filter: Filter::Nearest,
                ..Default::default()
            }
        ).unwrap();

        let memory_allocator = StandardMemoryAllocator::new_default(device.clone());

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
                image_format: Some(image_format),
                image_extent: dimensions.into(),
                image_usage: ImageUsage::COLOR_ATTACHMENT,
                composite_alpha,
                ..Default::default()
            }
        ).unwrap();

        let render_pass = vulkano::single_pass_renderpass!(
            device.clone(),
            attachments: {
                color: {
                    load: Clear,
                    store: Store,
                    format: image_format,
                    samples: 1,
                }
            },
            pass: {
                color: [color],
                depth_stencil: {}
            }
        ).unwrap();

        let framebuffers = framebuffers(images.into_iter(), render_pass.clone());

        let viewport = Viewport{
            origin: [0.0, 0.0],
            dimensions: dimensions.into(),
            depth_range: 0.0..1.0
        };


        let pipelines = Self::generate_pipelines(
            viewport.clone(),
            render_pass.clone(),
            device.clone(),
            &shaders
        );

        Self{
            device,
            swapchain,
            framebuffers,
            pipelines,
            viewport,
            surface,
            render_pass,
            sampler,
            shaders,
            descriptor_set_allocator,
            memory_allocator
        }
    }

    fn generate_pipeline(
        shader: &ShadersInfo,
        viewport: Viewport,
        subpass: Subpass,
        device: Arc<Device>
    ) -> PipelineInfoRaw
    {
        let pipeline = GraphicsPipeline::start()
            .vertex_input_state(ObjectVertex::per_vertex())
            .vertex_shader(shader.vertex_entry(), ())
            .input_assembly_state(InputAssemblyState::new())
            .viewport_state(ViewportState::viewport_fixed_scissor_irrelevant([viewport]))
            .fragment_shader(shader.fragment_entry(), ())
            .color_blend_state(ColorBlendState::new(subpass.num_color_attachments()).blend_alpha())
            .rasterization_state(RasterizationState{
                cull_mode: StateMode::Fixed(CullMode::Back),
                ..Default::default()
            })
            .render_pass(subpass)
            .build(device)
            .unwrap();

        PipelineInfoRaw{
            layout: pipeline.layout().clone(),
            pipeline
        }
    }

    fn generate_pipelines(
        viewport: Viewport,
        render_pass: Arc<RenderPass>,
        device: Arc<Device>,
        shaders: &[ShadersInfo]
    ) -> Vec<PipelineInfoRaw>
    {
        let subpass = Subpass::from(render_pass, 0).unwrap();

        shaders.iter().map(|shader|
        {
            Self::generate_pipeline(
                shader,
                viewport.clone(),
                subpass.clone(),
                device.clone()
            )
        }).collect()
    }

    pub fn pipeline_info(&self, index: usize) -> PipelineInfo
    {
        PipelineInfo::new(
            &self.descriptor_set_allocator,
            self.sampler.clone(),
            self.pipelines[index].layout.clone()
        )
    }

    pub fn resource_uploader<'a>(
        &'a self,
        builder: &'a mut CommandBuilderType,
        index: usize
    ) -> ResourceUploader<'a>
    {
        ResourceUploader{
            allocator: &self.memory_allocator,
            builder,
            pipeline_info: self.pipeline_info(index)
        }
    }

    pub fn recreate(
        &mut self,
        redraw_window: bool
    ) -> Result<(), SwapchainCreationError>
    {
        let dimensions = self.surface_size();

        let (new_swapchain, new_images) = self.swapchain.recreate(SwapchainCreateInfo{
            image_extent: dimensions.into(),
            ..self.swapchain.create_info()
        })?;

        self.swapchain = new_swapchain;
        self.framebuffers = framebuffers(new_images.into_iter(), self.render_pass.clone());

        if redraw_window
        {
            self.viewport.dimensions = dimensions.into();

            self.pipelines = Self::generate_pipelines(
                self.viewport.clone(),
                self.render_pass.clone(),
                self.device.clone(),
                &self.shaders
            );
        }

        Ok(())
    }

    pub fn aspect(&self) -> f32
    {
        let size: [f32; 2] = self.surface_size().into();

        size[0] / size[1]
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
    pub shaders: Vec<ShadersInfo>,
    pub queues: Vec<Arc<Queue>>
}

pub fn run<UserApp: YanyaApp + 'static>(info: GraphicsInfo, options: AppOptions)
{
    let GraphicsInfo{
        surface,
        event_loop,
        physical_device,
        device,
        shaders,
        queues
    } = info;

    let capabilities = physical_device
        .surface_capabilities(&surface, Default::default())
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

    let image_format = physical_device
        .surface_formats(&surface, Default::default())
        .unwrap()[0].0;

    let mut render_info = RenderInfo::new(
        device.clone(), surface.clone(), shaders,
        capabilities, image_format, composite_alpha
    );

    let command_allocator =
        StandardCommandBufferAllocator::new(device.clone(), Default::default());

    let queue = queues[0].clone();

    let fences_amount = render_info.framebuffers.len();
    let mut fences = vec![None; fences_amount].into_boxed_slice();
    let mut previous_frame_index = 0;

    let mut engine: Option<Engine> = None;
    let mut user_app: Option<UserApp> = None;

    let mut previous_time = Instant::now();

    let mut recreate_swapchain = false;
    let mut window_resized = false;

    let mut initialized = false;

    // ill change this later wutever
    let pipeline_index = 0;

    event_loop.run(move |event, _, control_flow|
    {
        match event
        {
            Event::WindowEvent{
                event: WindowEvent::CloseRequested,
                ..
            } =>
            {
                *control_flow = ControlFlow::Exit;
            },
            Event::WindowEvent{
                event: WindowEvent::Resized(_),
                ..
            } =>
            {
                window_resized = true;
            },
            Event::WindowEvent{
                event: WindowEvent::CursorMoved{position, ..},
                   ..
            } =>
            {
                if initialized 
                {
                    let (width, height): (f64, f64) = render_info.surface_size().into();
                    let position = (position.x / width, position.y / height);

                    user_app.as_mut().unwrap().mouse_move(position);
                }
            },
            Event::DeviceEvent{
                event: DeviceEvent::Button{
                    button,
                    state
                },
                ..
            } =>
            {
                if initialized
                {
                    let control = Control::Mouse{button, state};
                    user_app.as_mut().unwrap().input(control);
                }
            },
            Event::DeviceEvent{
                event: DeviceEvent::MouseWheel{
                    delta
                },
                ..
            } =>
            {
                if initialized
                {
                    let (x, y) = match delta
                    {
                        MouseScrollDelta::LineDelta(x, y) => (x as f64, y as f64),
                        MouseScrollDelta::PixelDelta(PhysicalPosition{x, y}) => (x as f64, y as f64)
                    };

                    let control = Control::Scroll{x, y};
                    user_app.as_mut().unwrap().input(control);
                }
            },
            Event::DeviceEvent{
                event: DeviceEvent::Key(input),
                ..
            } =>
            {
                if initialized
                {
                    let KeyboardInput{virtual_keycode: button, state, ..} = input;

                    if let Some(keycode) = button
                    {
                        let control = Control::Keyboard{keycode, state};
                        user_app.as_mut().unwrap().input(control);
                    }
                }
            },
            Event::MainEventsCleared =>
            {
                let mut builder = default_builder(&command_allocator, queue.queue_family_index());

                let acquired =
                    match swapchain::acquire_next_image(render_info.swapchain.clone(), None)
                    {
                        Ok(x) => Some(x),
                        Err(AcquireError::OutOfDate) =>
                        {
                            None
                        },
                        Err(e) => panic!("error getting next image >-< ({:?})", e)
                    };

                if let Some((image_index, suboptimal, acquire_future)) = acquired
                {
                    let image_index = image_index as usize;

                    if !initialized
                    {
                        initialized = true;

                        engine = Some(Engine::new(
                            &options.assets_paths,
                            render_info.resource_uploader(&mut builder, pipeline_index),
                            device.clone(),
                            fences_amount
                        ));

                        user_app = {
                            let aspect = render_info.aspect();

                            let init_info = engine
                                .as_mut()
                                .unwrap()
                                .init_partial_info(
                                    render_info.resource_uploader(&mut builder, pipeline_index),
                                    aspect,
                                    image_index
                                );

                            Some(UserApp::init(init_info))
                        };
                    }

                    let run_frame_info = RunFrameInfo
                    {
                        engine: engine.as_mut().unwrap(),
                        builder,
                        image_index,
                        layout: render_info.pipelines[pipeline_index].layout.clone(),
                        render_info: &mut render_info,
                        previous_time: &mut previous_time
                    };

                    let command_buffer = run_frame(
                        run_frame_info,
                        user_app.as_mut().unwrap(),
                        &options
                    );

                    recreate_swapchain |= suboptimal;
                    recreate_swapchain |= execute_builder(
                        device.clone(),
                        queue.clone(),
                        render_info.swapchain.clone(),
                        &mut fences,
                        FrameData{
                            command_buffer,
                            image_index,
                            previous_frame_index,
                            acquire_future
                        }
                    );

                    previous_frame_index = image_index;
                }
            },
            Event::RedrawEventsCleared =>
            {
                if recreate_swapchain || window_resized
                {
                    recreate_swapchain = false;

                    match render_info.recreate(window_resized)
                    {
                        Ok(_) => (),
                        Err(SwapchainCreationError::ImageExtentNotSupported{..}) => return,
                        Err(e) => panic!("couldnt recreate swapchain ; -; ({:?})", e)
                    }

                    if initialized
                    {
                        let swap_pipeline = render_info.pipeline_info(pipeline_index);

                        engine.as_mut().unwrap().swap_pipeline(&swap_pipeline);
                        user_app.as_mut().unwrap().swap_pipeline(swap_pipeline);

                        if window_resized
                        {
                            user_app.as_mut().unwrap().resize(render_info.aspect());
                        }
                    }

                    window_resized = false;
                }
            },
            _ => ()
        }
    });
}

type FutureInner = PresentFuture<CommandBufferExecFuture<JoinFuture<Box<dyn GpuFuture>, SwapchainAcquireFuture>>>;
type FutureType = Option<Arc<FenceSignalFuture<FutureInner>>>;

struct FrameData
{
    command_buffer: PrimaryAutoCommandBuffer,
    image_index: usize,
    previous_frame_index: usize,
    acquire_future: SwapchainAcquireFuture
}

struct RunFrameInfo<'a>
{
    engine: &'a mut Engine,
    image_index: usize,
    builder: CommandBuilderType,
    layout: Arc<PipelineLayout>,
    render_info: &'a mut RenderInfo,
    previous_time: &'a mut Instant
}

fn run_frame<UserApp: YanyaApp>(
    mut frame_info: RunFrameInfo,
    user_app: &mut UserApp,
    options: &AppOptions
) -> PrimaryAutoCommandBuffer
{

    let delta_time = frame_info.previous_time.elapsed().as_secs_f32();
    *frame_info.previous_time = Instant::now();

    user_app.update(delta_time);

    // and ill change this one later too
    let pipeline_index = 0;

    let aspect = frame_info.render_info.aspect();
    {
        let object_create_info = frame_info.engine
            .object_create_partial_info(
                frame_info.render_info.resource_uploader(&mut frame_info.builder, pipeline_index),
                aspect,
                frame_info.image_index
            );

        user_app.update_buffers(object_create_info);
    }

    frame_info.builder.begin_render_pass(
        RenderPassBeginInfo{
            clear_values: vec![Some(options.clear_color)],
            ..RenderPassBeginInfo::framebuffer(
                frame_info.render_info.framebuffers[frame_info.image_index].clone()
            )
        },
        SubpassContents::Inline
    ).unwrap().bind_pipeline_graphics(
        frame_info.render_info.pipelines[pipeline_index].pipeline.clone()
    );

    {
        let object_create_info = frame_info.engine
            .object_create_partial_info(
                frame_info.render_info.resource_uploader(&mut frame_info.builder, pipeline_index),
                aspect,
                frame_info.image_index
            );

        let draw_info = DrawInfo{
            object_info: object_create_info,
            layout: frame_info.layout
        };

        user_app.draw(draw_info);
    }

    frame_info.builder.end_render_pass().unwrap();
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

    let previous_fence = match fences[previous_frame_index].clone()
    {
        Some(fence) => fence.boxed(),
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
        Ok(fence) => Some(Arc::new(fence)),
        Err(FlushError::OutOfDate) =>
        {
            recreate_swapchain = true;
            None
        },
        Err(e) =>
        {
            eprintln!("error flushing future ;; ({:?})", e);
            None
        }
    };

    recreate_swapchain
}
