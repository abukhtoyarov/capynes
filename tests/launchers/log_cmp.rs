use crate::common::{NesTestFixture, run_log_test};

pub fn run_log_cmp_test(rom_path: &str, log_path: &str) {
    let mut nes = NesTestFixture::new(rom_path);
    nes.setup_nestest();

    run_log_test(&mut nes, log_path).unwrap_or_else(|e| {
        eprintln!("{}", e);
        panic!("Test failed");
    });
}
