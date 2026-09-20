//
// MMC5 (Mapper 5) - incremental implementation focused on test ROM bring-up.
// Covers:
// - PRG/CHR mode registers
// - basic PRG/CHR banking
// - basic nametable source mapping (CIRAM pages only) via $5105
//
use super::Mapper;
use super::Mirroring;
use std::cell::Cell;

pub struct MMC5 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    exram: [u8; 1024],

    prg_mode: u8,    // $5100
    chr_mode: u8,    // $5101
    exram_mode: u8,  // $5104
    nt_map: u8,      // $5105
    fill_tile: u8,   // $5106
    fill_attr: u8,   // $5107

    chr_upper: u8,             // $5130 (high CHR bank bits)
    prg_ram_bank: u8,          // $5113
    prg_bank: [u8; 4],         // $5114..$5117
    chr_bank: [u8; 8],         // $5120..$5127

    split_control: u8, // $5200
    irq_scanline: u8,  // $5203
    irq_control: u8,   // $5204
    irq_pending: Cell<bool>,
    in_frame: Cell<bool>,
    last_scanline: Cell<u16>,
    mul_a: u8, // $5205
    mul_b: u8, // $5206
}

impl MMC5 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mut chr_ram: Vec<u8>) -> Self {
        if chr_rom.is_empty() && chr_ram.is_empty() {
            chr_ram = vec![0; 8 * 1024];
        }

        Self {
            prg_rom,
            prg_ram: vec![0; 64 * 1024],
            chr_rom,
            chr_ram,
            exram: [0; 1024],
            prg_mode: 3,
            chr_mode: 3,
            exram_mode: 0,
            nt_map: 0,
            fill_tile: 0,
            fill_attr: 0,
            chr_upper: 0,
            prg_ram_bank: 0,
            prg_bank: [0, 0, 0, 0xFF],
            chr_bank: [0, 1, 2, 3, 4, 5, 6, 7],
            split_control: 0,
            irq_scanline: 0,
            irq_control: 0,
            irq_pending: Cell::new(false),
            in_frame: Cell::new(false),
            last_scanline: Cell::new(0xFFFF),
            mul_a: 0,
            mul_b: 0,
        }
    }

    fn prg_8k_banks(&self) -> usize {
        (self.prg_rom.len() / 0x2000).max(1)
    }

    fn prg_8k_index(&self, bank_reg: u8) -> usize {
        // MMC5 PRG regs use 7-bit bank number (bit7 selects ROM/RAM on hardware).
        let bank = (bank_reg & 0x7F) as usize;
        bank % self.prg_8k_banks()
    }

    fn read_prg_8k(&self, bank: usize, addr: u16) -> u8 {
        let base = (bank % self.prg_8k_banks()) * 0x2000;
        self.prg_rom[base + (addr as usize & 0x1FFF)]
    }

    fn prg_ram_read(&self, addr: u16) -> u8 {
        let bank = (self.prg_ram_bank & 0x0F) as usize;
        let idx = bank * 0x2000 + (addr as usize & 0x1FFF);
        self.prg_ram[idx % self.prg_ram.len()]
    }

    fn prg_ram_write(&mut self, addr: u16, value: u8) {
        let bank = (self.prg_ram_bank & 0x0F) as usize;
        let idx = bank * 0x2000 + (addr as usize & 0x1FFF);
        let idx = idx % self.prg_ram.len();
        self.prg_ram[idx] = value;
    }

    fn read_prg_by_mode(&self, addr: u16) -> u8 {
        let slot = ((addr - 0x8000) / 0x2000) as usize; // 0..3
        match self.prg_mode & 0x03 {
            // 32 KiB bank from $5117 (coarse)
            0 => {
                let bank32 = (self.prg_8k_index(self.prg_bank[3]) & !0x03) + slot;
                self.read_prg_8k(bank32, addr)
            }
            // 16 KiB at $8000 from $5115, fixed-ish upper 16 KiB from $5117
            1 => {
                let bank = if slot < 2 {
                    (self.prg_8k_index(self.prg_bank[1]) & !0x01) + slot
                } else {
                    (self.prg_8k_index(self.prg_bank[3]) & !0x01) + (slot - 2)
                };
                self.read_prg_8k(bank, addr)
            }
            // 16 KiB at $8000 from $5115, then 8 KiB $C000=$5116, $E000=$5117
            2 => {
                let bank = match slot {
                    0 | 1 => (self.prg_8k_index(self.prg_bank[1]) & !0x01) + slot,
                    2 => self.prg_8k_index(self.prg_bank[2]),
                    _ => self.prg_8k_index(self.prg_bank[3]),
                };
                self.read_prg_8k(bank, addr)
            }
            // 4x8 KiB from $5114..$5117
            _ => {
                let bank = self.prg_8k_index(self.prg_bank[slot]);
                self.read_prg_8k(bank, addr)
            }
        }
    }

    fn chr_1k_banks(&self) -> usize {
        let bytes = if !self.chr_rom.is_empty() {
            self.chr_rom.len()
        } else {
            self.chr_ram.len()
        };
        (bytes / 1024).max(1)
    }

    fn chr_index(&self, reg: u8, addr: u16) -> usize {
        let bank = (((self.chr_upper as usize) << 8) | reg as usize) % self.chr_1k_banks();
        bank * 1024 + (addr as usize & 0x03FF)
    }

    fn chr_reg_for_addr(&self, addr: u16) -> u8 {
        let seg = ((addr as usize) & 0x1FFF) / 0x0400; // 0..7
        match self.chr_mode & 0x03 {
            0 => self.chr_bank[7],
            1 => {
                if seg < 4 {
                    self.chr_bank[3]
                } else {
                    self.chr_bank[7]
                }
            }
            2 => self.chr_bank[(seg & !1) + 1],
            _ => self.chr_bank[seg],
        }
    }

    fn nt_source(&self, addr: u16) -> (u8, usize) {
        let off = (addr - 0x2000) % 0x1000;
        let table = (off / 0x400) as usize;
        let index = (off % 0x400) as usize;
        let source = (self.nt_map >> (table * 2)) & 0x03;
        (source, index)
    }
}

