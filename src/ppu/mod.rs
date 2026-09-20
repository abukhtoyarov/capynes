mod loopy_reg;
use loopy_reg::Address;

mod flags;
use flags::{ CtrlFlags, StatusFlags, MaskFlags };

mod frame;
pub use frame::Frame;

mod bg;
use bg::Background;

mod sprite;
use sprite::Sprites;

use crate::rom::RomRef;
use crate::rom::info::Mirroring;

/*
 *              ┌──────────────────────────────────┐
 *              │            PPU (2C02)            │
 *              │                                  │
 *    CPU → MMIO│ ┌────────────┐  ┌──────────────┐ │
 *    Registers │ │ Control &  │  │ State + Sync │ │
 *    ($2000–07)│ │ Configuration │              │ │
 *              │ └─────┬──────┘  └────┬─────────┘ │
 *              │       │              │           │
 *              │       ▼              ▼           │
 *              │ ┌───────────┐    ┌─────────────┐ │
 *              │ │ Memory    │    │ Rendering   │ │
 *              │ │ Fetch &   │    │ Pipeline    │ │
 *              │ │ Address   │    │             │ │
 *              │ │ Logic     │    │ Background  │ │
 *              │ └───┬───────┘    │ & Sprites   │ │
 *              │     │            └────┬────────┘ │
 *              │     ▼                 ▼          │
 *              │  VRAM/CHR ROM → Pixel Combiner   │
 *              │             Palette RAM          │
 *              │                  ↓               │
 *              │           Video Output (NTSC)    │
 *              └──────────────────────────────────┘
 */

pub struct Oam {
    pub addr: u8,
    pub data: [u8; 0x100]
}

pub struct Memory {
    vmem: [u8; 0x800],
    rom: RomRef,
    palette: [u8; 0x20],
    ppu_cycle: u64,
}

const POWER_UP_PALETTE: [u8; 0x20] = [
    0x09, 0x01, 0x00, 0x01, 0x00, 0x02, 0x02, 0x0D,
    0x08, 0x10, 0x08, 0x24, 0x00, 0x00, 0x04, 0x2C,
    0x09, 0x01, 0x34, 0x03, 0x00, 0x04, 0x00, 0x14,
    0x08, 0x3A, 0x00, 0x02, 0x00, 0x20, 0x2C, 0x08,
];

pub struct Ppu {
    mem: Memory,
    addr: Address,
    ctrl: CtrlFlags,
    mask: MaskFlags,
    status: StatusFlags,
    oam: Oam,
    read_buffer: u8,
    io_bus: u8,

    pub cycles: usize,
    pub scanlines: u16,
    frames: usize,

    nmi: bool,
    nmi_line: bool,
    nmi_delay: Option<u8>,
    suppress_vblank_flag: bool,

    bg: Background,
    sprites: Sprites,

    frame: Frame,
}

impl Ppu {
    pub fn new(rom: RomRef) -> std::rc::Rc<std::cell::RefCell<Self>> {
        std::rc::Rc::new(std::cell::RefCell::new(Ppu {
            mem: Memory::new(rom),
            ctrl: CtrlFlags::default(),
            mask: MaskFlags::default(),
            addr: Address::default(),
            status: StatusFlags::default(),
            oam: Oam::default(),
            read_buffer: 0,
            io_bus: 0,
            cycles: 0,
            scanlines: 0,
            frames: 0,

            nmi: false,
            nmi_line: false,
            nmi_delay: None,
            suppress_vblank_flag: false,
            bg: Background::default(),
            sprites: Sprites::default(),
            frame: Frame::default(),
        }))
    }

    pub fn reset(&mut self) {
        self.mem.reset();
        self.addr.reset();
        self.ctrl = CtrlFlags::default();
        self.mask = MaskFlags::default();
        self.status = StatusFlags::default();
        self.oam = Oam::default();
        self.read_buffer = 0;
        self.io_bus = 0;
        self.cycles = 0;
        self.scanlines = 0;
        self.frames = 0;
        self.nmi = false;
        self.nmi_line = false;
        self.nmi_delay = None;
        self.suppress_vblank_flag = false;
        self.bg.reset();
        self.sprites.reset();
    }

