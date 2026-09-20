use crate::rom::RomRef;
use crate::rom::info::TVFormat;
use bitflags::bitflags;

/*
 *              ┌───────────────────────────────────┐
 *              │            APU (2A03)             │
 *              │                                   │
 *    CPU → MMIO│ ┌────────────┐  ┌──────────────┐  │
 *    Registers │ │ Control &  │  │ Frame Counter│  │
 *    ($4000–17)│ │ Channel    │  │ & IRQ        │  │
 *              │ │ Config     │  │              │  │
 *              │ └─────┬──────┘  └────┬─────────┘  │
 *              │       │              │            │
 *              │       ▼              ▼            │
 *              │ ┌───────────────────────────────┐ │
 *              │ │   Sound Channels:             │ │
 *              │ │   • Pulse 1 & 2               │ │
 *              │ │   • Triangle                  │ │
 *              │ │   • Noise                     │ │
 *              │ │   • DMC (Delta Modulation)    │ │
 *              │ └───────────┬───────────────────┘ │
 *              │             │                     │
 *              │             ▼                     │
 *              │      ┌─────────────┐              │
 *              │      │   Mixer     │              │
 *              │      └──────┬──────┘              │
 *              │             ▼                     │
 *              │      Audio Output (40-48kHz)      │
 *              └───────────────────────────────────┘
 */

// Pulse channel (есть два: Pulse 1 и Pulse 2)
#[derive(Debug, Clone)]
struct PulseChannel {
    enabled: bool,
    
    // Envelope
    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
    envelope_loop: bool,
    constant_volume: bool,
    volume: u8,
    
    // Sweep
    sweep_enabled: bool,
    sweep_period: u8,
    sweep_negate: bool,
    sweep_shift: u8,
    sweep_reload: bool,
    sweep_divider: u8,
    
    // Timer
    timer_period: u16,
    timer_counter: u16,
    
    // Sequencer
    duty_cycle: u8,
    sequence_counter: u8,
    
    // Length counter
    length_counter: u8,
    length_halt: bool,
    pending_length_halt: Option<bool>,
    pending_length_reload: Option<u8>,
}

impl PulseChannel {
    pub fn new() -> Self {
        PulseChannel {
            enabled: false,
            envelope_start: false,
            envelope_divider: 0,
            envelope_decay: 0,
            envelope_loop: false,
            constant_volume: false,
            volume: 0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_negate: false,
            sweep_shift: 0,
            sweep_reload: false,
            sweep_divider: 0,
            timer_period: 0,
            timer_counter: 0,
            duty_cycle: 0,
            sequence_counter: 0,
            length_counter: 0,
            length_halt: false,
            pending_length_halt: None,
            pending_length_reload: None,
        }
    }
    
    pub fn clock_timer(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period.saturating_sub(1);
            self.sequence_counter = (self.sequence_counter + 1) % 8;
        } else {
            self.timer_counter -= 1;
        }
    }
    
    pub fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_decay = 15;
            self.envelope_divider = self.volume;
            self.envelope_start = false;
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.volume;
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if self.envelope_loop {
                self.envelope_decay = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
    }
    
    pub fn clock_sweep(&mut self, is_pulse1: bool) {
        // Calculate target period
        let change_amount = self.timer_period >> self.sweep_shift;
        let target_period = if self.sweep_negate {
            let complement = if is_pulse1 {
                // Pulse 1 uses one's complement
                self.timer_period.wrapping_sub(change_amount).wrapping_sub(1)
            } else {
                // Pulse 2 uses two's complement
                self.timer_period.wrapping_sub(change_amount)
            };
            complement
        } else {
            self.timer_period.wrapping_add(change_amount)
        };
        
        // Muting conditions
        let muting = self.timer_period < 8 || target_period > 0x7FF;
        
        // Update divider
        if self.sweep_divider == 0 && self.sweep_enabled && !muting && self.sweep_shift != 0 {
            self.timer_period = target_period;
        }
        
        if self.sweep_divider == 0 || self.sweep_reload {
            self.sweep_divider = self.sweep_period;
            self.sweep_reload = false;
        } else {
            self.sweep_divider -= 1;
        }
    }
    
    pub fn clock_length(&mut self) {
        if !self.length_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    pub fn output(&self) -> u8 {
        const DUTY_TABLE: [[u8; 8]; 4] = [
            [0, 1, 0, 0, 0, 0, 0, 0], // 12.5%
            [0, 1, 1, 0, 0, 0, 0, 0], // 25%
            [0, 1, 1, 1, 1, 0, 0, 0], // 50%
            [1, 0, 0, 1, 1, 1, 1, 1], // 25% negated
        ];
        
        if !self.enabled || self.length_counter == 0 || self.timer_period < 8 || self.timer_period > 0x7FF {
            return 0;
        }
        
        let duty_bit = DUTY_TABLE[self.duty_cycle as usize][self.sequence_counter as usize];
        if duty_bit == 0 {
            return 0;
        }
        
        if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        }
    }
}

