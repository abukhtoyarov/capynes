use super::Mapper;
use super::Mirroring;

pub struct Bnrom34 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_bank: u8,
}

impl Bnrom34 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            prg_bank: 0,
        }
    }

    fn prg_32k_bank_count(&self) -> usize {
        (self.prg_rom.len() / (32 * 1024)).max(1)
    }
}

impl Mapper for Bnrom34 {
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
                let bank = (self.prg_bank as usize) % self.prg_32k_bank_count();
                let offset = (addr as usize - 0x8000) & 0x7FFF;
                self.prg_rom[bank * 32 * 1024 + offset]
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
                // BNROM: writing any value selects 32KB PRG bank at $8000-$FFFF.
                self.prg_bank = value;
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        None
    }
}