impl Mapper for MMC5 {
    fn read_chr(&self, addr: u16) -> u8 {
        let reg = self.chr_reg_for_addr(addr);
        let idx = self.chr_index(reg, addr);
        if !self.chr_rom.is_empty() {
            self.chr_rom[idx % self.chr_rom.len()]
        } else {
            self.chr_ram[idx % self.chr_ram.len()]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if self.chr_ram.is_empty() {
            return;
        }
        let reg = self.chr_reg_for_addr(addr);
        let idx = self.chr_index(reg, addr) % self.chr_ram.len();
        self.chr_ram[idx] = value;
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x5204 => {
                let irq = if self.irq_pending.get() { 0x80 } else { 0x00 };
                let in_frame = if self.in_frame.get() { 0x40 } else { 0x00 };
                self.irq_pending.set(false);
                irq | in_frame
            }
            0x5205 => {
                let prod = (self.mul_a as u16) * (self.mul_b as u16);
                (prod & 0x00FF) as u8
            }
            0x5206 => {
                let prod = (self.mul_a as u16) * (self.mul_b as u16);
                ((prod >> 8) & 0x00FF) as u8
            }
            0x5C00..=0x5FFF => self.exram[(addr as usize - 0x5C00) & 0x03FF],
            0x6000..=0x7FFF => self.prg_ram_read(addr),
            0x8000..=0xFFFF => self.read_prg_by_mode(addr),
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match addr {
            0x5C00..=0x5FFF => {
                // Keep ExRAM writable in all modes for now (good enough for tests bring-up).
                if self.exram_mode <= 3 {
                    self.exram[(addr as usize - 0x5C00) & 0x03FF] = value;
                }
            }
            0x6000..=0x7FFF => self.prg_ram_write(addr, value),
            0x5100 => self.prg_mode = value & 0x03,
            0x5101 => self.chr_mode = value & 0x03,
            0x5104 => self.exram_mode = value & 0x03,
            0x5105 => self.nt_map = value,
            0x5106 => self.fill_tile = value,
            0x5107 => self.fill_attr = value & 0x03,
            0x5113 => self.prg_ram_bank = value,
            0x5114..=0x5117 => {
                self.prg_bank[(addr as usize - 0x5114) & 0x03] = value;
            }
            0x5120..=0x5127 => {
                self.chr_bank[(addr as usize - 0x5120) & 0x07] = value;
            }
            0x5130 => self.chr_upper = value & 0x03,
            0x5200 => self.split_control = value,
            0x5203 => self.irq_scanline = value,
            0x5204 => {
                self.irq_control = value;
                if (value & 0x80) == 0 {
                    self.irq_pending.set(false);
                }
            }
            0x5205 => self.mul_a = value,
            0x5206 => self.mul_b = value,
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        None
    }

    fn read_nametable(&self, addr: u16) -> Option<u8> {
        let (source, index) = self.nt_source(addr);
        match source {
            2 => Some(self.exram[index]),
            3 => {
                if index < 0x03C0 {
                    Some(self.fill_tile)
                } else {
                    let a = self.fill_attr & 0x03;
                    Some(a | (a << 2) | (a << 4) | (a << 6))
                }
            }
            _ => None,
        }
    }

    fn write_nametable(&mut self, addr: u16, value: u8) -> bool {
        let (source, index) = self.nt_source(addr);
        match source {
            2 => {
                self.exram[index] = value;
                true
            }
            3 => true, // fill mode is read-only from PPU point of view
            _ => false,
        }
    }

    fn map_nametable_addr(&self, addr: u16) -> Option<usize> {
        let (source, index) = self.nt_source(addr);
        match source {
            0 => Some(index),          // CIRAM page 0
            1 => Some(0x400 + index),  // CIRAM page 1
            _ => None,
        }
    }

    fn on_ppu_addr(&mut self, _addr: u16, ppu_cycle: u64) {
        let scanline = ((ppu_cycle / 341) % 262) as u16;
        if self.last_scanline.get() == scanline {
            return;
        }
        self.last_scanline.set(scanline);

        if scanline < 240 {
            self.in_frame.set(true);
            if scanline as u8 == self.irq_scanline && (self.irq_control & 0x80) != 0 {
                self.irq_pending.set(true);
            }
        } else if scanline >= 241 {
            self.in_frame.set(false);
        }
    }

    fn poll_irq(&mut self) -> bool {
        self.irq_pending.get()
    }
}
