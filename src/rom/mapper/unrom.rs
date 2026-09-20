use super::Mapper;
use super::Mirroring;

pub struct UNROM {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,

    prg_bank: u8, // текущая переключаемая PRG-банка
}

impl UNROM {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            prg_bank: 0,
        }
    }

    fn get_prg_bank(&self, addr: u16) -> usize {
        let num_banks = self.prg_rom.len() / (16 * 1024);
        let bank = if addr < 0xC000 {
            self.prg_bank as usize
        } else {
            num_banks - 1 // последняя фиксированная PRG-банка
        };
        bank % num_banks
    }
}

impl Mapper for UNROM {
    fn read_chr(&self, addr: u16) -> u8 {
        let addr = addr as usize & 0x1FFF;
        if self.chr_rom.is_empty() {
            self.chr_ram[addr]
        } else {
            self.chr_rom[addr]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if self.chr_rom.is_empty() {
            let addr = addr as usize & 0x1FFF;
            self.chr_ram[addr] = value;
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                let addr = (addr as usize - 0x6000) & 0x1FFF;
                self.prg_ram[addr]
            }
            0x8000..=0xFFFF => {
                let bank = self.get_prg_bank(addr);
                let offset = addr as usize & 0x3FFF;
                self.prg_rom[bank * 16 * 1024 + offset]
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match addr {
            0x6000..=0x7FFF => {
                let addr = (addr as usize - 0x6000) & 0x1FFF;
                self.prg_ram[addr] = value;
            }
            0x8000..=0xFFFF => {
                // Mapper 2: любая запись в $8000-$FFFF меняет переключаемую банку
                self.prg_bank = value & 0x0F; // ограничиваем до 16 банков (макс 256 КБ)
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> { None }
}