    pub fn step_cycles(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.step();
        }
    }

    pub fn poll_nmi(&mut self) -> bool {
        if self.nmi {
            self.nmi = false;
            return true;
        }
        false
    }

    #[inline]
    pub fn frame(&self) -> &Frame {
        &self.frame
    }

    #[inline]
    pub fn io_bus(&self) -> u8 {
        self.io_bus
    }

    #[inline]
    pub fn absolute_cycle(&self) -> u64 {
        self.frames as u64 * 262 * 341 + self.mem.ppu_cycle
    }

    #[inline]
    pub fn data_read_refresh_mask(&self) -> u8 {
        let addr = self.addr.v & 0x3FFF;
        if (0x3F00..=0x3FFF).contains(&addr) {
            0x3F
        } else {
            0xFF
        }
    }

    // PPUCTRL - Miscellaneous settings ($2000 write)
    pub fn ctrl(&mut self, value: u8) {
        let preserve_pending_nmi = self.status.contains(StatusFlags::VBLANK)
            && self.ctrl.contains(CtrlFlags::VBLANK_NMI_ENABLE)
            && (value & CtrlFlags::VBLANK_NMI_ENABLE.bits()) == 0
            && self.scanlines == 241
            && self.cycles >= 3;
        let pending_nmi = self.nmi_delay;
        self.io_bus = value;
        self.ctrl = CtrlFlags::from_bits_truncate(value);
        self.addr.write_ppuctrl(value);
        self.update_nmi_state();
        if preserve_pending_nmi {
            self.nmi_delay = pending_nmi;
        }
    }

    // PPUMASK - Rendering settings ($2001 write)
    pub fn write_mask(&mut self, value: u8) {
        self.io_bus = value;
        self.mask = MaskFlags::from_bits_truncate(value);
    }

    // PPUSTATUS - Rendering events ($2002 read)
    pub fn read_status(&mut self) -> u8 {
        self.read_status_with_open_bus(self.io_bus)
    }

    pub fn read_status_with_open_bus(&mut self, open_bus: u8) -> u8 {
        let data = (self.status.bits() & 0xe0) | (open_bus & 0x1f);
        let preserve_pending_nmi = self.scanlines == 241 && self.cycles >= 3;
        let pending_nmi = self.nmi_delay;
        if self.scanlines == 241 && self.cycles == 0 {
            self.suppress_vblank_flag = true;
            self.nmi = false;
            self.nmi_delay = None;
        }
        self.status.remove(StatusFlags::VBLANK);
        self.update_nmi_state();
        if preserve_pending_nmi {
            self.nmi_delay = pending_nmi;
        }
        self.addr.reset_latch();
        self.io_bus = data;
        data
    }

    // OAMADDR - Sprite RAM address ($2003 write)
    pub fn write_oam_addr(&mut self, value: u8) {
        self.io_bus = value;
        self.oam.addr = value;
    }

    // OAMDATA - Sprite RAM data ($2004 read/write)
    pub fn read_oam_data(&mut self) -> u8 {
        let value = self.oam.data[self.oam.addr as usize];
        let data = if self.oam.addr & 0x03 == 0x02 {
            value & 0xE3
        } else {
            value
        };
        self.io_bus = data;
        data
    }

    // OAMDMA - Sprite DMA ($4014 write)
    pub fn write_oam_dma_bytes(&mut self, bytes: &[u8; 256]) {
        let start = self.oam.addr;

        for i in 0..256u16 {
            let byte = bytes[i as usize];
            let addr = start.wrapping_add(i as u8);
            self.oam.data[addr as usize] = byte;
        }
    }

    pub fn write_oam_data(&mut self, value: u8) {
        self.io_bus = value;
        self.oam.data[self.oam.addr as usize] = value;
        self.oam.addr = self.oam.addr.wrapping_add(1);
    }

    // PPUSCROLL - X and Y scroll ($2005 write)
    pub fn write_scroll(&mut self, value: u8) {
        self.io_bus = value;
        self.addr.write_ppuscroll(value);
    }

    // PPUADDR - VRAM address ($2006 write)
    pub fn write_addr(&mut self, value: u8) {
        self.io_bus = value;
        self.addr.write_ppuaddr(value);

        // MMC3 can be clocked by toggling A12 via $2006; report both writes.
        if !self.rendering_enabled() {
            let observed_addr = if self.addr.w { self.addr.t } else { self.addr.v } & 0x3FFF;
            self.mem
                .rom
                .mapper
                .borrow_mut()
                .on_cpu_ppu_addr(observed_addr, self.absolute_cycle());
        }
    }

    // PPUDATA read ($2007)
    pub fn read(&mut self) -> u8 {
        let addr = self.addr.v & 0x3FFF; // 14-bit VRAM address

        let value = if (0x3F00..=0x3FFF).contains(&addr) {
            let palette_value = self.mem.read_palette(addr);
            self.read_buffer = self.mem.read(addr & 0x2FFF);
            palette_value
        } else {
            let ret = self.read_buffer;
            self.read_buffer = self.mem.read(addr);
            ret
        };

        self.addr.v = self.addr.v.wrapping_add(self.ctrl.increment());
        if !self.rendering_enabled() {
            self.mem
                .rom
                .mapper
                .borrow_mut()
                .on_cpu_ppu_addr(self.addr.v & 0x3FFF, self.absolute_cycle());
        }
        self.io_bus = value;
        value
    }

    // PPUDATA write ($2007)
    pub fn write(&mut self, value: u8) {
        self.io_bus = value;
        self.mem.write(self.addr.v & 0x3FFF, value);
        self.addr.v = self.addr.v.wrapping_add(self.ctrl.increment());
        if !self.rendering_enabled() {
            self.mem
                .rom
                .mapper
                .borrow_mut()
                .on_cpu_ppu_addr(self.addr.v & 0x3FFF, self.absolute_cycle());
        }
    }

    fn update_background(&mut self) {
        if !self.rendering_enabled() {
            return;
        }

        self.bg.update(&self.cycles, &self.scanlines, &self.ctrl,
            &self.mask, &mut self.addr, &mut self.mem);
    }

    fn update_sprites(&mut self) {
        if !self.rendering_enabled() {
            return;
        }
        if self.scanlines >= 240 && !(self.scanlines == 261 && self.cycles == 257) {
            return;
        }
        self.sprites.update(&self.cycles, &self.scanlines,
            &self.ctrl, &mut self.status, &self.oam, &mut self.mem);
    }

    fn render_pixel(&mut self) {
        if self.scanlines >= 240 || self.cycles < 1 || self.cycles > 256 {
            return;
        }

        let x = (self.cycles - 1) as usize;
        let y = self.scanlines as usize;

        let (bg_pixel, bg_palette) = self.bg.get_pixel(&self.mask, &self.cycles, &mut self.addr);
        let (sprite_pixel, sprite_palette, sprite_priority) = self.get_sprite_pixel();
        let render_enable = self.mask.contains(MaskFlags::ENABLE_BACKGROUND_RENDERING)
            && self.mask.contains(MaskFlags::ENABLE_SPRITE_RENDERING);
        let sprite_zero = self.sprites.zero_rendered && self.sprites.zero_hit_possible;
        if render_enable && sprite_zero && sprite_pixel != 0 {
            let bleft = self.mask.contains(MaskFlags::BACKGROUND_LEFTMOST);
            let sleft = self.mask.contains(MaskFlags::SPRITE_LEFTMOST);
            let btdd_seam_compat = self.mem.rom.info.mapper_id == 7
                && self.mem.rom.info.submapper_id == 2
                && x >= 248;
            let hit_visible = bg_pixel != 0 || btdd_seam_compat;
            if hit_visible {
                if x < 8 {
                    if bleft && sleft {
                        self.status.set(StatusFlags::SPRITE_ZERO_HIT, true);
                    }
                } else if x < 255 {
                    self.status.set(StatusFlags::SPRITE_ZERO_HIT, true);
                }
            }
        }

        // Combine pixels
        let (pixel, palette) = match (bg_pixel, sprite_pixel) {
            (0, 0) => (0, 0),
            (0, _) => (sprite_pixel, sprite_palette),
            (_, 0) => (bg_pixel, bg_palette),
            (_, _) => {
                if sprite_priority {
                    (bg_pixel, bg_palette)
                } else {
                    (sprite_pixel, sprite_palette)
                }
            }
        };

        // Write to frame buffer
        let color_addr = 0x3F00 + ((palette as u16) << 2) + pixel as u16;
        let color = self.mem.read_palette(color_addr);
        self.frame.set_pixel(x, y, color & 0x3F);
    }


    fn get_sprite_pixel(&mut self) -> (u8, u8, bool) {
        if !self.mask.contains(MaskFlags::ENABLE_SPRITE_RENDERING) {
            return (0, 0, false);
        }

        // Mask leftmost 8 pixels if needed
        if !self.mask.contains(MaskFlags::SPRITE_LEFTMOST) && self.cycles <= 8 {
            return (0, 0, false);
        }

        self.sprites.get_pixel()
    }

    fn rendering_enabled(&self) -> bool {
        self.mask.contains(MaskFlags::ENABLE_BACKGROUND_RENDERING)
            || self.mask.contains(MaskFlags::ENABLE_SPRITE_RENDERING)
    }

    fn is_odd_frame_cycle(&self) -> bool {
        self.rendering_enabled()
            && self.scanlines == 261
            && self.cycles == 340
            && (self.frames % 2 != 0)
    }

    fn handle_odd_frame_cycle(&mut self) {
        self.cycles = 0;
        self.scanlines = self.scanlines.wrapping_add(1);

        if self.scanlines >= 262 {
            self.scanlines = 0;
            self.frames = self.frames.wrapping_add(1);
        }
    }

    fn update_counters(&mut self) {
        self.cycles += 1;

        if self.cycles >= 341 {
            self.cycles = 0;
            self.scanlines = self.scanlines.wrapping_add(1);

            if self.scanlines >= 262 {
                self.scanlines = 0;
                self.frames = self.frames.wrapping_add(1);
            }
        }
    }

    fn update_nmi_state(&mut self) {
        let next_line = self.status.contains(StatusFlags::VBLANK)
            && self.ctrl.contains(CtrlFlags::VBLANK_NMI_ENABLE);

        if !self.nmi_line && next_line {
            self.nmi_delay = Some(1);
        } else if !next_line {
            self.nmi_delay = None;
        }

        self.nmi_line = next_line;
    }

    fn clock_nmi_delay(&mut self) {
        let Some(delay) = self.nmi_delay else {
            return;
        };

        if delay <= 1 {
            self.nmi = true;
            self.nmi_delay = None;
        } else {
            self.nmi_delay = Some(delay - 1);
        }
    }

    fn handle_vblank(&mut self) {
        if self.scanlines != 241 || self.cycles != 1 {
            return;
        }
        if self.suppress_vblank_flag {
            self.suppress_vblank_flag = false;
            return;
        }
        self.status.set(StatusFlags::VBLANK, true);
        if self.ctrl.contains(CtrlFlags::VBLANK_NMI_ENABLE) {
            self.nmi_delay = Some(9);
        }
        self.nmi_line = self.ctrl.contains(CtrlFlags::VBLANK_NMI_ENABLE);
    }

    fn prerender_scanline(&mut self) {
        if self.scanlines != 261 || self.cycles != 1 {
            return;
        }
        self.status.remove(StatusFlags::VBLANK);
        self.status.remove(StatusFlags::SPRITE_ZERO_HIT);
        self.status.remove(StatusFlags::SPRITE_OVERFLOW);
        self.suppress_vblank_flag = false;
        self.update_nmi_state();
        self.sprites.zero_hit_possible = false;
    }

    fn update(&mut self) {
        if self.scanlines >= 240 && self.scanlines != 261 {
            return;
        }

        self.update_background();
        self.update_sprites();
        self.render_pixel();
        self.bg.update_x(self.cycles, &self.mask);
        if self.scanlines < 240 {
            self.sprites.update_x(self.cycles);
        }
    }

    fn step(&mut self) {
        self.update_counters();
        if self.is_odd_frame_cycle() {
            self.handle_odd_frame_cycle();
            return;
        }
        let absolute_cycle =
            self.frames as u64 * 262 * 341 + self.scanlines as u64 * 341 + self.cycles as u64;
        self.mem.set_ppu_cycle(absolute_cycle);
        self.handle_vblank();
        self.prerender_scanline();
        self.update()
        ;
        self.clock_nmi_delay();
    }
}

