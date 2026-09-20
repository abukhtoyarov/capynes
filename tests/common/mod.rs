use capynes_lib::rom::{Mmc3IrqRevision, Rom};
use capynes_lib::ppu::Ppu;
use capynes_lib::apu::Apu;
use capynes_lib::mmio::NesMmio;
use capynes_lib::cpu::cpu::Cpu;
use capynes_lib::ctrl::{Buttons, Controller, ControllerState, create_controller_state};
use std::path::Path;
use std::sync::atomic::Ordering;

pub struct NesTestFixture {
    pub cpu: Cpu,
    pub apu: std::rc::Rc<std::cell::RefCell<Apu>>,
    pub ppu: std::rc::Rc<std::cell::RefCell<Ppu>>,
    #[allow(dead_code)]
    pub controller1: ControllerState,
}

struct InterruptEdges {
    first_nmi: Option<usize>,
    first_irq: Option<usize>,
}

impl NesTestFixture {
    pub fn new<P: AsRef<Path>>(rom_path: P) -> Self {
        Self::new_internal(rom_path, None)
    }

    #[allow(dead_code)]
    pub fn new_with_mmc3_irq_revision<P: AsRef<Path>>(
        rom_path: P,
        revision: Mmc3IrqRevision,
    ) -> Self {
        Self::new_internal(rom_path, Some(revision))
    }

    fn new_internal<P: AsRef<Path>>(rom_path: P, mmc3_irq_revision: Option<Mmc3IrqRevision>) -> Self {
        let path_str = rom_path.as_ref()
            .to_str()
            .expect("Invalid UTF-8 in path");

        let rom = Rom::new(path_str).expect("Cannot load rom");
        if let Some(revision) = mmc3_irq_revision {
            rom.mapper
                .borrow_mut()
                .set_mmc3_irq_revision(revision);
        }
        let apu = Apu::new(rom.clone());
        if path_str.contains("/pal_apu_tests/") {
            apu.borrow_mut().set_pal_frame_counter_profile(true);
        }
        let ppu = Ppu::new(rom.clone());

        
        let js1 = create_controller_state();
        let j1 = Controller::new(js1.clone());
        let mmio = NesMmio::new(rom, apu.clone(), ppu.clone(), j1.clone(), j1.clone());
        let cpu = Cpu::with_bus(mmio);
        
        Self {
            cpu,
            ppu,
            apu,
            controller1: js1,
        }
    }

    pub fn reset(&mut self) {
        self.cpu.reset();
        self.ppu.borrow_mut().reset();
        self.apu.borrow_mut().reset();
    }

    pub fn console_reset(&mut self) {
        self.cpu.console_reset();
        self.ppu.borrow_mut().reset();
        self.apu.borrow_mut().console_reset();
    }

    #[allow(dead_code)]
    pub fn setup_nestest(&mut self) {
        self.reset();
        self.cpu.cycles = 7;
        self.ppu.borrow_mut().cycles = 21;
        self.cpu.pc = 0xC000;
    }

    #[allow(dead_code)]
    pub fn get_state(&self) -> String {
        format!(
            "{} PPU:{: >3},{: >3} CYC:{}",
            self.cpu,
            self.ppu.borrow().scanlines,
            self.ppu.borrow().cycles,
            self.cpu.cycles
        )
    }

    #[allow(dead_code)]
    pub fn step(&mut self) -> u8 {
        let cycles = self.cpu.step();
        self.ppu.borrow_mut().step_cycles(cycles as usize * 3);
        cycles as u8
    }

