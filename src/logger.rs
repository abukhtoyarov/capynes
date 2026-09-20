#[cfg(target_arch = "wasm32")]
pub fn init_logging() {
    use log::Level;
    console_error_panic_hook::set_once();
    console_log::init_with_level(Level::Info).expect("could not initialize console_log");
}

#[cfg(not(target_arch = "wasm32"))]
pub fn init_logging() {
    env_logger::Builder::from_default_env()
        .filter(Some("wgpu"), log::LevelFilter::Off) // вырубаем именно wgpu
        .filter(Some("winit"), log::LevelFilter::Off) // вырубаем именно wgpu
        .filter(Some("naga"), log::LevelFilter::Off) // вырубаем именно wgpu
        .init();
}
