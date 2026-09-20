//
// https://www.nesdev.org/wiki/Programming_NROM
//
use super::Mapper;
use super::Mirroring;

pub struct NROM {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
}

impl NROM {
    pub fn new(
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        chr_ram: Vec<u8>,
    ) -> Self {
        Self { prg_rom, prg_ram: vec![0; 8 * 1024], chr_rom, chr_ram }
    }
}

impl Mapper for NROM {
    fn read_chr(&self, addr: u16) -> u8 {
        let addr = addr as usize & 0x1FFF;

        if !self.chr_ram.is_empty() {
            self.chr_ram[addr]
        } else {
            self.chr_rom[addr]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if !self.chr_ram.is_empty() {
            let addr = addr as usize & 0x1FFF;
            self.chr_ram[addr] = value;
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr as usize - 0x6000) & 0x1FFF],
            0x8000..=0xFFFF => {
                let mut mapped = addr as usize - 0x8000;
                if self.prg_rom.len() == 16 * 1024 {
                    mapped %= 16 * 1024;
                }
                self.prg_rom[mapped]
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        if (0x6000..=0x7FFF).contains(&addr) {
            self.prg_ram[(addr as usize - 0x6000) & 0x1FFF] = value;
        }
    }

    fn mirroring(&self) -> Option<Mirroring> { None }
}