#[cfg(test)]
mod ppu_tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    use crate::rom::info::{Mirroring, RomInfo, TVFormat};
    use crate::rom::{Mapper, Rom};

    #[derive(Default)]
    struct ProbeState {
        qualified_a12_rise_cycles: Vec<u64>,
        a12_last_state: bool,
        a12_low_cycle: u64,
    }

    struct ProbeMapper {
        state: Rc<RefCell<ProbeState>>,
    }

    impl ProbeMapper {
        fn new(state: Rc<RefCell<ProbeState>>) -> Self {
            Self { state }
        }
    }

    impl Mapper for ProbeMapper {
        fn read_chr(&self, _addr: u16) -> u8 {
            0
        }

        fn write_chr(&mut self, _addr: u16, _value: u8) {}

        fn read_prg(&self, _addr: u16) -> u8 {
            0
        }

        fn write_prg(&mut self, _addr: u16, _value: u8) {}

        fn mirroring(&self) -> Option<Mirroring> {
            None
        }

        fn on_ppu_addr(&mut self, addr: u16, ppu_cycle: u64) {
            let a12 = (addr & 0x1000) != 0;
            let mut state = self.state.borrow_mut();

            if !a12 {
                if state.a12_last_state {
                    state.a12_low_cycle = ppu_cycle;
                }
                state.a12_last_state = false;
                return;
            }

            if !state.a12_last_state && ppu_cycle.saturating_sub(state.a12_low_cycle) >= 8 {
                state.qualified_a12_rise_cycles.push(ppu_cycle);
            }
            state.a12_last_state = true;
        }
    }

    fn probe_test_ppu() -> (Rc<RefCell<Ppu>>, Rc<RefCell<ProbeState>>) {
        let state = Rc::new(RefCell::new(ProbeState::default()));
        let rom = Rc::new(Rom {
            trainer: None,
            info: RomInfo {
                mapper_id: 4,
                submapper_id: 0,
                mirroring: Mirroring::Horizontal,
                tv_format: TVFormat::NTSC,
                has_battery: false,
                has_trainer: false,
                vs_unisystem: false,
                playchoice10: false,
                nes2_format: false,
                prg_rom_size: 0,
                chr_rom_size: 8 * 1024,
                prg_ram_size: 0,
                chr_ram_size: 0,
            },
            mapper: RefCell::new(Box::new(ProbeMapper::new(state.clone()))),
        });
        (Ppu::new(rom), state)
    }

    fn create_test_ppu() -> Rc<RefCell<Ppu>> {
        let rom = Rom::new("./tests/test_roms/cpu/nestest.nes").expect("Cannot load rom");
        Ppu::new(rom)
    }

    #[test]
    fn test_sprite_pattern_high_generates_241_filtered_a12_rises_per_frame() {
        let (ppu, probe_state) = probe_test_ppu();

        {
            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.ctrl(0x08);
            ppu_ref.write_mask(0x18);
            ppu_ref.step_cycles(262 * 341);
        }

        let probe = probe_state.borrow();
        assert_eq!(probe.qualified_a12_rise_cycles.len(), 241);
    }

    #[test]
    fn test_ppu_creation() {
        let ppu = create_test_ppu();
        let ppu_ref = ppu.borrow();
        
        assert_eq!(ppu_ref.cycles, 0);
        assert_eq!(ppu_ref.scanlines, 0);
        assert_eq!(ppu_ref.frames, 0);
    }

    #[test]
    fn test_ppu_reset() {
        let ppu = create_test_ppu();
        
        {
            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.cycles = 100;
            ppu_ref.scanlines = 200;
            ppu_ref.write_mask(0xFF);
            ppu_ref.ctrl(0xFF);
        }
        
        {
            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.reset();
        }
        
        let ppu_ref = ppu.borrow();
        assert_eq!(ppu_ref.cycles, 0);
        assert_eq!(ppu_ref.scanlines, 0);
        assert_eq!(ppu_ref.frames, 0);
    }

    #[test]
    fn test_ppuctrl_write() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Test nametable select
        ppu_ref.ctrl(0b00000011); // Nametable 3
        assert_eq!(ppu_ref.ctrl.bits() & 0x03, 0x03);
        
        // Test increment mode
        ppu_ref.ctrl(0b00000100); // +32 increment
        assert!(ppu_ref.ctrl.contains(CtrlFlags::INCREMENT_VRAM_ADDR));
    }

    #[test]
    fn test_ppumask_write() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.write_mask(0b00011000); // Show background and sprites
        assert!(ppu_ref.mask.contains(MaskFlags::ENABLE_BACKGROUND_RENDERING));
        assert!(ppu_ref.mask.contains(MaskFlags::ENABLE_SPRITE_RENDERING));
    }

    #[test]
    fn test_ppustatus_read() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Set VBlank flag
        ppu_ref.status.set(StatusFlags::VBLANK, true);
        
        let status = ppu_ref.read_status();
        assert!(status & 0x80 != 0); // VBlank bit set
        
        // Reading status should clear VBlank
        let status2 = ppu_ref.read_status();
        assert!(status2 & 0x80 == 0);
    }

    #[test]
    fn test_ppuaddr_write() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Write high byte
        ppu_ref.write_addr(0x20);
        assert_eq!(ppu_ref.addr.t & 0xFF00, 0x2000);
        
        // Write low byte
        ppu_ref.write_addr(0x00);
        assert_eq!(ppu_ref.addr.v, 0x2000);
    }

    #[test]
    fn test_ppuscroll_write() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Write X scroll
        ppu_ref.write_scroll(123);
        assert_eq!(ppu_ref.addr.x, 123 & 0x07); // Fine X
        
        // Write Y scroll
        ppu_ref.write_scroll(200);
    }

    #[test]
    fn test_oam_addr_write() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.write_oam_addr(0x10);
        assert_eq!(ppu_ref.oam.addr, 0x10);
    }

    #[test]
    fn test_oam_data_read_write() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.write_oam_addr(0x00);
        ppu_ref.write_oam_data(0x42);
        
        assert_eq!(ppu_ref.oam.addr, 0x01); // Auto-increment
        
        ppu_ref.write_oam_addr(0x00);
        assert_eq!(ppu_ref.read_oam_data(), 0x42);
    }

    #[test]
    fn test_oam_attribute_read_masks_unused_bits() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();

        ppu_ref.write_oam_addr(0x02);
        ppu_ref.write_oam_data(0xFF);

        ppu_ref.write_oam_addr(0x02);
        assert_eq!(ppu_ref.read_oam_data(), 0xE3);
    }

    #[test]
    fn test_oam_dma_starts_at_current_addr_and_wraps() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();

        ppu_ref.write_oam_addr(0xFE);
        let mut bytes = [0x44u8; 256];
        bytes[0] = 0x11;
        bytes[1] = 0x22;
        bytes[2] = 0x33;
        ppu_ref.write_oam_dma_bytes(&bytes);

        assert_eq!(ppu_ref.oam.data[0xFE], 0x11);
        assert_eq!(ppu_ref.oam.data[0xFF], 0x22);
        assert_eq!(ppu_ref.oam.data[0x00], 0x33);
        assert_eq!(ppu_ref.oam.addr, 0xFE);
    }

    #[test]
    fn test_vram_read_write() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Write to VRAM
        ppu_ref.write_addr(0x20); // High byte
        ppu_ref.write_addr(0x00); // Low byte
        ppu_ref.write(0xAB);
        
        // Read from VRAM (with buffering)
        ppu_ref.write_addr(0x20);
        ppu_ref.write_addr(0x00);
        ppu_ref.read(); // Dummy read
        let value = ppu_ref.read(); // Actual value
        
        assert_eq!(value, 0xAB);
    }

    #[test]
    fn test_vram_increment_mode() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Test +1 increment (horizontal)
        ppu_ref.ctrl(0x00);
        ppu_ref.write_addr(0x20);
        ppu_ref.write_addr(0x00);
        ppu_ref.write(0x01);
        assert_eq!(ppu_ref.addr.v, 0x2001);
        
        // Test +32 increment (vertical)
        ppu_ref.ctrl(0x04);
        ppu_ref.write_addr(0x20);
        ppu_ref.write_addr(0x00);
        ppu_ref.write(0x02);
        assert_eq!(ppu_ref.addr.v, 0x2020);
    }

    #[test]
    fn test_nmi_generation() {
        let ppu = create_test_ppu();
        
        {
            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.ctrl(0x80); // Enable NMI
        }
        
        // Step to scanline 241, cycle 1 (VBlank start)
        {
            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.scanlines = 241;
            ppu_ref.cycles = 0;
            ppu_ref.step_cycles(9);
        }
        
        let mut ppu_ref = ppu.borrow_mut();
        assert!(ppu_ref.poll_nmi());
        assert!(!ppu_ref.poll_nmi()); // Should clear after poll
    }

    #[test]
    fn test_vblank_flag() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Before VBlank
        assert!(!ppu_ref.status.contains(StatusFlags::VBLANK));
        
        // Step to VBlank
        ppu_ref.scanlines = 241;
        ppu_ref.cycles = 0;
        ppu_ref.step_cycles(1);
        
        assert!(ppu_ref.status.contains(StatusFlags::VBLANK));
    }

    #[test]
    fn test_vblank_clear_on_prerender() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Set VBlank
        ppu_ref.status.set(StatusFlags::VBLANK, true);
        
        // Step to pre-render scanline
        ppu_ref.scanlines = 261;
        ppu_ref.cycles = 0;
        ppu_ref.step_cycles(1);
        
        assert!(!ppu_ref.status.contains(StatusFlags::VBLANK));
    }

    #[test]
    fn test_frame_counter() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        let initial_frames = ppu_ref.frames;
        
        // Step one complete frame (262 scanlines * 341 cycles)
        ppu_ref.step_cycles(262 * 341);
        
        assert_eq!(ppu_ref.frames, initial_frames + 1);
    }

    #[test]
    fn test_scanline_counter() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        assert_eq!(ppu_ref.scanlines, 0);
        
        // Step one scanline
        ppu_ref.step_cycles(341);
        
        assert_eq!(ppu_ref.scanlines, 1);
        assert_eq!(ppu_ref.cycles, 0);
    }

    #[test]
    fn test_scanline_wraparound() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.scanlines = 261;
        ppu_ref.cycles = 340;

        ppu_ref.step_cycles(1);
        
        assert_eq!(ppu_ref.scanlines, 0);
        assert_eq!(ppu_ref.cycles, 0);
    }

    #[test]
    fn test_rendering_disabled_by_default() {
        let ppu = create_test_ppu();
        let ppu_ref = ppu.borrow();
        
        assert!(!ppu_ref.rendering_enabled());
    }

    #[test]
    fn test_rendering_enabled() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.write_mask(0x08); // Enable background
        assert!(ppu_ref.rendering_enabled());
        
        ppu_ref.write_mask(0x10); // Enable sprites
        assert!(ppu_ref.rendering_enabled());
    }

    #[test]
    fn test_background_pattern_table_select() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Pattern table 0
        ppu_ref.ctrl(0x00);
        assert!(!ppu_ref.ctrl.contains(CtrlFlags::BG_PATTERN));
        
        // Pattern table 1
        ppu_ref.ctrl(0x10);
        assert!(ppu_ref.ctrl.contains(CtrlFlags::BG_PATTERN));
    }

    #[test]
    fn test_sprite_pattern_table_select() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Pattern table 0
        ppu_ref.ctrl(0x00);
        assert!(!ppu_ref.ctrl.contains(CtrlFlags::SPRITE_PATTERN));
        
        // Pattern table 1
        ppu_ref.ctrl(0x08);
        assert!(ppu_ref.ctrl.contains(CtrlFlags::SPRITE_PATTERN));
    }

    #[test]
    fn test_sprite_size_8x8() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.ctrl(0x00);
        assert!(!ppu_ref.ctrl.contains(CtrlFlags::SPRITE_SIZE));
    }

    #[test]
    fn test_sprite_size_8x16() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.ctrl(0x20);
        assert!(ppu_ref.ctrl.contains(CtrlFlags::SPRITE_SIZE));
    }

    #[test]
    fn test_status_read_clears_write_latch() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Write first byte to PPUADDR
        ppu_ref.write_addr(0x20);
        assert!(ppu_ref.addr.w);
        
        // Read status
        ppu_ref.read_status();
        
        // Write latch should be cleared
        assert!(!ppu_ref.addr.w);
    }

    #[test]
    fn test_multiple_frames() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        let cycles_per_frame = 262 * 341;
        
        for i in 0..5 {
            ppu_ref.step_cycles(cycles_per_frame);
            assert_eq!(ppu_ref.frames, i + 1);
        }
    }

    #[test]
    fn test_nmi_not_generated_when_disabled() {
        let ppu = create_test_ppu();
        
        {
            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.ctrl(0x00); // NMI disabled
        }
        
        // Step to VBlank
        {
            let mut ppu_ref = ppu.borrow_mut();
            ppu_ref.scanlines = 240;
            ppu_ref.cycles = 340;
            ppu_ref.step_cycles(1);
        }
        
        let mut ppu_ref = ppu.borrow_mut();
        assert!(!ppu_ref.poll_nmi());
    }

    #[test]
    fn test_nmi_enable_during_vblank() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        // Enter VBlank with NMI disabled
        ppu_ref.ctrl(0x00);
        ppu_ref.scanlines = 241;
        ppu_ref.cycles = 0;
        ppu_ref.step_cycles(1);
        
        assert!(ppu_ref.status.contains(StatusFlags::VBLANK));
        assert!(!ppu_ref.poll_nmi());
        
        // Enable NMI during VBlank
        ppu_ref.ctrl(0x80);
        ppu_ref.step_cycles(1);
        assert!(ppu_ref.poll_nmi());
    }

    #[test]
    fn test_odd_frame_skips_last_cycle_when_rendering_enabled() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();

        ppu_ref.write_mask(0x08);
        ppu_ref.frames = 1;
        ppu_ref.scanlines = 261;
        ppu_ref.cycles = 339;

        ppu_ref.step();

        assert_eq!(ppu_ref.scanlines, 0);
        assert_eq!(ppu_ref.cycles, 0);
        assert_eq!(ppu_ref.frames, 2);
    }

    #[test]
    fn test_status_read_at_vblank_start_suppresses_vblank_and_nmi() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();

        ppu_ref.ctrl(0x80);
        ppu_ref.scanlines = 241;
        ppu_ref.cycles = 0;
        let status = ppu_ref.read_status();

        assert_eq!(status & 0x80, 0);
        ppu_ref.handle_vblank();
        assert!(!ppu_ref.status.contains(StatusFlags::VBLANK));
        assert!(!ppu_ref.poll_nmi());
    }

    #[test]
    fn test_oam_data_auto_increment() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.write_oam_addr(0x00);
        
        for i in 0..10 {
            ppu_ref.write_oam_data(i);
        }
        
        assert_eq!(ppu_ref.oam.addr, 10);
    }

    #[test]
    fn test_oam_addr_wraparound() {
        let ppu = create_test_ppu();
        let mut ppu_ref = ppu.borrow_mut();
        
        ppu_ref.write_oam_addr(0xFF);
        ppu_ref.write_oam_data(0x42);
        
        assert_eq!(ppu_ref.oam.addr, 0x00); // Should wrap
    }
}