    #[allow(dead_code)]
    pub fn step_headless(&mut self, pending_nmi: &mut bool) -> usize {
        let mut total = 0usize;

        let cycles = self.cpu.step();
        total += cycles;
        let edges = self.advance_components(cycles);
        let mut fresh_nmi = edges.first_nmi;
        if let Some(cpu_cycle) = fresh_nmi {
            if self.cpu.last_opcode == 0x00 && cpu_cycle < 5 {
                self.cpu.hijack_interrupt_vector_to_nmi();
                fresh_nmi = None;
            }
        }

        let irq_became_pending_on_last_cycle =
            edges.first_irq.is_some_and(|cpu_cycle| cpu_cycle + 1 >= cycles);

        if self.cpu.bus.poll_irq()
            && !self.cpu.irq_inhibited()
            && !irq_became_pending_on_last_cycle
        {
            let irq_cycles = self.cpu.irq_interrupt();
            total += irq_cycles;
            let irq_edges = self.advance_components(irq_cycles);
            let irq_fresh_nmi = irq_edges.first_nmi;
            if let Some(cpu_cycle) = irq_fresh_nmi {
                if cpu_cycle < 5 {
                    self.cpu.hijack_interrupt_vector_to_nmi();
                } else if fresh_nmi.is_none() {
                    fresh_nmi = Some(cpu_cycle);
                }
            }
        }

        if *pending_nmi {
            let nmi_cycles = self.cpu.nmi_interrupt();
            total += nmi_cycles;
            *pending_nmi = false;
            let _ = self.advance_components(nmi_cycles);
        }

        if fresh_nmi.is_some() {
            *pending_nmi = true;
        }

        self.cpu.finish_interrupt_poll();

        total
    }

    fn advance_components(&mut self, cpu_cycles: usize) -> InterruptEdges {
        let mut first_nmi = None;
        let mut first_irq = None;

        for cpu_cycle in 0..cpu_cycles {
            self.ppu.borrow_mut().step_cycles(3);
            self.apu.borrow_mut().step_cycle();

            if first_nmi.is_none() && self.ppu.borrow_mut().poll_nmi() {
                first_nmi = Some(cpu_cycle);
            }

            if first_irq.is_none() && self.cpu.bus.poll_irq() {
                first_irq = Some(cpu_cycle);
            }
        }

        InterruptEdges {
            first_nmi,
            first_irq,
        }
    }

    #[allow(dead_code)]
    pub fn set_controller1(&self, buttons: Buttons) {
        self.controller1.store(buttons.bits(), Ordering::Relaxed);
    }
}

#[allow(dead_code)]
pub fn read_zero_terminated_cpu_text(
    nes: &NesTestFixture,
    start_addr: u16,
    max_len: usize,
) -> String {
    let mut bytes = Vec::with_capacity(max_len);
    for offset in 0..max_len {
        let b = nes.cpu.read::<u8>(start_addr.wrapping_add(offset as u16));
        if b == 0 {
            break;
        }
        bytes.push(b);
    }

    String::from_utf8_lossy(&bytes).into_owned()
}

#[allow(dead_code)]
pub fn run_blargg_test(
    nes: &mut NesTestFixture,
    max_cpu_cycles: u64,
    allow_reset: bool,
) -> Result<(), String> {
    const TEXT_ADDR: u16 = 0x6004;
    const STATUS_ADDR: u16 = 0x6000;
    const SIGNATURE_ADDR: u16 = 0x6001;
    const SIGNATURE: [u8; 3] = [0xDE, 0xB0, 0x61];
    const RESET_DELAY_CYCLES: u64 = 1_000_000;

    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut reset_requested_at = None;

    while total_cpu_cycles < max_cpu_cycles {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        let signature = [
            nes.cpu.read::<u8>(SIGNATURE_ADDR),
            nes.cpu.read::<u8>(SIGNATURE_ADDR + 1),
            nes.cpu.read::<u8>(SIGNATURE_ADDR + 2),
        ];

        if signature != SIGNATURE {
            continue;
        }

        let status = nes.cpu.read::<u8>(STATUS_ADDR);
        match status {
            0x80 => {}
            0x81 if allow_reset => {
                let requested_at = reset_requested_at.get_or_insert(total_cpu_cycles);
                if total_cpu_cycles.saturating_sub(*requested_at) >= RESET_DELAY_CYCLES {
                    nes.console_reset();
                    pending_nmi = false;
                    reset_requested_at = None;
                }
            }
            0x81 => {
                let text = read_zero_terminated_cpu_text(nes, TEXT_ADDR, 4096);
                return Err(format!(
                    "ROM requested reset after {} CPU cycles, but reset is disabled.\nText output:\n{}",
                    total_cpu_cycles, text
                ));
            }
            0x00 => return Ok(()),
            code => {
                let text = read_zero_terminated_cpu_text(nes, TEXT_ADDR, 4096);
                return Err(format!(
                    "ROM failed with code {} after {} CPU cycles.\nText output:\n{}",
                    code, total_cpu_cycles, text
                ));
            }
        }
    }

    let text = read_zero_terminated_cpu_text(nes, TEXT_ADDR, 4096);
    Err(format!(
        "ROM timed out after {} CPU cycles.\nText output:\n{}",
        total_cpu_cycles, text
    ))
}

