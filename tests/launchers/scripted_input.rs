use crate::common::{NesTestFixture, read_nametable_text};
use capynes_lib::ctrl::Buttons;

const SCREEN_MAX_CPU_CYCLES: u64 = 80_000_000;
const SCREEN_TEXT_CHECK_PERIOD: u64 = 20_000;

pub fn run_read_joy3_result_test(path: &str, success_marker: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut next_text_check = SCREEN_TEXT_CHECK_PERIOD;
    let success_marker = success_marker.to_ascii_lowercase();

    while total_cpu_cycles < SCREEN_MAX_CPU_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        if total_cpu_cycles < next_text_check {
            continue;
        }

        let text = read_nametable_text(&mut nes);
        let text_lower = text.to_ascii_lowercase();

        if text_lower.contains(&success_marker) {
            return;
        }

        if text_lower.contains("failed") || text_lower.contains("internal error") {
            panic!(
                "read_joy3 ROM failed after {} CPU cycles.\nScreen dump:\n{}",
                total_cpu_cycles, text
            );
        }

        next_text_check += SCREEN_TEXT_CHECK_PERIOD;
    }

    let text = read_nametable_text(&mut nes);
    panic!(
        "read_joy3 ROM timed out after {} CPU cycles.\nScreen dump:\n{}",
        total_cpu_cycles, text
    );
}

pub fn run_read_joy3_button_sequence_test(path: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    const HOLD_CYCLES: u64 = 100_000;
    const BUTTON_SEQUENCE: [(&str, Buttons); 8] = [
        ("A", Buttons::A),
        ("B", Buttons::B),
        ("Select", Buttons::SELECT),
        ("Start", Buttons::START),
        ("Up", Buttons::UP),
        ("Down", Buttons::DOWN),
        ("Left", Buttons::LEFT),
        ("Right", Buttons::RIGHT),
    ];

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut next_text_check = SCREEN_TEXT_CHECK_PERIOD;
    let mut button_index = 0usize;
    let mut pressed_button: Option<Buttons> = None;
    let mut button_was_pressed = false;
    let mut release_at = 0u64;

    fn current_button_prompt<'a>(text: &'a str, sequence: &[(&str, Buttons)]) -> Option<&'a str> {
        text.lines()
            .filter_map(|line| {
                let trimmed = line.trim();
                sequence
                    .iter()
                    .find(|(label, _)| trimmed == *label)
                    .map(|_| trimmed)
            })
            .last()
    }

    while total_cpu_cycles < SCREEN_MAX_CPU_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        if total_cpu_cycles < next_text_check {
            continue;
        }

        let text = read_nametable_text(&mut nes);
        let text_lower = text.to_ascii_lowercase();
        if text_lower.contains("passed") {
            return;
        }
        if text_lower.contains("failed") || text_lower.contains("internal error") {
            panic!(
                "read_joy3 test_buttons failed after {} CPU cycles.\nScreen dump:\n{}",
                total_cpu_cycles, text
            );
        }

        if button_index < BUTTON_SEQUENCE.len() {
            let (label, button) = BUTTON_SEQUENCE[button_index];
            let prompt_visible = current_button_prompt(&text, &BUTTON_SEQUENCE) == Some(label);

            if prompt_visible {
                if pressed_button.is_none() && !button_was_pressed {
                    nes.set_controller1(button);
                    pressed_button = Some(button);
                    release_at = total_cpu_cycles + HOLD_CYCLES;
                } else if total_cpu_cycles >= release_at {
                    nes.set_controller1(Buttons::empty());
                    pressed_button = None;
                    button_was_pressed = true;
                }
            } else if button_was_pressed && pressed_button.is_none() {
                button_index += 1;
                button_was_pressed = false;
            }
        }

        next_text_check += SCREEN_TEXT_CHECK_PERIOD;
    }

    let text = read_nametable_text(&mut nes);
    panic!(
        "read_joy3 test_buttons timed out after {} CPU cycles.\nScreen dump:\n{}",
        total_cpu_cycles, text
    );
}

