use capynes_lib::{snake, logger};

fn main() {
    logger::init_logging();
    log::info!("+ Snake application started");
    snake::run();
}