#[allow(dead_code)]
pub fn read_nametable_text(nes: &mut NesTestFixture) -> String {
    let mut ppu = nes.ppu.borrow_mut();
    ppu.write_addr(0x20);
    ppu.write_addr(0x00);
    let _ = ppu.read();

    let mut out = String::with_capacity(32 * 31);
    for i in 0..(32 * 30) {
        let b = ppu.read();
        let c = if (32..=126).contains(&b) { b as char } else { ' ' };
        out.push(c);
        if (i + 1) % 32 == 0 {
            out.push('\n');
        }
    }
    out
}

#[allow(dead_code)]
pub fn run_screen_text_test(
    nes: &mut NesTestFixture,
    max_cpu_cycles: u64,
    text_check_period: u64,
) -> Result<(), String> {
    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut next_text_check = text_check_period;

    while total_cpu_cycles < max_cpu_cycles {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        if total_cpu_cycles < next_text_check {
            continue;
        }

        let text = read_nametable_text(nes);
        let text_lower = text.to_ascii_lowercase();

        if text_lower.contains("passed")
            || text_lower.contains("0assed")
            || text_lower.contains("all tests complete")
        {
            return Ok(());
        }

        if text_lower.contains("failed") || text_lower.contains("error") {
            return Err(format!(
                "ROM failed after {} CPU cycles.\nScreen dump:\n{}",
                total_cpu_cycles, text
            ));
        }

        next_text_check += text_check_period;
    }

    let text = read_nametable_text(nes);
    Err(format!(
        "ROM timed out after {} CPU cycles.\nScreen dump:\n{}",
        total_cpu_cycles, text
    ))
}

#[allow(dead_code)]
pub fn run_screen_result_code_test(
    nes: &mut NesTestFixture,
    max_cpu_cycles: u64,
    text_check_period: u64,
) -> Result<(), String> {
    let mut pending_nmi = false;
    let mut total_cpu_cycles = 0u64;
    let mut next_text_check = text_check_period;

    while total_cpu_cycles < max_cpu_cycles {
        total_cpu_cycles += nes.step_headless(&mut pending_nmi) as u64;

        if total_cpu_cycles < next_text_check {
            continue;
        }

        let text = read_nametable_text(nes);
        let text_lower = text.to_ascii_lowercase();

        if text_lower.contains("passed") {
            return Ok(());
        }

        if text_lower.contains("failed") {
            return Err(format!(
                "ROM failed after {} CPU cycles.\nScreen dump:\n{}",
                total_cpu_cycles, text
            ));
        }

        if let Some(code) = find_screen_hex_code(&text) {
            if code == 0x01 {
                return Ok(());
            }

            return Err(format!(
                "ROM failed with code ${:02X} after {} CPU cycles.\nScreen dump:\n{}",
                code, total_cpu_cycles, text
            ));
        }

        next_text_check += text_check_period;
    }

    let text = read_nametable_text(nes);
    Err(format!(
        "ROM timed out after {} CPU cycles.\nScreen dump:\n{}",
        total_cpu_cycles, text
    ))
}

fn find_screen_hex_code(text: &str) -> Option<u8> {
    for token in text.split_whitespace() {
        if token.len() == 3
            && token.starts_with('$')
            && token[1..].chars().all(|c| c.is_ascii_hexdigit())
        {
            if let Ok(code) = u8::from_str_radix(&token[1..], 16) {
                return Some(code);
            }
        }
    }

    None
}

#[allow(dead_code)]
pub fn run_log_test<P: AsRef<Path>>(
    nes: &mut NesTestFixture,
    log_path: P,
) -> Result<(), String> {
    use std::fs::File;
    use std::io::{BufRead, BufReader};

    let log_file = File::open(log_path)
        .map_err(|e| format!("Cannot open log file: {}", e))?;
    let reader = BufReader::new(log_file);

    for (line_num, line_read) in reader.lines().enumerate() {
        let line = line_read
            .map_err(|e| format!("Cannot read line {}: {}", line_num + 1, e))?;
        let line = line.trim();
        let state = nes.get_state();
        
        if line != state {
            return Err(format!(
                "Mismatch at line {}:\nExpected: {}\nGot:      {}",
                line_num + 1,
                line,
                state
            ));
        }
        
        nes.step();
    }

    Ok(())
}