// Triangle channel
#[derive(Debug, Clone)]
struct TriangleChannel {
    enabled: bool,
    
    // Linear counter
    linear_counter: u8,
    linear_counter_reload: u8,
    linear_counter_reload_flag: bool,
    control_flag: bool,
    
    // Timer
    timer_period: u16,
    timer_counter: u16,
    
    // Sequencer
    sequence_counter: u8,
    
    // Length counter
    length_counter: u8,
}

impl TriangleChannel {
    pub fn new() -> Self {
        TriangleChannel {
            enabled: false,
            linear_counter: 0,
            linear_counter_reload: 0,
            linear_counter_reload_flag: false,
            control_flag: false,
            timer_period: 0,
            timer_counter: 0,
            sequence_counter: 0,
            length_counter: 0,
        }
    }
    
    pub fn clock_timer(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period;
            if self.length_counter > 0 && self.linear_counter > 0 {
                self.sequence_counter = (self.sequence_counter + 1) % 32;
            }
        } else {
            self.timer_counter -= 1;
        }
    }
    
    pub fn clock_linear_counter(&mut self) {
        if self.linear_counter_reload_flag {
            self.linear_counter = self.linear_counter_reload;
        } else if self.linear_counter > 0 {
            self.linear_counter -= 1;
        }
        
        if !self.control_flag {
            self.linear_counter_reload_flag = false;
        }
    }
    
    pub fn clock_length(&mut self) {
        if !self.control_flag && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    pub fn output(&self) -> u8 {
        const TRIANGLE_SEQUENCE: [u8; 32] = [
            15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0,
            0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
        ];
        
        if !self.enabled || self.length_counter == 0 || self.linear_counter == 0 {
            return 0;
        }
        
        // Silence ultrasonic frequencies
        if self.timer_period < 2 {
            return 7; // Middle value to reduce pops
        }
        
        TRIANGLE_SEQUENCE[self.sequence_counter as usize]
    }
}

// Noise channel
#[derive(Debug, Clone)]
struct NoiseChannel {
    enabled: bool,
    
    // Envelope
    envelope_start: bool,
    envelope_divider: u8,
    envelope_decay: u8,
    envelope_loop: bool,
    constant_volume: bool,
    volume: u8,
    
    // Timer
    timer_period: u16,
    timer_counter: u16,
    
    // LFSR (Linear Feedback Shift Register)
    shift_register: u16,
    mode: bool, // false = 15-bit, true = 6-bit
    
    // Length counter
    length_counter: u8,
    length_halt: bool,
}

impl NoiseChannel {
    pub fn new() -> Self {
        NoiseChannel {
            enabled: false,
            envelope_start: false,
            envelope_divider: 0,
            envelope_decay: 0,
            envelope_loop: false,
            constant_volume: false,
            volume: 0,
            timer_period: 0,
            timer_counter: 0,
            shift_register: 1,
            mode: false,
            length_counter: 0,
            length_halt: false,
        }
    }
    
    pub fn clock_timer(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period;
            
            let feedback = if self.mode {
                ((self.shift_register & 1) ^ ((self.shift_register >> 6) & 1)) & 1
            } else {
                ((self.shift_register & 1) ^ ((self.shift_register >> 1) & 1)) & 1
            };
            
            self.shift_register >>= 1;
            self.shift_register |= feedback << 14;
        } else {
            self.timer_counter -= 1;
        }
    }
    
    pub fn clock_envelope(&mut self) {
        if self.envelope_start {
            self.envelope_decay = 15;
            self.envelope_divider = self.volume;
            self.envelope_start = false;
        } else if self.envelope_divider == 0 {
            self.envelope_divider = self.volume;
            if self.envelope_decay > 0 {
                self.envelope_decay -= 1;
            } else if self.envelope_loop {
                self.envelope_decay = 15;
            }
        } else {
            self.envelope_divider -= 1;
        }
    }
    
    pub fn clock_length(&mut self) {
        if !self.length_halt && self.length_counter > 0 {
            self.length_counter -= 1;
        }
    }
    
    pub fn output(&self) -> u8 {
        if !self.enabled || self.length_counter == 0 || (self.shift_register & 1) == 1 {
            return 0;
        }
        
        if self.constant_volume {
            self.volume
        } else {
            self.envelope_decay
        }
    }
}

