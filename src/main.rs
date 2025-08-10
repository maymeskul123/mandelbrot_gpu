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

    // Изначальная конфигурация поверхности
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
        pos: [0.0, 0.0, -3.0],
        _pad1: 0.0,
        dir: [0.0, 0.0, 1.0],
        _pad2: 0.0,
        up: [0.0, 1.0, 0.0],
        _pad3: 0.0,
        fov: std::f32::consts::FRAC_PI_4,
        aspect: config.width as f32 / config.height as f32,
        _pad4: [0.0; 2],
    };

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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

    let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
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
    let world_up = Vec3::Y;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        let pressed = input.state == ElementState::Pressed;
                        if pressed {
                            // Обработка переключения полноэкранного режима
                            if keycode == VirtualKeyCode::F11 {
                                if window.fullscreen().is_some() {
                                    window.set_fullscreen(None);
                                    // Возвращаем windowed размер
                                    config.width = WIDTH;
                                    config.height = HEIGHT;
                                } else {
                                    let monitor = window.current_monitor();
                                    if let Some(monitor) = monitor {
                                        // Получаем первый доступный видео режим (можно улучшить выбор)
                                        if let Some(video_mode) = monitor.video_modes().next() {
                                            window.set_fullscreen(Some(Fullscreen::Exclusive(video_mode.clone())));
                                            config.width = video_mode.size().width;
                                            config.height = video_mode.size().height;
                                        } else {
                                            window.set_fullscreen(Some(Fullscreen::Borderless(Some(monitor.clone()))));
                                            config.width = monitor.size().width;
                                            config.height = monitor.size().height;
                                        }
                                    }
                                }
                                camera.aspect = config.width as f32 / config.height as f32;
                                surface.configure(&device, &config);
                            }

                            // Передвижение камеры
                            let mut pos = Vec3::from(camera.pos);
                            let forward = Vec3::from(camera.dir).normalize();
                            let up = Vec3::from(camera.up).normalize();
                            let right = forward.cross(up).normalize();
                            let speed = 0.2;
                            match keycode {
                                VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                                VirtualKeyCode::W => pos += forward * speed,
                                VirtualKeyCode::S => pos -= forward * speed,
                                VirtualKeyCode::A => pos -= right * speed,
                                VirtualKeyCode::D => pos += right * speed,
                                VirtualKeyCode::Space => pos += up * speed,
                                VirtualKeyCode::LShift => pos -= up * speed,
                                _ => {}
                            }
                            camera.pos = pos.into();
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

                        let dir = Vec3::from(camera.dir);
                        let up = Vec3::from(camera.up);
                        let right = dir.cross(up).normalize();

                        let rot_y = Quat::from_axis_angle(up, -dx);
                        let rot_x = Quat::from_axis_angle(right, -dy);
                        let new_dir = (rot_y * rot_x) * dir;

                        let forward = new_dir.normalize();

                        let new_right = forward.cross(world_up).normalize();
                        let new_up = new_right.cross(forward).normalize();

                        camera.dir = forward.into();
                        camera.up = new_up.into();
                    }
                    last_cursor_pos = (x, y);
                }
                WindowEvent::MouseWheel { delta, .. } => {
                    let mut pos = Vec3::from(camera.pos);
                    let forward = Vec3::from(camera.dir).normalize();
                    let scroll_amount = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y,
                        MouseScrollDelta::PixelDelta(p) => p.y as f32 * 0.1,
                    };
                    pos += forward * scroll_amount * 0.5;
                    camera.pos = pos.into();
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
