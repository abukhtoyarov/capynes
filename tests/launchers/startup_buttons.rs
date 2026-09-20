use crate::common::{NesTestFixture, read_nametable_text};
use capynes_lib::ctrl::Buttons;

const SCREEN_TEXT_CHECK_PERIOD: u64 = 20_000;
const CPU_TIMING_TEST6_MAX_CPU_CYCLES: u64 = 60_000_000;
const CPU_TIMING_TEST6_STARTUP_BUTTON_HOLD_CYCLES: u64 = 200_000;

fn read_cpu_timing_test6_diag(nes: &NesTestFixture) -> String {
    let opcode = nes.cpu.read::<u8>(0x0013);
    let test_mode = nes.cpu.read::<u8>(0x0014);
    let measured = nes.cpu.read::<u8>(0x0002);
    let expected = nes.cpu.read::<u8>(0x0003);
    let counter_lo = nes.cpu.read::<u8>(0x0010);
    let counter_hi = nes.cpu.read::<u8>(0x0011);

    format!(
        "opcode=${:02X}, mode={}, measured={}, expected={}, counter=${:02X}{:02X}, pc=${:04X}",
        opcode,
        if test_mode == 2 { "no-cross" } else { "cross" },
        measured,
        expected,
        counter_hi,
        counter_lo,
        nes.cpu.pc
    )
}

pub fn run_cpu_timing_test6(path: &str, buttons: Buttons, mode_name: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();
    nes.set_controller1(buttons);

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut next_text_check = SCREEN_TEXT_CHECK_PERIOD;

    while total_cpu_cycles < CPU_TIMING_TEST6_MAX_CPU_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        if total_cpu_cycles >= CPU_TIMING_TEST6_STARTUP_BUTTON_HOLD_CYCLES {
            nes.set_controller1(Buttons::empty());
        }

        if total_cpu_cycles < next_text_check {
            continue;
        }

        let text = read_nametable_text(&mut nes);

        if text.contains("PASSED") {
            return;
        }

        if text.contains("FAIL OP :")
            || text.contains("UNKNOWN ERROR")
            || text.contains("BASIC TIMING WRONG")
        {
            for _ in 0..200_000 {
                total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;
            }
            let text = read_nametable_text(&mut nes);
            let diag = read_cpu_timing_test6_diag(&nes);
            panic!(
                "cpu_timing_test6 ({}) failed after {} CPU cycles.\n{}\nScreen dump:\n{}",
                mode_name, total_cpu_cycles, diag, text
            );
        }

        next_text_check += SCREEN_TEXT_CHECK_PERIOD;
    }

    let text = read_nametable_text(&mut nes);
    panic!(
        "cpu_timing_test6 ({}) timed out after {} CPU cycles.\nScreen dump:\n{}",
        mode_name, total_cpu_cycles, text
    );
}
