//
// Mapper 22 (Konami VRC2a) - minimal implementation for test ROM bring-up.
//
use super::Mapper;
use super::Mirroring;

pub struct Vrc2a22 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,

    mirroring: Mirroring,
    prg_bank_8000: u8,
    prg_bank_a000: u8,
    chr_bank_low: [u8; 8],
    chr_bank_high: [u8; 8],
}

impl Vrc2a22 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mut chr_ram: Vec<u8>) -> Self {
        if chr_rom.is_empty() && chr_ram.is_empty() {
            chr_ram = vec![0; 8 * 1024];
        }

        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            mirroring: Mirroring::Vertical,
            prg_bank_8000: 0,
            prg_bank_a000: 1,
            chr_bank_low: [0; 8],
            chr_bank_high: [0; 8],
        }
    }

    fn prg_8k_banks(&self) -> usize {
        (self.prg_rom.len() / 0x2000).max(1)
    }

    fn read_prg_bank(&self, bank: usize, addr: u16) -> u8 {
        let bank = bank % self.prg_8k_banks();
        self.prg_rom[bank * 0x2000 + (addr as usize & 0x1FFF)]
    }

    fn chr_1k_banks(&self) -> usize {
        let bytes = if !self.chr_rom.is_empty() {
            self.chr_rom.len()
        } else {
            self.chr_ram.len()
        };
        (bytes / 1024).max(1)
    }

    fn chr_bank_value(&self, i: usize) -> usize {
        // Mapper 22 (VRC2a) uses 8-bit regs where effective bank is shifted right by 1.
        let raw = (self.chr_bank_high[i] << 4) | (self.chr_bank_low[i] & 0x0F);
        ((raw >> 1) as usize) % self.chr_1k_banks()
    }

    fn chr_index(&self, addr: u16) -> usize {
        let segment = ((addr as usize) & 0x1FFF) / 0x0400;
        let bank = self.chr_bank_value(segment);
        bank * 1024 + (addr as usize & 0x03FF)
    }

    fn decode_chr_reg(addr: u16) -> Option<(usize, bool)> {
        let base = match addr & 0xF000 {
            0xB000 => 0usize,
            0xC000 => 2usize,
            0xD000 => 4usize,
            0xE000 => 6usize,
            _ => return None,
        };
        let sub = (addr & 0x0003) as usize;
        // Mapper 22 (VRC2a) has A0/A1 wiring differences versus later VRC2 variants.
        // This decode matches the "swapped" nibble selection used by test ROMs.
        let reg = base + (sub & 0x01);
        let high = (sub & 0x02) != 0;
        Some((reg, high))
    }
}

impl Mapper for Vrc2a22 {
    fn read_chr(&self, addr: u16) -> u8 {
        let idx = self.chr_index(addr);
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
        let idx = self.chr_index(addr) % self.chr_ram.len();
        self.chr_ram[idx] = value;
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr as usize - 0x6000) & 0x1FFF],
            0x8000..=0x9FFF => self.read_prg_bank(self.prg_bank_8000 as usize, addr),
            0xA000..=0xBFFF => self.read_prg_bank(self.prg_bank_a000 as usize, addr),
            0xC000..=0xDFFF => self.read_prg_bank(self.prg_8k_banks().saturating_sub(2), addr),
            0xE000..=0xFFFF => self.read_prg_bank(self.prg_8k_banks().saturating_sub(1), addr),
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr as usize - 0x6000) & 0x1FFF] = value;
            }
            0x8000..=0x8FFF => {
                self.prg_bank_8000 = value & 0x1F;
            }
            0xA000..=0xAFFF => {
                self.prg_bank_a000 = value & 0x1F;
            }
            0x9000..=0x9FFF if (addr & 0x0003) == 0 => {
                self.mirroring = match value & 0x03 {
                    0 => Mirroring::Vertical,
                    1 => Mirroring::Horizontal,
                    2 => Mirroring::SingleScreenLower,
                    _ => Mirroring::SingleScreenUpper,
                };
            }
            0xB000..=0xEFFF => {
                if let Some((reg, high)) = Self::decode_chr_reg(addr) {
                    if high {
                        self.chr_bank_high[reg] = value & 0x0F;
                    } else {
                        self.chr_bank_low[reg] = value & 0x0F;
                    }
                }
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        Some(self.mirroring)
    }
}