// DMC (Delta Modulation Channel)
#[derive(Debug, Clone)]
struct DmcChannel {
    enabled: bool,
    
    irq_enabled: bool,
    irq_flag: bool,
    loop_flag: bool,
    
    // Sample
    sample_address: u16,
    sample_length: u16,
    current_address: u16,
    bytes_remaining: u16,
    
    sample_buffer: Option<u8>,
    shift_register: u8,
    bits_remaining: u8,
    
    // Timer
    timer_period: u16,
    timer_counter: u16,
    
    // Output
    output_level: u8,
    silence: bool,
    dma_request: bool,
}

impl DmcChannel {
    pub fn new() -> Self {
        DmcChannel {
            enabled: false,
            irq_enabled: false,
            irq_flag: false,
            loop_flag: false,
            sample_address: 0xC000,
            sample_length: 0,
            current_address: 0xC000,
            bytes_remaining: 0,
            sample_buffer: None,
            shift_register: 0,
            bits_remaining: 0,
            timer_period: 0,
            timer_counter: 0,
            output_level: 0,
            silence: true,
            dma_request: false,
        }
    }
    
    pub fn clock_timer(&mut self) {
        if self.timer_counter == 0 {
            self.timer_counter = self.timer_period;
            
            if !self.silence {
                if (self.shift_register & 1) == 1 {
                    if self.output_level <= 125 {
                        self.output_level += 2;
                    }
                } else {
                    if self.output_level >= 2 {
                        self.output_level -= 2;
                    }
                }
                
                self.shift_register >>= 1;
            }
            
            if self.bits_remaining > 0 {
                self.bits_remaining -= 1;
            }
            
            if self.bits_remaining == 0 {
                if let Some(sample) = self.sample_buffer.take() {
                    self.silence = false;
                    self.shift_register = sample;
                    self.bits_remaining = 8;
                    self.dma_request = self.bytes_remaining > 0 || self.loop_flag;
                } else {
                    self.silence = true;
                    self.bits_remaining = 0;
                    self.dma_request = self.bytes_remaining > 0 || self.loop_flag;
                }
            }
        } else {
            self.timer_counter -= 1;
        }
    }
    
    pub fn start_sample(&mut self) {
        self.current_address = self.sample_address;
        self.bytes_remaining = self.sample_length;

        if self.sample_length == 0 {
            return;
        }

        self.dma_request = true;
    }
    
    pub fn output(&self) -> u8 {
        if !self.enabled {
            return 0;
        }
        self.output_level
    }
}

bitflags! {
    // $4015 write - APU Status
    // ---D NT21
    // |||| ||||
    // |||| |||+- Pulse 1 enable
    // |||| ||+-- Pulse 2 enable
    // |||| |+--- Triangle enable
    // |||| +---- Noise enable
    // |||+------ DMC enable
    struct StatusFlags: u8 {
        const PULSE1_ENABLE = (1 << 0);
        const PULSE2_ENABLE = (1 << 1);
        const TRIANGLE_ENABLE = (1 << 2);
        const NOISE_ENABLE = (1 << 3);
        const DMC_ENABLE = (1 << 4);
    }
}

impl StatusFlags {
    pub fn new() -> Self {
        StatusFlags::empty()
    }
}

#[derive(Debug, Clone, Copy)]
pub enum FrameCounterMode {
    FourStep,  // 4-step sequence
    FiveStep,  // 5-step sequence
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameCounterProfile {
    Ntsc,
    Pal,
}

pub struct Apu {
    _rom: RomRef,

    pulse1: PulseChannel,
    pulse2: PulseChannel,
    triangle: TriangleChannel,
    noise: NoiseChannel,
    dmc: DmcChannel,
    
    status: StatusFlags,
    
    // Frame counter
    frame_counter_mode: FrameCounterMode,
    frame_counter: usize,
    frame_counter_write_delay: usize,
    pending_frame_counter_write: Option<u8>,
    irq_inhibit: bool,
    frame_irq: bool,
    frame_irq_assert_cycle: usize,
    last_frame_counter_write: u8,
    frame_profile: FrameCounterProfile,
    
