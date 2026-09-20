#[path = "common/mod.rs"]
mod common;

use capynes_lib::ctrl::Buttons;
use crate::common::{NesTestFixture, read_nametable_text};

fn mmc1_a12_signature(screen: &str) -> String {
    screen
        .lines()
        .skip(13)
        .take(4)
        .collect::<Vec<_>>()
        .join("\n")
}

fn mmc1_a12_runtime_signature(
    nes: &mut NesTestFixture,
) -> (u16, u8, u8, u8, u8, u8, u8, u8, u8, u8) {
    (
        nes.cpu.pc,
        nes.cpu.acc,
        nes.cpu.rx,
        nes.cpu.ry,
        nes.cpu.flags.bits(),
        nes.cpu.sp,
        nes.cpu.read::<u8>(0x6000),
        nes.cpu.read::<u8>(0x6001),
        nes.cpu.read::<u8>(0x6002),
        nes.cpu.read::<u8>(0x6003),
    )
}

fn mmc1_a12_runtime_core_signature(nes: &mut NesTestFixture) -> (u8, u8, u8, u8, u8, u8, u8, u8) {
    (
        nes.cpu.acc,
        nes.cpu.rx,
        nes.cpu.flags.bits(),
        nes.cpu.sp,
        nes.cpu.read::<u8>(0x6000),
        nes.cpu.read::<u8>(0x6001),
        nes.cpu.read::<u8>(0x6002),
        nes.cpu.read::<u8>(0x6003),
    )
}

fn frame_rgba_signature(nes: &NesTestFixture) -> u64 {
    let mut rgba = vec![0u8; 256 * 240 * 4];
    nes.ppu.borrow().frame().render(&mut rgba);

    let mut hash = 0xcbf29ce484222325u64;
    for &b in &rgba {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn run_visual_frame_oracle(path: &str, target_cycles: u64) -> u64 {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    while total_cpu_cycles < target_cycles {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
    }

    frame_rgba_signature(&nes)
}

macro_rules! oracle_visual_frame_manifest {
    ($(($name:ident, $path:expr, $target_cycles:expr, $signature:expr)),* $(,)?) => {
        $(
            #[test]
            fn $name() {
                assert_eq!(run_visual_frame_oracle($path, $target_cycles), $signature);
            }
        )*
    };
}

include!("test_roms/oracle/manifest.rs");
