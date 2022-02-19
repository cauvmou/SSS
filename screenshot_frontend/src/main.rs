use std::{process::Command, env};

use clipboard_ext::{clipboard::ClipboardContext, prelude::ClipboardProvider};
use dirs::home_dir;
use image::{DynamicImage, GenericImageView};
use wgpu::include_wgsl;
use wgpu::util::DeviceExt;
use winit::{event_loop::{EventLoop, ControlFlow}, window::{WindowBuilder, Fullscreen, Window}, event::{KeyboardInput, ElementState, VirtualKeyCode, MouseButton}, dpi::{PhysicalPosition, LogicalPosition}};

// args: monitor_x monitor_y path
fn main() {

    let args: Vec<String> = env::args().collect();
    let monitor_x: u32 = args.get(1).unwrap().to_owned().parse().unwrap();
    let monitor_y: u32 = args.get(2).unwrap().to_owned().parse().unwrap();
    let path = args.get(3).unwrap().to_owned();
    let image = image::open(path).unwrap();

    let monitor_position: PhysicalPosition<u32> = PhysicalPosition::new(monitor_x, monitor_y);

    let preview_event_loop: EventLoop<()> = EventLoop::new();

    let window = WindowBuilder::new()
        .with_decorations(false)
        .with_transparent(false)
        .with_title("SSS Preview")
        .with_position(monitor_position)
        .with_fullscreen(Some(Fullscreen::Borderless(None)))
        .build(&preview_event_loop).unwrap();
    
    let mut state = pollster::block_on(State::new(&window, &image));

    let mut mouse_down: Option<PhysicalPosition<u32>> = None;
    let mut mouse_up: Option<PhysicalPosition<u32>> = None;
    let mut mouse_position: PhysicalPosition<u32> = PhysicalPosition::new(0,0);
    
    preview_event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;
        match event {
        winit::event::Event::WindowEvent {
            ref event,
            window_id,
        } if window_id == window.id() => match event {
            winit::event::WindowEvent::CloseRequested
            | winit::event::WindowEvent::KeyboardInput {
                input:
                    KeyboardInput {
                        state: ElementState::Pressed,
                        virtual_keycode: Some(VirtualKeyCode::Escape),
                        ..
                    },
                ..
            } => {
                crop(&window, mouse_down, mouse_up, &image);
                *control_flow = ControlFlow::Exit;
            },
            winit::event::WindowEvent::Resized(physical_size) => {
                state.resize(*physical_size);
            }
            winit::event::WindowEvent::ScaleFactorChanged {new_inner_size, .. } => {
                state.resize(**new_inner_size);
            },
            winit::event::WindowEvent::CursorMoved { position, .. } => {
                mouse_position = PhysicalPosition::new(position.x as u32, position.y as u32);
            }
            winit::event::WindowEvent::MouseInput { state, button, .. } => {
                    match state {
                        ElementState::Pressed if *button == MouseButton::Left => {
                            mouse_down = Some(mouse_position)
                        },
                        ElementState::Released if *button == MouseButton::Left => {
                            mouse_up = Some(mouse_position);
                            crop(&window, mouse_down, mouse_up, &image);
                            *control_flow = ControlFlow::Exit;
                        },
                        _ => {}
                    }
            }
            _ => {}
        },
        winit::event::Event::RedrawRequested(window_id) if window_id == window.id() => {

            
            if mouse_down.is_some() && mouse_up.is_none() {

                let pos1: LogicalPosition<f32> = LogicalPosition::new((mouse_down.unwrap().x as f32 / window.inner_size().width as f32) * 2.0 - 1.0, 
                    ((mouse_down.unwrap().y as f32 / window.inner_size().height as f32) * 2.0 - 1.0) * -1.0);
                let pos2: LogicalPosition<f32> = LogicalPosition::new((mouse_position.x as f32 / window.inner_size().width as f32) * 2.0 - 1.0, 
                    ((mouse_position.y as f32 / window.inner_size().height as f32) * 2.0 - 1.0) * -1.0);

                state.update(pos1, pos2);
            }

            match state.render() {
                Ok(_) => {}

                Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                Err(wgpu::SurfaceError::Outdated) => state.resize(window.inner_size()),

                Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,

                Err(e) => eprintln!("Err: {:?}", e),
            }
        },
        winit::event::Event::MainEventsCleared => {
            window.request_redraw();
        }
        _ => {}
    }})
}

fn crop(window: &Window, mouse_down: Option<PhysicalPosition<u32>>, mouse_up: Option<PhysicalPosition<u32>>, image: &DynamicImage) {
    let mut clipboard_ctx: ClipboardContext = ClipboardProvider::new().unwrap();
    let path = format!("{}/.sss/tmp.png", home_dir().unwrap().to_str().unwrap());
    let (mut x, mut y, mut width, mut height) = (0, 0, image.width(), image.height());
    window.set_visible(false);
    if mouse_down.is_some() && mouse_up.is_some() {
        x = if mouse_up.unwrap().x < mouse_down.unwrap().x {mouse_up.unwrap().x} else {mouse_down.unwrap().x};
        width = if mouse_up.unwrap().x < mouse_down.unwrap().x {mouse_down.unwrap().x - mouse_up.unwrap().x} 
            else {mouse_up.unwrap().x - mouse_down.unwrap().x};

        y = if mouse_up.unwrap().y < mouse_down.unwrap().y {mouse_up.unwrap().y} else {mouse_down.unwrap().y};
        height = if mouse_up.unwrap().y < mouse_down.unwrap().y {mouse_down.unwrap().y - mouse_up.unwrap().y} 
            else {mouse_up.unwrap().y - mouse_down.unwrap().y};

    }
    clipboard_ctx.set_contents("Image is being saved...".to_owned()).unwrap();
    image.crop_imm(x,y,width,height).save_with_format(&path, image::ImageFormat::Png).unwrap();
    Command::new("xclip").args(&["-in", "-selection", "clipboard", "-target", "image/png", &path]).spawn().unwrap();
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 3],
    tex_coords: [f32; 2],
}

