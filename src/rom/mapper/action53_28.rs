use super::Mapper;
use super::Mirroring;

pub struct Action53_28 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,

    reg_select: u8,
    reg_00_chr_mirr: u8,
    reg_01_inner_bank: u8,
    reg_80_mode: u8,
    reg_81_outer_bank: u8,
    one_screen_bit: u8,
}

impl Action53_28 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mut chr_ram: Vec<u8>) -> Self {
        // Action 53 multicarts commonly use up to 32 KiB CHR-RAM.
        if chr_rom.is_empty() && chr_ram.len() < 32 * 1024 {
            chr_ram.resize(32 * 1024, 0);
        }

        let prg_16k_banks = (prg_rom.len() / (16 * 1024)).max(1);
        let default_outer_bank = ((prg_16k_banks / 2).saturating_sub(1)).min(0x7F) as u8;

        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            reg_select: 0x00,
            reg_00_chr_mirr: 0x00,
            reg_01_inner_bank: 0x00,
            reg_80_mode: 0x00,
            reg_81_outer_bank: default_outer_bank,
            one_screen_bit: 0,
        }
    }

    fn prg_16k_bank_count(&self) -> usize {
        (self.prg_rom.len() / (16 * 1024)).max(1)
    }

    fn chr_8k_bank_count_rom(&self) -> usize {
        (self.chr_rom.len() / (8 * 1024)).max(1)
    }

    fn chr_8k_bank_count_ram(&self) -> usize {
        (self.chr_ram.len() / (8 * 1024)).max(1)
    }

    fn effective_mirroring_mode(&self) -> u8 {
        // bits 0-1 of reg_80_mode choose mirroring:
        // 0,1: one-screen (bit selected by one_screen_bit)
        // 2: vertical
        // 3: horizontal
        let mm = self.reg_80_mode & 0x03;
        if mm < 2 {
            (mm & 0x02) | (self.one_screen_bit & 0x01)
        } else {
            mm
        }
    }

    fn action53_prg_bank_16k(&self, cpu_a14: bool) -> usize {
        // Reference: NESdev Action53 formulas (bank_mode/current_bank/outer_bank).
        let mut bank_mode = self.reg_80_mode >> 2;
        let mut current_bank = self.reg_01_inner_bank;
        let mut outer = self.reg_81_outer_bank;

        // Incorporate CPU A14 into outer/current bank path.
        outer = (outer << 1) | u8::from(cpu_a14);

        // UxROM-style modes can fix one 16 KiB half.
        if (bank_mode & 0x02) != 0 && ((outer ^ bank_mode) & 0x01) == 0 {
            bank_mode = 0;
        }

        // For non-UxROM path, include A14 into current bank bit 0.
        if (bank_mode & 0x02) == 0 {
            current_bank = (current_bank << 1) | (outer & 0x01);
        }

        // Select how many lower bank bits can vary.
        let size_index = (bank_mode >> 2) & 0x03;
        let mask = match size_index {
            0 => 0x01,
            1 => 0x03,
            2 => 0x07,
            _ => 0x0F,
        };

        let bank = (((current_bank ^ outer) & mask) ^ outer) as usize;
        bank % self.prg_16k_bank_count()
    }
}

impl Mapper for Action53_28 {
    fn read_chr(&self, addr: u16) -> u8 {
        let offset = addr as usize & 0x1FFF;
        let chr_bank = (self.reg_00_chr_mirr & 0x03) as usize;

        if self.chr_rom.is_empty() {
            let bank = chr_bank % self.chr_8k_bank_count_ram();
            self.chr_ram[bank * 8 * 1024 + offset]
        } else {
            let bank = chr_bank % self.chr_8k_bank_count_rom();
            self.chr_rom[bank * 8 * 1024 + offset]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if self.chr_rom.is_empty() {
            let offset = addr as usize & 0x1FFF;
            let chr_bank = (self.reg_00_chr_mirr & 0x03) as usize;
            let bank = chr_bank % self.chr_8k_bank_count_ram();
            self.chr_ram[bank * 8 * 1024 + offset] = value;
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                let offset = (addr as usize - 0x6000) & 0x1FFF;
                self.prg_ram[offset]
            }
            0x8000..=0xBFFF => {
                let bank = self.action53_prg_bank_16k(false);
                let offset = (addr as usize - 0x8000) & 0x3FFF;
                self.prg_rom[bank * 16 * 1024 + offset]
            }
            0xC000..=0xFFFF => {
                let bank = self.action53_prg_bank_16k(true);
                let offset = (addr as usize - 0xC000) & 0x3FFF;
                self.prg_rom[bank * 16 * 1024 + offset]
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match addr {
            0x5000..=0x5FFF => {
                // Register select is decoded from CPU address lines A7/A0.
                self.reg_select = (addr & 0x81) as u8;
            }
            0x6000..=0x7FFF => {
                let offset = (addr as usize - 0x6000) & 0x1FFF;
                self.prg_ram[offset] = value;
            }
            0x8000..=0xFFFF => match self.reg_select {
                0x00 => {
                    self.reg_00_chr_mirr = value;
                    self.one_screen_bit = (value >> 4) & 0x01;
                }
                0x01 => {
                    self.reg_01_inner_bank = value;
                    self.one_screen_bit = (value >> 4) & 0x01;
                }
                0x80 => {
                    self.reg_80_mode = value;
                }
                0x81 => {
                    self.reg_81_outer_bank = value;
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        Some(match self.effective_mirroring_mode() {
            0 => Mirroring::SingleScreenLower,
            1 => Mirroring::SingleScreenUpper,
            2 => Mirroring::Vertical,
            _ => Mirroring::Horizontal,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_prg(banks_16k: usize) -> Vec<u8> {
        let mut prg = vec![0u8; banks_16k * 16 * 1024];
        for b in 0..banks_16k {
            let base = b * 16 * 1024;
            // Tag each bank by filling the first byte with its index.
            prg[base] = b as u8;
        }
        prg
    }

    #[test]
    fn defaults_to_last_outer_half_range() {
        let mapper = Action53_28::new(make_prg(32), vec![], vec![0; 32 * 1024]);
        // 32x16K => default outer bank 15 => $8000 maps bank 30, $C000 maps bank 31.
        assert_eq!(mapper.read_prg(0x8000), 30);
        assert_eq!(mapper.read_prg(0xC000), 31);
    }

    #[test]
    fn register_select_comes_from_address_lines() {
        let mut mapper = Action53_28::new(make_prg(32), vec![], vec![0; 32 * 1024]);

        // Select register 0x81 through address decode and write outer bank.
        mapper.write_prg(0x5081, 0x00);
        mapper.write_prg(0x8000, 0x03);

        // Select register 0x80 and force mode 0.
        mapper.write_prg(0x5080, 0x00);
        mapper.write_prg(0x8000, 0x00);

        // In mode 0, outer=3 should map to banks 6/7.
        assert_eq!(mapper.read_prg(0x8000), 6);
        assert_eq!(mapper.read_prg(0xC000), 7);
    }
}