#[allow(dead_code)]
pub fn run_nestress_menu_probe(path: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    const TAP_HOLD_CYCLES: u64 = 70_000;
    const TAP_GAP_CYCLES: u64 = 170_000;
    const NESTRESS_TEXT_CHECK_PERIOD: u64 = 2_000_000;

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut next_text_check = NESTRESS_TEXT_CHECK_PERIOD;

    // Default menu item is "PPU Test". Move down twice to "CPU Test (Code)",
    // then press A to start.
    let mut stage = 0u8;
    let mut stage_deadline = 0u64;

    while total_cpu_cycles < SCREEN_MAX_CPU_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        if stage_deadline != 0 && total_cpu_cycles >= stage_deadline {
            nes.set_controller1(Buttons::empty());
            stage_deadline = 0;
            stage = stage.saturating_add(1);
        }

        if stage_deadline == 0 {
            match stage {
                // Let menu settle.
                0 if total_cpu_cycles >= 600_000 => {
                    nes.set_controller1(Buttons::DOWN);
                    stage_deadline = total_cpu_cycles + TAP_HOLD_CYCLES;
                }
                1 if total_cpu_cycles >= 600_000 + TAP_HOLD_CYCLES + TAP_GAP_CYCLES => {
                    nes.set_controller1(Buttons::DOWN);
                    stage_deadline = total_cpu_cycles + TAP_HOLD_CYCLES;
                }
                2 if total_cpu_cycles >= 600_000 + 2 * (TAP_HOLD_CYCLES + TAP_GAP_CYCLES) => {
                    nes.set_controller1(Buttons::A);
                    stage_deadline = total_cpu_cycles + TAP_HOLD_CYCLES;
                }
                _ => {}
            }
        }

        if total_cpu_cycles < next_text_check {
            continue;
        }

        let text = read_nametable_text(&mut nes);
        let text_lower = text.to_ascii_lowercase();

        if text_lower.contains("score: 11/11") || text_lower.contains("score:11/11") {
            return;
        }

        if text_lower.contains("pass") || text_lower.contains("success") {
            return;
        }

        if text_lower.contains("fail") || text_lower.contains("error") {
            panic!(
                "NEStress scripted menu probe failed after {} CPU cycles.\nScreen dump:\n{}",
                total_cpu_cycles, text
            );
        }

        next_text_check += NESTRESS_TEXT_CHECK_PERIOD;
    }

    let text = read_nametable_text(&mut nes);
    panic!(
        "NEStress scripted menu probe timed out after {} CPU cycles.\nScreen dump:\n{}",
        total_cpu_cycles, text
    );
}

#[allow(dead_code)]
pub fn run_button_sweep_screen_probe(path: &str) {
    let mut nes = NesTestFixture::new(path);
    nes.reset();

    const TEXT_CHECK_PERIOD: u64 = 200_000;
    const HOLD_CYCLES: u64 = 90_000;
    const GAP_CYCLES: u64 = 220_000;
    const BUTTON_SEQUENCE: [Buttons; 8] = [
        Buttons::START,
        Buttons::A,
        Buttons::B,
        Buttons::SELECT,
        Buttons::UP,
        Buttons::DOWN,
        Buttons::LEFT,
        Buttons::RIGHT,
    ];

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut next_text_check = TEXT_CHECK_PERIOD;
    let mut step = 0usize;
    let mut pressed = false;
    let mut stage_deadline = 600_000u64;

    while total_cpu_cycles < SCREEN_MAX_CPU_CYCLES {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        if total_cpu_cycles >= stage_deadline {
            if pressed {
                nes.set_controller1(Buttons::empty());
                pressed = false;
                stage_deadline = total_cpu_cycles + GAP_CYCLES;
                step = step.saturating_add(1);
            } else if step < BUTTON_SEQUENCE.len() * 3 {
                let button = BUTTON_SEQUENCE[step % BUTTON_SEQUENCE.len()];
                nes.set_controller1(button);
                pressed = true;
                stage_deadline = total_cpu_cycles + HOLD_CYCLES;
            } else {
                stage_deadline = SCREEN_MAX_CPU_CYCLES + 1;
            }
        }

        if total_cpu_cycles < next_text_check {
            continue;
        }

        let text = read_nametable_text(&mut nes);
        let lower = text.to_ascii_lowercase();
        if lower.contains("pass") || lower.contains("passed") || lower.contains("success") {
            nes.set_controller1(Buttons::empty());
            return;
        }
        if lower.contains("fail") || lower.contains("failed") || lower.contains("error") {
            panic!(
                "Scripted button sweep probe failed after {} CPU cycles.\nScreen dump:\n{}",
                total_cpu_cycles, text
            );
        }

        next_text_check += TEXT_CHECK_PERIOD;
    }

    nes.set_controller1(Buttons::empty());
    let text = read_nametable_text(&mut nes);
    panic!(
        "Scripted button sweep probe timed out after {} CPU cycles.\nScreen dump:\n{}",
        total_cpu_cycles, text
    );
}