impl Vertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
                wgpu::VertexAttribute {
                    offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                }
            ]
        }
    }
}

const VERTICES: &[Vertex] = &[
    Vertex { position: [-1.0, 1.0, 0.0], tex_coords: [0.0, 0.0] },
    Vertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0] },

    Vertex { position: [1.0, 1.0, 0.0], tex_coords: [1.0, 0.0] },
    Vertex { position: [-1.0, -1.0, 0.0], tex_coords: [0.0, 1.0] },
    Vertex { position: [1.0, -1.0, 0.0], tex_coords: [1.0, 1.0] },
];

// This is very messy 
struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    num_vertices: u32,
    texture_bind_group: wgpu::BindGroup,

    overlay_render_pipeline: Option<wgpu::RenderPipeline>,
    overlay_vertex_buffer: Option<wgpu::Buffer>,
    overlay_num_vertices: Option<u32>,
}

impl State {
    
    async fn new(window: &Window, img: &DynamicImage) -> Self {
        let size = window.inner_size();
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
                label: None,
            },
            None,
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let rgba = img.as_rgba8().unwrap();
        let dimensions = img.dimensions();

        let texture_size = wgpu::Extent3d {
            width: dimensions.0,
            height: dimensions.1,
            depth_or_array_layers: 1,
        };
        let texture = device.create_texture(
            &wgpu::TextureDescriptor {
                size: texture_size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8UnormSrgb,
                usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
                label: Some("diffuse_texture"),
            }
        );

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            }, 
            rgba, 
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: std::num::NonZeroU32::new(4 * dimensions.0),
                rows_per_image: std::num::NonZeroU32::new(dimensions.1),
            },
            texture_size,
        );

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let texture_bind_group_layout = device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        let texture_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &texture_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::Sampler(&sampler),
                    }
                ],
                label: Some("diffuse_bind_group"),
            }
        );

        let shader = device.create_shader_module(&include_wgsl!("shader.wgsl"));

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc()
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }]
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(VERTICES),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let num_vertices = VERTICES.len() as u32;
        
         
        Self {
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            vertex_buffer,
            num_vertices,
            texture_bind_group,
            overlay_render_pipeline: None,
            overlay_vertex_buffer: None,
            overlay_num_vertices: None,
        }
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    fn update(&mut self, pos1: LogicalPosition<f32>, pos2: LogicalPosition<f32>) {

        let tex1: LogicalPosition<f32> = LogicalPosition::new((pos1.x + 1.0) / 2.0, 1.0 - (pos1.y + 1.0) / 2.0);
        let tex2: LogicalPosition<f32> = LogicalPosition::new((pos2.x + 1.0) / 2.0, 1.0 - (pos2.y + 1.0) / 2.0);

        let vert: &[Vertex] = &[
            Vertex { position: [pos1.x, pos1.y, 0.0], tex_coords: [tex1.x, tex1.y] },
            Vertex { position: [pos1.x, pos2.y, 0.0], tex_coords: [tex1.x, tex2.y] },
            Vertex { position: [pos2.x, pos1.y, 0.0], tex_coords: [tex2.x, tex1.y] },

            Vertex { position: [pos2.x, pos1.y, 0.0], tex_coords: [tex2.x, tex1.y] },
            Vertex { position: [pos1.x, pos2.y, 0.0], tex_coords: [tex1.x, tex2.y] },
            Vertex { position: [pos2.x, pos2.y, 0.0], tex_coords: [tex2.x, tex2.y] },
        ];

        let shader = self.device.create_shader_module(&include_wgsl!("overlay.wgsl"));

        let texture_bind_group_layout = self.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(
                            wgpu::SamplerBindingType::Filtering,
                        ),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            }
        );

        let render_pipeline_layout = self.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&texture_bind_group_layout],
            push_constant_ranges: &[],
        });

        let render_pipeline = self.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[
                    Vertex::desc()
                ],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState {
                    format: self.config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }]
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
        });

        let vertex_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(vert),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let num_vertices = vert.len() as u32;

        self.overlay_render_pipeline = Some(render_pipeline);
        self.overlay_vertex_buffer = Some(vertex_buffer);
        self.overlay_num_vertices = Some(num_vertices);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(0..self.num_vertices, 0..1);
            if self.overlay_render_pipeline.is_some() && self.overlay_vertex_buffer.is_some() && self.overlay_num_vertices.is_some() {
                render_pass.set_pipeline(&self.overlay_render_pipeline.as_ref().unwrap());
                render_pass.set_bind_group(0, &self.texture_bind_group, &[]);
                render_pass.set_vertex_buffer(0, self.overlay_vertex_buffer.as_ref().unwrap().slice(..));
                render_pass.draw(0..self.overlay_num_vertices.unwrap(), 0..1);
            }
        }
    
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}
