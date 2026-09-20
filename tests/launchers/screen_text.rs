use crate::common::{NesTestFixture, run_screen_text_test};
use capynes_lib::rom::Mmc3IrqRevision;

const SCREEN_MAX_CPU_CYCLES: u64 = 80_000_000;
#[allow(dead_code)]
const TEXT_CHECK_PERIOD: u64 = 20_000;
#[allow(dead_code)]
const LOW_INTRUSION_TEXT_CHECK_PERIOD: u64 = 2_000_000;

#[allow(dead_code)]
pub fn run_screen_rom_test(path: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    run_screen_text_test(&mut nes, SCREEN_MAX_CPU_CYCLES, TEXT_CHECK_PERIOD).unwrap_or_else(|e| {
        eprintln!("{}", e);
        panic!("Test failed");
    });
}

#[allow(dead_code)]
pub fn run_screen_rom_test_mmc3_rev_a(path: &str) {
    let mut nes = NesTestFixture::new_with_mmc3_irq_revision(path, Mmc3IrqRevision::RevA);
    nes.reset();

    run_screen_text_test(&mut nes, SCREEN_MAX_CPU_CYCLES, TEXT_CHECK_PERIOD).unwrap_or_else(|e| {
        eprintln!("{}", e);
        panic!("Test failed");
    });
}

#[allow(dead_code)]
pub fn run_low_intrusion_screen_rom_test(path: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    run_screen_text_test(&mut nes, SCREEN_MAX_CPU_CYCLES, LOW_INTRUSION_TEXT_CHECK_PERIOD)
        .unwrap_or_else(|e| {
            eprintln!("{}", e);
            panic!("Test failed");
        });
}
