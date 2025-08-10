use glam::{Vec3, Quat};
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Fullscreen},
};

const WIDTH: u32 = 1920;
const HEIGHT: u32 = 1080;

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct Camera {
    pos: [f32; 3],
    _pad1: f32,
    dir: [f32; 3],
    _pad2: f32,
    up: [f32; 3],
    _pad3: f32,
    fov: f32,
    aspect: f32,
    _pad4: [f32; 2],
}

async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Mandelbulb 3D")
        .with_inner_size(winit::dpi::PhysicalSize::new(WIDTH, HEIGHT))
        .build(&event_loop)
        .unwrap();

    let instance = wgpu::Instance::default();
    let surface = unsafe { instance.create_surface(&window) }.unwrap();

    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        })
        .await
        .unwrap();

    let (device, queue) = adapter
        .request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    let surface_format = surface.get_capabilities(&adapter).formats[0];

    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: WIDTH,
        height: HEIGHT,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Opaque,
        view_formats: vec![],
    };
    surface.configure(&device, &config);

    let mut camera = Camera {
        pos: [0.0, 0.0, -0.7],
        _pad1: 0.0,
        dir: [0.0, 0.0, 1.0],
        _pad2: 0.0,
        up: [0.0, 1.0, 0.0],
        _pad3: 0.0,
        fov: std::f32::consts::FRAC_PI_4,
        aspect: WIDTH as f32 / HEIGHT as f32,
        _pad4: [0.0; 2],
    };

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let camera_bind_group_layout =
        device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Camera Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Camera Bind Group"),
        layout: &camera_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: camera_buffer.as_entire_binding(),
        }],
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Mandelbulb Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("mandelbulb.wgsl").into()),
    });

    let render_pipeline_layout =
        device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[&camera_bind_group_layout],
            push_constant_ranges: &[],
        });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&render_pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: surface_format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let mut dragging = false;
    let mut last_cursor_pos = (0.0f32, 0.0f32);

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        let pressed = input.state == ElementState::Pressed;

                        if pressed {
                            // Управление камерой WASD + Space + LShift
                            let forward = glam::Vec3::from(camera.dir).normalize();
                            let right = forward.cross(glam::Vec3::from(camera.up)).normalize();
                            let up = glam::Vec3::from(camera.up).normalize();
                            let speed = 0.2;

                            match keycode {
                                VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,

                                VirtualKeyCode::W => {
                                    let pos = glam::Vec3::from(camera.pos) + forward * speed;
                                    camera.pos = pos.into();
                                }
                                VirtualKeyCode::S => {
                                    let pos = glam::Vec3::from(camera.pos) - forward * speed;
                                    camera.pos = pos.into();
                                }
                                VirtualKeyCode::A => {
                                    let pos = glam::Vec3::from(camera.pos) - right * speed;
                                    camera.pos = pos.into();
                                }
                                VirtualKeyCode::D => {
                                    let pos = glam::Vec3::from(camera.pos) + right * speed;
                                    camera.pos = pos.into();
                                }
                                VirtualKeyCode::Space => {
                                    let pos = glam::Vec3::from(camera.pos) + up * speed;
                                    camera.pos = pos.into();
                                }
                                VirtualKeyCode::LShift => {
                                    let pos = glam::Vec3::from(camera.pos) - up * speed;
                                    camera.pos = pos.into();
                                }

                                // Переключение fullscreen по F11
                                VirtualKeyCode::F11 => {
                                if window.fullscreen().is_some() {
                                    window.set_fullscreen(None);
                                    config.width = WIDTH;
                                    config.height = HEIGHT;
                                    surface.configure(&device, &config);
                                    camera.aspect = WIDTH as f32 / HEIGHT as f32;
                                } else {
                                    if let Some(monitor) = window.current_monitor() {
                                        if let Some(video_mode) = monitor.video_modes().next() {
                                            let width = video_mode.size().width;
                                            let height = video_mode.size().height;
                                            window.set_fullscreen(Some(Fullscreen::Exclusive(video_mode)));
                                            config.width = width;
                                            config.height = height;
                                            surface.configure(&device, &config);
                                            camera.aspect = config.width as f32 / config.height as f32;
                                        } else {
                                            window.set_fullscreen(Some(Fullscreen::Borderless(Some(monitor))));
                                    }
        }
    }
}


                                _ => {}
                            }
                        }
                    }
                }

                WindowEvent::MouseInput { state, button: MouseButton::Left, .. } => {
                    dragging = state == ElementState::Pressed;
                }

                WindowEvent::CursorMoved { position, .. } => {
                    let (x, y) = (position.x as f32, position.y as f32);
                    if dragging {
                        let dx = (x - last_cursor_pos.0) * 0.005;
                        let dy = (y - last_cursor_pos.1) * 0.005;
                        let dir = glam::Vec3::from(camera.dir);
                        let up = glam::Vec3::from(camera.up);
                        let right = up.cross(dir).normalize();
                        let rot_y = Quat::from_axis_angle(up, -dx);
                        let rot_x = Quat::from_axis_angle(right, -dy);
                        let new_dir = (rot_y * rot_x) * dir;
                        camera.dir = new_dir.normalize().into();
                    }
                    last_cursor_pos = (x, y);
                }

                _ => {}
            },

            Event::MainEventsCleared => {
                queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera]));

                let frame = surface
                    .get_current_texture()
                    .expect("Failed to acquire next swap chain texture");
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations {
                                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                                store: true,
                            },
                        })],
                        depth_stencil_attachment: None,
                    });

                    render_pass.set_pipeline(&render_pipeline);
                    render_pass.set_bind_group(0, &camera_bind_group, &[]);
                    render_pass.draw(0..6, 0..1);
                }

                queue.submit(Some(encoder.finish()));
                frame.present();
            }

            _ => {}
        }
    });
}

fn main() {
    pollster::block_on(run());
}