impl Default for Oam {
    fn default() -> Self {
        Self{ addr: 0, data: [0; 0x100] }
    }
}

impl Memory {
    pub fn new(rom: RomRef) -> Self {
        Self {
            vmem: [0; 0x800],
            rom,
            palette: POWER_UP_PALETTE,
            ppu_cycle: 0,
        }
    }

    pub fn set_ppu_cycle(&mut self, cycle: u64) {
        self.ppu_cycle = cycle;
    }

    pub fn ppu_cycle(&self) -> u64 {
        self.ppu_cycle
    }

    pub fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x1FFF => {
                let mut mapper = self.rom.mapper.borrow_mut();
                mapper.on_ppu_addr(addr, self.ppu_cycle + 1);
                mapper.read_chr(addr)
            } // CHR ROM/RAM
            0x2000..=0x2FFF => {
                if let Some(v) = self.rom.mapper.borrow().read_nametable(addr) {
                    v
                } else {
                    self.vmem[self.mirror_vram_addr(addr)]
                }
            } // VRAM + mirroring
            0x3000..=0x3EFF => {
                let mirrored = addr - 0x1000;
                if let Some(v) = self.rom.mapper.borrow().read_nametable(mirrored) {
                    v
                } else {
                    self.vmem[self.mirror_vram_addr(addr)]
                }
            } // mirror $2000-$2EFF
            _ => 0,                                                     // $3F00–$3FFF handled separately
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                let mut mapper = self.rom.mapper.borrow_mut();
                mapper.on_ppu_addr(addr, self.ppu_cycle + 1);
                mapper.write_chr(addr, value);
            } // CHR RAM, if it avalible
            0x2000..=0x2FFF => {
                if !self.rom.mapper.borrow_mut().write_nametable(addr, value) {
                    self.vmem[self.mirror_vram_addr(addr)] = value;
                }
            } // VRAM
            0x3000..=0x3EFF => {
                let mirrored = addr - 0x1000;
                if !self.rom.mapper.borrow_mut().write_nametable(mirrored, value) {
                    self.vmem[self.mirror_vram_addr(addr)] = value;
                }
            } // mirror
            0x3F00..=0x3FFF => self.write_palette(addr, value),
            _ => {}
        }
    }

    pub fn reset(&mut self) {
        self.ppu_cycle = 0;
    }

    pub fn write_palette(&mut self, addr: u16, value: u8) {
        let mut index = addr & 0x1F;
        if index >= 0x10 && index % 4 == 0 {
            index -= 0x10;
        }
        self.palette[index as usize] = value;
    }

    pub fn read_palette(&self, addr: u16) -> u8 {
        let mut index = addr & 0x1F;
        // $3F10/$3F14/$3F18/$3F1C mirror to $3F00/$3F04/$3F08/$3F0C
        if index >= 0x10 && index % 4 == 0 {
            index -= 0x10;
        }
        self.palette[index as usize]
    }

    fn mirror_vram_addr(&self, addr: u16) -> usize {
        if let Some(mapped) = self.rom.mapper.borrow().map_nametable_addr(addr) {
            return mapped & 0x7FF;
        }

        let addr = (addr - 0x2000) % 0x1000;
        let table = addr / 0x400;
        let offset = addr % 0x400;

        let vram_index = match self.rom.mirroring() {
            Mirroring::Vertical => (table % 2) * 0x400 + offset,
            Mirroring::Horizontal => (table / 2) * 0x400 + offset,
            Mirroring::FourScreen => table * 0x400 + offset,
            Mirroring::SingleScreenLower => offset,
            Mirroring::SingleScreenUpper => 0x400 + offset,
        };
        vram_index as usize
    }
}