    pub cycles: usize,
}

impl Apu {
    pub fn new(rom: RomRef) -> std::rc::Rc<std::cell::RefCell<Self>> {
        let frame_profile = if matches!(rom.info.tv_format, TVFormat::PAL) {
            FrameCounterProfile::Pal
        } else {
            FrameCounterProfile::Ntsc
        };
        std::rc::Rc::new(std::cell::RefCell::new(Apu {
            _rom: rom,
            pulse1: PulseChannel::new(),
            pulse2: PulseChannel::new(),
            triangle: TriangleChannel::new(),
            noise: NoiseChannel::new(),
            dmc: DmcChannel::new(),
            status: StatusFlags::new(),
            frame_counter_mode: FrameCounterMode::FourStep,
            frame_counter: 0,
            frame_counter_write_delay: 0,
            pending_frame_counter_write: None,
            irq_inhibit: false,
            frame_irq: false,
            frame_irq_assert_cycle: 0,
            last_frame_counter_write: 0,
            frame_profile,
            cycles: 0,
        }))
    }
    
    pub fn reset(&mut self) {
        self.power_reset();
    }

    pub fn power_reset(&mut self) {
        self.pulse1 = PulseChannel::new();
        self.pulse2 = PulseChannel::new();
        self.triangle = TriangleChannel::new();
        self.noise = NoiseChannel::new();
        self.dmc = DmcChannel::new();
        self.status = StatusFlags::new();
        self.frame_counter_mode = FrameCounterMode::FourStep;
        self.frame_counter = 10;
        self.frame_counter_write_delay = 0;
        self.pending_frame_counter_write = None;
        self.irq_inhibit = false;
        self.frame_irq = false;
        self.frame_irq_assert_cycle = 0;
        self.last_frame_counter_write = 0;
        // At power-on the APU behaves as if $00 was written to $4017
        // a small number of clocks before reset vector execution.
        self.cycles = 10;
    }

    pub fn console_reset(&mut self) {
        self.pulse1.enabled = false;
        self.pulse2.enabled = false;
        self.triangle.enabled = false;
        self.noise.enabled = false;
        self.dmc.enabled = false;
        self.dmc.irq_flag = false;

        self.pulse1.length_counter = 0;
        self.pulse2.length_counter = 0;
        self.triangle.length_counter = 0;
        self.noise.length_counter = 0;
        self.dmc.bytes_remaining = 0;

        self.status = StatusFlags::new();
        self.frame_counter = 10;
        self.frame_counter_write_delay = 0;
        self.pending_frame_counter_write = None;
        self.frame_irq = false;
        self.frame_irq_assert_cycle = 0;
        
        // On console reset the frame counter mode survives, but the IRQ
        // inhibit bit does not reliably behave like a full $4017 replay.
        let effective_frame_counter = self.last_frame_counter_write & 0x80;
        self.frame_counter_mode = if (effective_frame_counter & 0x80) != 0 {
            FrameCounterMode::FiveStep
        } else {
            FrameCounterMode::FourStep
        };
        self.irq_inhibit = false;

        if matches!(self.frame_counter_mode, FrameCounterMode::FiveStep) {
            self.clock_quarter_frame();
            self.clock_half_frame();
        }

        // Reset appears shortly after the effective $4017 rewrite.
        self.cycles = 10;
    }
    
    pub fn step_cycle(&mut self) {
        let is_even_cycle = self.cycles % 2 == 0;
        
        // Triangle and DMC clock every CPU cycle
        self.triangle.clock_timer();
        self.dmc.clock_timer();
        self.service_dmc_dma();
        
        // Pulse and noise clock every other CPU cycle
        if is_even_cycle {
            self.pulse1.clock_timer();
            self.pulse2.clock_timer();
            self.noise.clock_timer();
        }
        
        // Frame counter
        self.clock_frame_counter();
        self.apply_pending_length_writes();
        
        self.cycles += 1;
    }

