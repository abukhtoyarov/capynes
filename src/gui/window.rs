use cfg_if::cfg_if;
use winit::event_loop::EventLoop;
use winit::window::Window;
use winit::window::WindowBuilder;

pub fn new(event_loop: &EventLoop<()>, size: winit::dpi::LogicalSize<u32>,  _title: &str) -> Window {
    log::info!("+ Creating window...");
    cfg_if! {
        if #[cfg(target_arch = "wasm32")] {
            use winit::platform::web::WindowBuilderExtWebSys;
            use wasm_bindgen::JsCast;

            let canvas = web_sys::window().unwrap()
                .document().unwrap()
                .get_element_by_id("wgpu-canvas")
                .expect("Can't find <canvas id=\"wgpu-canvas\">")
                .dyn_into::<web_sys::HtmlCanvasElement>()
                .unwrap();

            let window = WindowBuilder::new()
                .with_canvas(Some(canvas.clone()))
                .build(event_loop)
                .unwrap();

            canvas.set_width(size.width);
            canvas.set_height(size.height);

            canvas.style().set_property("width", &format!("{}px", size.width)).expect("Cannot set <canvas> size");
            canvas.style().set_property("height", &format!("{}px", size.height)).expect("Cannot set <canvas> size");

            window
        } else {
            WindowBuilder::new()
                .with_title(_title)
                .with_inner_size(size)
                .build(event_loop)
                .unwrap()
        }
    }
}
