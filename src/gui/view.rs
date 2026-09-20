use wgpu::util::DeviceExt;
use winit::event_loop::EventLoop;
use winit::window::Window;
use thiserror::Error;
use std::rc::Rc;
use std::panic;
use super::window;
use super::utils::Vertex;
use super::material::Canvas;
use triple_buffer::Output;

#[derive(Error, Debug)]
pub enum ViewError {
    #[error("Failed to create surface")]
    SurfaceCreation,
    #[error("Failed to request adapter")]
    AdapterRequest,
    #[error("Failed to request device")]
    DeviceRequest,
    #[error("Failed to get current texture")]
    GetCurrentTexture,
    #[error("Failed to render")]
    RenderError
}

pub struct View {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
    pub canvas: Canvas,
    vertex_buffer: wgpu::Buffer,
}

pub const VERTICES: &[Vertex] = &[
    Vertex { pos: [-1.0, -1.0] },
    Vertex { pos: [3.0, -1.0] },
    Vertex { pos: [-1.0, 3.0] },
];

impl View {
    async fn initialize_wgpu(window: &Window, texture_format: wgpu::TextureFormat)
        -> Result<(wgpu::Surface, wgpu::Device,
                    wgpu::SurfaceConfiguration, wgpu::Queue), ViewError> 
    {
        log::info!("+ Initializing WGPU context...");
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            dx12_shader_compiler: Default::default(),
        });

        let surface = unsafe {
            instance.create_surface(window).map_err(|_| ViewError::SurfaceCreation)?
        };

        let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: Some(&surface),
            force_fallback_adapter: false,
        }).await.ok_or(ViewError::AdapterRequest)?;

        let (device, queue) = adapter.request_device(&wgpu::DeviceDescriptor {
            label: None,
            features: wgpu::Features::empty(),
            limits: wgpu::Limits::downlevel_webgl2_defaults(),
        }, None).await.map_err(|_| ViewError::DeviceRequest)?;

        let size = window.inner_size();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: texture_format,
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
        };
        surface.configure(&device, &config);

        Ok((surface, device, config, queue))
    }
    
    async fn new(
        window: &Window,
        size: &winit::dpi::PhysicalSize<u32>,
        texture_format: wgpu::TextureFormat
        ) -> Result<Self, ViewError> {
        let (surface, device, config, queue) = View::initialize_wgpu(&window, texture_format).await?;
        let canvas = Canvas::new(&device, &config, size);

        log::info!("+ Initializing vertex buffer...");
        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(&VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        Ok(Self {
            surface,
            device,
            queue,
            config,
            canvas,
            vertex_buffer,
        })
    }

    pub fn create(
        event_loop: &EventLoop<()>,
        title: &str,
        window_size: winit::dpi::LogicalSize<u32>,
        render_size: winit::dpi::PhysicalSize<u32>,
        texture_format: wgpu::TextureFormat
        ) -> (Rc<Window>, View) 
    {
        let window = window::new(event_loop, window_size, title);
        let window = Rc::new(window);
        let view = pollster::block_on(View::new(&window, &render_size, texture_format))
            .expect("Failed to initialize view");
        (window, view)
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    pub fn update_from(&mut self, buf: &mut Output<Vec<u8>>) {
        self.canvas.write(&self.queue, buf.read());
    }

    pub fn render(&mut self) -> Result<(), ViewError> {
        let output = self.surface.get_current_texture()
            .or_else(|_| {
                self.surface.configure(&self.device, &self.config);
                self.surface.get_current_texture()
            })
            .map_err(|_| ViewError::GetCurrentTexture)?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });

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

            rpass.set_pipeline(&self.canvas.pipeline.pipeline);
            rpass.set_bind_group(0, &self.canvas.bind_group, &[]);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            rpass.draw(0..VERTICES.len() as u32, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));

        if let Err(e) = panic::catch_unwind(panic::AssertUnwindSafe(|| {
            output.present();
        })) {
            eprintln!("Failed to present frame: {:?}", e);
            return Err(ViewError::RenderError);
        }

        Ok(())
    }
}
