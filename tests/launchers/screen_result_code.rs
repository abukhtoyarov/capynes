use crate::common::{NesTestFixture, run_screen_result_code_test};

const SCREEN_MAX_CPU_CYCLES: u64 = 80_000_000;
const SCREEN_TEXT_CHECK_PERIOD: u64 = 20_000;

pub fn run_screen_result_code_rom_test(path: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    run_screen_result_code_test(&mut nes, SCREEN_MAX_CPU_CYCLES, SCREEN_TEXT_CHECK_PERIOD)
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            panic!("Test failed");
        });
}
