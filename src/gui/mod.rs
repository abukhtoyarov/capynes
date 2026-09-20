mod utils;
mod window;
mod material;
mod view;
use winit::event::*;
use winit::event_loop::{ControlFlow, EventLoop};

type InputBuf = triple_buffer::Input<Vec<u8>>;
type OutputBuf = triple_buffer::Output<Vec<u8>>;

pub struct GUI<const WIDTH: u32, const HEIGHT: u32> {
    pub window_title: String,
    pub window_size: winit::dpi::LogicalSize<u32>,
    pub texture_format: wgpu::TextureFormat,
    inbuf: InputBuf,
    outbuf: OutputBuf,
}

impl<const WIDTH: u32, const HEIGHT: u32> GUI<WIDTH, HEIGHT> {
    const FRAME_SIZE: usize = (WIDTH as usize) * (HEIGHT as usize) * 4;
    
    pub fn new(
        window_title: impl Into<String>,
        window_size: winit::dpi::LogicalSize<u32>,
        texture_format: wgpu::TextureFormat
    ) -> Self {
        let buffer = vec![0u8; Self::FRAME_SIZE];
        let (inbuf, outbuf) = triple_buffer::triple_buffer(&buffer);
        
        Self {
            window_title: window_title.into(),
            window_size,
            texture_format,
            inbuf,
            outbuf
        }
    }
    
    pub const fn render_size() -> winit::dpi::PhysicalSize<u32> {
        winit::dpi::PhysicalSize::new(WIDTH, HEIGHT)
    }
    
    pub fn run<F, T>(mut self, working_thread: F, control_handler: T)
    where
        F: Fn(&mut InputBuf) -> bool + Send + 'static,
        T: Fn(KeyboardInput) -> ControlFlow + 'static,
    {
        let event_loop = EventLoop::new();
        let render_size = Self::render_size();
        let (window, mut view) = view::View::create(
            &event_loop, 
            &self.window_title, 
            self.window_size,
            render_size,
            self.texture_format
        );
        
        let mut inbuf = self.inbuf;
        std::thread::spawn(move || {
            loop { working_thread(&mut inbuf); }
        });
        
        log::info!("+ Running event loop...");
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Poll;
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
                    WindowEvent::Resized(new_size) => { view.resize(new_size); }
                    WindowEvent::KeyboardInput { input, .. } => *control_flow = control_handler(input),
                    _ => {}
                },
                Event::RedrawRequested(_) | Event::MainEventsCleared => {
                    view.update_from(&mut self.outbuf);
                    if let Err(e) = view.render() {
                        eprintln!("Render error: {:?}", e);
                    }
                    window.request_redraw();
                },
                _ => {}
            }
        });
    }
}