    fn service_dmc_dma(&mut self) {
        if !self.dmc.enabled || !self.dmc.dma_request || self.dmc.sample_buffer.is_some() {
            return;
        }
        if self.dmc.bytes_remaining == 0 {
            if self.dmc.loop_flag && self.dmc.sample_length > 0 {
                self.dmc.current_address = self.dmc.sample_address;
                self.dmc.bytes_remaining = self.dmc.sample_length;
            } else {
                self.dmc.dma_request = false;
                if self.dmc.irq_enabled {
                    self.dmc.irq_flag = true;
                }
                return;
            }
        }

        let sample = self._rom.mapper.borrow_mut().read_prg(self.dmc.current_address);
        self.dmc.sample_buffer = Some(sample);
        self.dmc.dma_request = false;

        self.dmc.bytes_remaining -= 1;
        self.dmc.current_address = self.dmc.current_address.wrapping_add(1);
        if self.dmc.current_address == 0 {
            self.dmc.current_address = 0x8000;
        }
        if self.dmc.bytes_remaining == 0 && !self.dmc.loop_flag && self.dmc.irq_enabled {
            self.dmc.irq_flag = true;
        }
    }

    fn apply_pending_length_writes(&mut self) {
        if let Some(length_halt) = self.pulse1.pending_length_halt.take() {
            self.pulse1.length_halt = length_halt;
            self.pulse1.envelope_loop = length_halt;
        }

        if let Some(length_counter) = self.pulse1.pending_length_reload.take() {
            self.pulse1.length_counter = length_counter;
        }

        if let Some(length_halt) = self.pulse2.pending_length_halt.take() {
            self.pulse2.length_halt = length_halt;
            self.pulse2.envelope_loop = length_halt;
        }

        if let Some(length_counter) = self.pulse2.pending_length_reload.take() {
            self.pulse2.length_counter = length_counter;
        }
    }

    fn half_frame_clocks_length_this_cycle(&self) -> bool {
        if self.frame_counter_write_delay > 0 {
            return false;
        }

        match (self.frame_profile, self.frame_counter_mode) {
            (FrameCounterProfile::Ntsc, FrameCounterMode::FourStep) => {
                matches!(self.frame_counter, 14913 | 29829)
            }
            (FrameCounterProfile::Ntsc, FrameCounterMode::FiveStep) => {
                matches!(self.frame_counter, 14913 | 37281)
            }
            (FrameCounterProfile::Pal, FrameCounterMode::FourStep) => {
                matches!(self.frame_counter, 16627 | 33253)
            }
            (FrameCounterProfile::Pal, FrameCounterMode::FiveStep) => {
                matches!(self.frame_counter, 16627 | 41565)
            }
        }
    }

