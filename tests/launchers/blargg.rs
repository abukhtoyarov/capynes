use crate::common::{NesTestFixture, run_blargg_test};
use capynes_lib::rom::Mmc3IrqRevision;

const BLARGG_MAX_CPU_CYCLES: u64 = 150_000_000;

pub fn run_blargg_rom_test(path: &str, allow_reset: bool) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    run_blargg_test(&mut nes, BLARGG_MAX_CPU_CYCLES, allow_reset).unwrap_or_else(|e| {
        eprintln!("{}", e);
        panic!("Test failed");
    });
}

#[allow(dead_code)]
pub fn run_blargg_rom_test_mmc3_rev_a(path: &str, allow_reset: bool) {
    let mut nes = NesTestFixture::new_with_mmc3_irq_revision(path, Mmc3IrqRevision::RevA);
    nes.reset();

    run_blargg_test(&mut nes, BLARGG_MAX_CPU_CYCLES, allow_reset).unwrap_or_else(|e| {
        eprintln!("{}", e);
        panic!("Test failed");
    });
}
