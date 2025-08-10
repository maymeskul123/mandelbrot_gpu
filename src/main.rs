use winit::{
    event::{Event, WindowEvent, KeyboardInput, VirtualKeyCode, ElementState, MouseScrollDelta},
    event_loop::{ControlFlow, EventLoop},
    window::{WindowBuilder, Fullscreen},
};
use wgpu::util::DeviceExt;
use glam::Vec3;
use std::{iter, collections::HashSet};

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
struct CameraUniform {
    pos: [f32; 3],
    _pad1: f32,
    dir: [f32; 3],
    _pad2: f32,
    up: [f32; 3],
    _pad3: f32,
    fov: f32,
    aspect: f32,
    resolution: [f32; 2],
}

struct Camera {
    pos: Vec3,
    yaw: f32,
    pitch: f32,
    fov: f32,
    aspect: f32,
}

impl Camera {
    fn new(aspect: f32) -> Self {
        Self {
            pos: Vec3::new(0.0, 0.0, 4.0),
            yaw: 0.0,
            pitch: 0.0,
            fov: 45f32.to_radians(),
            aspect,
        }
    }

    fn dir(&self) -> Vec3 {
        Vec3::new(
            self.yaw.cos() * self.pitch.cos(),
            self.pitch.sin(),
            self.yaw.sin() * self.pitch.cos(),
        )
        .normalize()
    }

    fn up(&self) -> Vec3 {
        Vec3::Y
    }

    fn build_uniform(&self, resolution: (f32, f32)) -> CameraUniform {
        let dir = self.dir();
        CameraUniform {
            pos: self.pos.to_array(),
            _pad1: 0.0,
            dir: dir.to_array(),
            _pad2: 0.0,
            up: self.up().to_array(),
            _pad3: 0.0,
            fov: self.fov,
            aspect: self.aspect,
            resolution: [resolution.0, resolution.1],
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Mandelbulb Fractal")
        .build(&event_loop)
        .unwrap();

    pollster::block_on(run(event_loop, window));
}

async fn run(event_loop: EventLoop<()>, window: winit::window::Window) {
    let size = window.inner_size();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        dx12_shader_compiler: Default::default(),
    });
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
                label: Some("Device"),
                features: wgpu::Features::empty(),
                limits: wgpu::Limits::default(),
            },
            None,
        )
        .await
        .unwrap();

    let surface_caps = surface.get_capabilities(&adapter);
    let surface_format = surface_caps.formats[0];
    let mut config = wgpu::SurfaceConfiguration {
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        format: surface_format,
        width: size.width,
        height: size.height,
        present_mode: wgpu::PresentMode::Fifo,
        alpha_mode: wgpu::CompositeAlphaMode::Auto,
        view_formats: vec![],
    };
    surface.configure(&device, &config);


    let mut camera = Camera::new(config.width as f32 / config.height as f32);
    let mut camera_uniform = camera.build_uniform((config.width as f32, config.height as f32));

    let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Camera Buffer"),
        contents: bytemuck::cast_slice(&[camera_uniform]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("camera_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        }],
    });

    let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("camera_bind_group"),
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

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Render Pipeline Layout"),
        bind_group_layouts: &[&camera_bind_group_layout],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: config.format,
                blend: Some(wgpu::BlendState::REPLACE),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    let mut mouse_pressed = false;
    let mut last_mouse_pos: Option<winit::dpi::PhysicalPosition<f64>> = None;
    let zoom_speed = 0.5;
    let mut fullscreen = false;

    let mut pressed_keys = HashSet::new();
    let move_speed = 0.05f32;

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,

                WindowEvent::Resized(new_size) => {
                    config.width = new_size.width;
                    config.height = new_size.height;
                    camera.aspect = new_size.width as f32 / new_size.height as f32;
                    surface.configure(&device, &config);
                }

                WindowEvent::KeyboardInput { input, .. } => {
                    if let Some(keycode) = input.virtual_keycode {
                        match input.state {
                            ElementState::Pressed => {
                                pressed_keys.insert(keycode);
                                match keycode {
                                    VirtualKeyCode::Escape => *control_flow = ControlFlow::Exit,
                                    VirtualKeyCode::F11 => {
                                        if fullscreen {
                                            window.set_fullscreen(None);
                                            fullscreen = false;
                                        } else {
                                            window.set_fullscreen(Some(Fullscreen::Borderless(None)));
                                            fullscreen = true;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            ElementState::Released => {
                                pressed_keys.remove(&keycode);
                            }
                        }
                    }
                }

                WindowEvent::MouseInput { state, button, .. } => {
                    if button == winit::event::MouseButton::Left {
                        mouse_pressed = state == ElementState::Pressed;
                        if !mouse_pressed {
                            last_mouse_pos = None;
                        }
                    }
                }

                WindowEvent::CursorMoved { position, .. } => {
                    if mouse_pressed {
                        if let Some(last_pos) = last_mouse_pos {
                            let dx = position.x - last_pos.x;
                            let dy = position.y - last_pos.y;

                            camera.yaw += (dx as f32) * 0.005;
                            camera.pitch -= (dy as f32) * 0.005;
                            camera.pitch = camera.pitch.clamp(-1.5, 1.5);
                        }
                        last_mouse_pos = Some(position);
                    }
                }

                WindowEvent::MouseWheel { delta, .. } => {
                    let scroll_amount = match delta {
                        MouseScrollDelta::LineDelta(_, y) => y as f32,
                        MouseScrollDelta::PixelDelta(pos) => pos.y as f32 * 0.1,
                    };
                    let zoom_vec = camera.dir() * scroll_amount * zoom_speed;
                    camera.pos += zoom_vec;
                }

                _ => {}
            },

            Event::MainEventsCleared => {
                // Обработка клавиш WASD + QE
                let forward = camera.dir();
                let right = forward.cross(camera.up()).normalize();
                let up = camera.up();

                if pressed_keys.contains(&VirtualKeyCode::W) {
                    camera.pos += forward * move_speed;
                }
                if pressed_keys.contains(&VirtualKeyCode::S) {
                    camera.pos -= forward * move_speed;
                }
                if pressed_keys.contains(&VirtualKeyCode::A) {
                    camera.pos -= right * move_speed;
                }
                if pressed_keys.contains(&VirtualKeyCode::D) {
                    camera.pos += right * move_speed;
                }
                if pressed_keys.contains(&VirtualKeyCode::Q) {
                    camera.pos -= up * move_speed;
                }
                if pressed_keys.contains(&VirtualKeyCode::E) {
                    camera.pos += up * move_speed;
                }

                camera_uniform = camera.build_uniform((config.width as f32, config.height as f32));
                queue.write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

                let frame = match surface.get_current_texture() {
                    Ok(frame) => frame,
                    Err(_) => {
                        surface.configure(&device, &config);
                        surface.get_current_texture().expect("Failed to acquire next swap chain texture!")
                    }
                };
                let view = frame.texture.create_view(&wgpu::TextureViewDescriptor::default());

                let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

                {
                    let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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

                    rpass.set_pipeline(&render_pipeline);
                    rpass.set_bind_group(0, &camera_bind_group, &[]);
                    rpass.draw(0..6, 0..1);
                }

                queue.submit(iter::once(encoder.finish()));
                frame.present();
            }

            _ => {}
        }
    });
}