    fn clock_frame_counter(&mut self) {
        if self.frame_counter_write_delay > 0 {
            self.frame_counter_write_delay -= 1;
            if self.frame_counter_write_delay == 0 {
                if let Some(value) = self.pending_frame_counter_write.take() {
                    self.apply_frame_counter_write(value);
                }
            }
            return;
        }

        // Frame counter logic based on profile + mode.
        match (self.frame_profile, self.frame_counter_mode) {
            (FrameCounterProfile::Ntsc, FrameCounterMode::FourStep) => match self.frame_counter {
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                22371 => self.clock_quarter_frame(),
                29828 => {
                    if !self.irq_inhibit {
                        self.set_frame_irq();
                    }
                }
                29829 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    if !self.irq_inhibit {
                        self.set_frame_irq();
                    }
                }
                29830 => {
                    if !self.irq_inhibit {
                        self.set_frame_irq();
                    }
                    self.frame_counter = 1;
                    return;
                }
                _ => {}
            },
            (FrameCounterProfile::Ntsc, FrameCounterMode::FiveStep) => match self.frame_counter {
                7457 => self.clock_quarter_frame(),
                14913 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                22371 => self.clock_quarter_frame(),
                37281 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    self.frame_counter = 0;
                    return;
                }
                _ => {}
            },
            (FrameCounterProfile::Pal, FrameCounterMode::FourStep) => match self.frame_counter {
                8313 => self.clock_quarter_frame(),
                16627 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                24939 => self.clock_quarter_frame(),
                33252 => {
                    if !self.irq_inhibit {
                        self.set_frame_irq();
                    }
                }
                33253 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    if !self.irq_inhibit {
                        self.set_frame_irq();
                    }
                }
                33254 => {
                    if !self.irq_inhibit {
                        self.set_frame_irq();
                    }
                    self.frame_counter = 1;
                    return;
                }
                _ => {}
            },
            (FrameCounterProfile::Pal, FrameCounterMode::FiveStep) => match self.frame_counter {
                8313 => self.clock_quarter_frame(),
                16627 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                }
                24939 => self.clock_quarter_frame(),
                33253 => {}
                41565 => {
                    self.clock_quarter_frame();
                    self.clock_half_frame();
                    self.frame_counter = 0;
                    return;
                }
                _ => {}
            },
        }

        self.frame_counter += 1;
    }
    
    fn clock_quarter_frame(&mut self) {
        // Clock envelopes and triangle's linear counter
        self.pulse1.clock_envelope();
        self.pulse2.clock_envelope();
        self.triangle.clock_linear_counter();
        self.noise.clock_envelope();
    }
    
    fn clock_half_frame(&mut self) {
        // Clock length counters and sweep units
        self.pulse1.clock_length();
        self.pulse1.clock_sweep(true);
        
        self.pulse2.clock_length();
        self.pulse2.clock_sweep(false);
        
        self.triangle.clock_length();
        self.noise.clock_length();
    }
    
    // $4000 - Pulse 1 control
    pub fn write_pulse1_ctrl(&mut self, value: u8) {
        self.pulse1.duty_cycle = (value >> 6) & 0x03;
        let length_halt = (value & 0x20) != 0;
        if self.half_frame_clocks_length_this_cycle() {
            self.pulse1.pending_length_halt = Some(length_halt);
        } else {
            self.pulse1.length_halt = length_halt;
            self.pulse1.envelope_loop = length_halt;
        }
        self.pulse1.constant_volume = (value & 0x10) != 0;
        self.pulse1.volume = value & 0x0F;
    }
    
    // $4001 - Pulse 1 sweep
    pub fn write_pulse1_sweep(&mut self, value: u8) {
        self.pulse1.sweep_enabled = (value & 0x80) != 0;
        self.pulse1.sweep_period = (value >> 4) & 0x07;
        self.pulse1.sweep_negate = (value & 0x08) != 0;
        self.pulse1.sweep_shift = value & 0x07;
        self.pulse1.sweep_reload = true;
    }
    
    // $4002 - Pulse 1 timer low
    pub fn write_pulse1_timer_low(&mut self, value: u8) {
        self.pulse1.timer_period = (self.pulse1.timer_period & 0xFF00) | (value as u16);
    }
    
    // $4003 - Pulse 1 length counter / timer high
    pub fn write_pulse1_length(&mut self, value: u8) {
        const LENGTH_TABLE: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
            12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];
        let length_counter = LENGTH_TABLE[(value >> 3) as usize];
        
        self.pulse1.timer_period = (self.pulse1.timer_period & 0x00FF) | (((value & 0x07) as u16) << 8);
        
        if self.pulse1.enabled {
            if self.half_frame_clocks_length_this_cycle() {
                if self.pulse1.length_counter == 0 {
                    self.pulse1.pending_length_reload = Some(length_counter);
                }
            } else {
                self.pulse1.length_counter = length_counter;
            }
        }
        
        self.pulse1.sequence_counter = 0;
        self.pulse1.envelope_start = true;
    }
    
    // $4004-$4007 - Pulse 2 (аналогично Pulse 1)
    pub fn write_pulse2_ctrl(&mut self, value: u8) {
        self.pulse2.duty_cycle = (value >> 6) & 0x03;
        let length_halt = (value & 0x20) != 0;
        if self.half_frame_clocks_length_this_cycle() {
            self.pulse2.pending_length_halt = Some(length_halt);
        } else {
            self.pulse2.length_halt = length_halt;
            self.pulse2.envelope_loop = length_halt;
        }
        self.pulse2.constant_volume = (value & 0x10) != 0;
        self.pulse2.volume = value & 0x0F;
    }
    
    pub fn write_pulse2_sweep(&mut self, value: u8) {
        self.pulse2.sweep_enabled = (value & 0x80) != 0;
        self.pulse2.sweep_period = (value >> 4) & 0x07;
        self.pulse2.sweep_negate = (value & 0x08) != 0;
        self.pulse2.sweep_shift = value & 0x07;
        self.pulse2.sweep_reload = true;
    }
    
    pub fn write_pulse2_timer_low(&mut self, value: u8) {
        self.pulse2.timer_period = (self.pulse2.timer_period & 0xFF00) | (value as u16);
    }
    
    pub fn write_pulse2_length(&mut self, value: u8) {
        const LENGTH_TABLE: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
            12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];
        
        self.pulse2.timer_period = (self.pulse2.timer_period & 0x00FF) | (((value & 0x07) as u16) << 8);
        
        let length_counter = LENGTH_TABLE[(value >> 3) as usize];
        if self.pulse2.enabled {
            if self.half_frame_clocks_length_this_cycle() {
                if self.pulse2.length_counter == 0 {
                    self.pulse2.pending_length_reload = Some(length_counter);
                }
            } else {
                self.pulse2.length_counter = length_counter;
            }
        }
        
        self.pulse2.sequence_counter = 0;
        self.pulse2.envelope_start = true;
    }
    
    // $4008 - Triangle control
    pub fn write_triangle_ctrl(&mut self, value: u8) {
        self.triangle.control_flag = (value & 0x80) != 0;
        self.triangle.linear_counter_reload = value & 0x7F;
    }
    
    // $400A - Triangle timer low
    pub fn write_triangle_timer_low(&mut self, value: u8) {
        self.triangle.timer_period = (self.triangle.timer_period & 0xFF00) | (value as u16);
    }
    
    // $400B - Triangle length counter / timer high
    pub fn write_triangle_length(&mut self, value: u8) {
        const LENGTH_TABLE: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
            12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];
        
        self.triangle.timer_period = (self.triangle.timer_period & 0x00FF) | (((value & 0x07) as u16) << 8);
        
        if self.triangle.enabled {
            self.triangle.length_counter = LENGTH_TABLE[(value >> 3) as usize];
        }
        
        self.triangle.linear_counter_reload_flag = true;
    }
    
    // $400C - Noise control
    pub fn write_noise_ctrl(&mut self, value: u8) {
        self.noise.length_halt = (value & 0x20) != 0;
        self.noise.envelope_loop = (value & 0x20) != 0;
        self.noise.constant_volume = (value & 0x10) != 0;
        self.noise.volume = value & 0x0F;
    }
    
    // $400E - Noise period
    pub fn write_noise_period(&mut self, value: u8) {
        const NOISE_PERIOD_TABLE: [u16; 16] = [
            4, 8, 16, 32, 64, 96, 128, 160, 202, 254, 380, 508, 762, 1016, 2034, 4068,
        ];
        
        self.noise.mode = (value & 0x80) != 0;
        self.noise.timer_period = NOISE_PERIOD_TABLE[(value & 0x0F) as usize];
    }
    
    // $400F - Noise length counter
    pub fn write_noise_length(&mut self, value: u8) {
        const LENGTH_TABLE: [u8; 32] = [
            10, 254, 20, 2, 40, 4, 80, 6, 160, 8, 60, 10, 14, 12, 26, 14,
            12, 16, 24, 18, 48, 20, 96, 22, 192, 24, 72, 26, 16, 28, 32, 30,
        ];
        
        if self.noise.enabled {
            self.noise.length_counter = LENGTH_TABLE[(value >> 3) as usize];
        }
        
        self.noise.envelope_start = true;
    }
    
    // $4010 - DMC control
    pub fn write_dmc_ctrl(&mut self, value: u8) {
        const DMC_RATE_TABLE: [u16; 16] = [
            428, 380, 340, 320, 286, 254, 226, 214, 190, 160, 142, 128, 106, 84, 72, 54,
        ];
        
        self.dmc.irq_enabled = (value & 0x80) != 0;
        if !self.dmc.irq_enabled {
            self.dmc.irq_flag = false;
        }
        self.dmc.loop_flag = (value & 0x40) != 0;
        self.dmc.timer_period = DMC_RATE_TABLE[(value & 0x0F) as usize];
    }
    
    // $4011 - DMC output
    pub fn write_dmc_output(&mut self, value: u8) {
        self.dmc.output_level = value & 0x7F;
    }
    
    // $4012 - DMC sample address
    pub fn write_dmc_address(&mut self, value: u8) {
        self.dmc.sample_address = 0xC000 + ((value as u16) * 64);
    }
    
    // $4013 - DMC sample length
    pub fn write_dmc_length(&mut self, value: u8) {
        self.dmc.sample_length = ((value as u16) * 16) + 1;
    }
    
    // $4015 write - Status
    pub fn write_status(&mut self, value: u8) {
        self.status = StatusFlags::from_bits_truncate(value);
        self.dmc.irq_flag = false;
        
        self.pulse1.enabled = self.status.contains(StatusFlags::PULSE1_ENABLE);
        self.pulse2.enabled = self.status.contains(StatusFlags::PULSE2_ENABLE);
        self.triangle.enabled = self.status.contains(StatusFlags::TRIANGLE_ENABLE);
        self.noise.enabled = self.status.contains(StatusFlags::NOISE_ENABLE);
        self.dmc.enabled = self.status.contains(StatusFlags::DMC_ENABLE);
        
        if !self.pulse1.enabled {
            self.pulse1.length_counter = 0;
        }
        if !self.pulse2.enabled {
            self.pulse2.length_counter = 0;
        }
        if !self.triangle.enabled {
            self.triangle.length_counter = 0;
        }
        if !self.noise.enabled {
            self.noise.length_counter = 0;
        }
        
        if !self.dmc.enabled {
            self.dmc.bytes_remaining = 0;
            self.dmc.irq_flag = false;
            self.dmc.dma_request = false;
        } else if self.dmc.bytes_remaining == 0 {
            self.dmc.start_sample();
        }
    }
    
    // $4015 read - Status
    pub fn read_status(&mut self) -> u8 {
        let mut status = 0u8;
        
        if self.pulse1.length_counter > 0 {
            status |= 0x01;
        }
        if self.pulse2.length_counter > 0 {
            status |= 0x02;
        }
        if self.triangle.length_counter > 0 {
            status |= 0x04;
        }
        if self.noise.length_counter > 0 {
            status |= 0x08;
        }
        if self.dmc.bytes_remaining > 0 {
            status |= 0x10;
        }
        if self.frame_irq {
            status |= 0x40;
        }
        if self.dmc.irq_flag {
            status |= 0x80;
        }
        
        self.frame_irq = false;
        self.frame_irq_assert_cycle = 0;
        
        status
    }
    
    // $4017 - Frame counter
    pub fn write_frame_counter(&mut self, value: u8) {
        self.last_frame_counter_write = value;
        self.pending_frame_counter_write = Some(value);

        if (value & 0x40) != 0 {
            self.frame_irq = false;
            self.frame_irq_assert_cycle = 0;
        }

        // The frame counter write takes effect a few CPU cycles later.
        self.frame_counter_write_delay = match self.frame_profile {
            FrameCounterProfile::Ntsc => {
                if (self.cycles & 1) == 0 { 2 } else { 3 }
            }
            FrameCounterProfile::Pal => {
                if (self.cycles & 1) == 0 { 2 } else { 3 }
            }
        };
    }

    fn apply_frame_counter_write(&mut self, value: u8) {
        self.frame_counter_mode = if (value & 0x80) != 0 {
            FrameCounterMode::FiveStep
        } else {
            FrameCounterMode::FourStep
        };
        self.irq_inhibit = (value & 0x40) != 0;

        if self.irq_inhibit {
            self.frame_irq = false;
            self.frame_irq_assert_cycle = 0;
        }

        self.frame_counter = 0;

        if matches!(self.frame_counter_mode, FrameCounterMode::FiveStep) {
            self.clock_quarter_frame();
            self.clock_half_frame();
        }
    }
    
    // Mix all channels for audio output
    pub fn output(&self) -> f32 {
        let pulse_out = {
            let pulse1 = self.pulse1.output() as f32;
            let pulse2 = self.pulse2.output() as f32;
            
            if pulse1 + pulse2 == 0.0 {
                0.0
            } else {
                95.88 / ((8128.0 / (pulse1 + pulse2)) + 100.0)
            }
        };
        
        let tnd_out = {
            let triangle = self.triangle.output() as f32;
            let noise = self.noise.output() as f32;
            let dmc = self.dmc.output() as f32;
            
            if triangle + noise + dmc == 0.0 {
                0.0
            } else {
                159.79 / ((1.0 / ((triangle / 8227.0) + (noise / 12241.0) + (dmc / 22638.0))) + 100.0)
            }
        };
        
        pulse_out + tnd_out
    }
    
    pub fn get_irq(&self) -> bool {
        (self.frame_irq && self.cycles >= self.frame_irq_assert_cycle) || self.dmc.irq_flag
    }

    pub fn set_pal_frame_counter_profile(&mut self, enable: bool) {
        self.frame_profile = if enable {
            FrameCounterProfile::Pal
        } else {
            FrameCounterProfile::Ntsc
        };
    }

    fn set_frame_irq(&mut self) {
        self.frame_irq = true;
        // Keep $4015 flag semantics unchanged, but model CPU-visible IRQ line
        // with profile-specific assertion latency.
        self.frame_irq_assert_cycle = self.cycles
            + match self.frame_profile {
                FrameCounterProfile::Ntsc => 2,
                FrameCounterProfile::Pal => 3,
            };
    }
}
