use super::Mapper;
use super::Mirroring;

pub struct ColorDreams11 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    prg_bank: u8,
    chr_bank: u8,
}

impl ColorDreams11 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            prg_bank: 0,
            chr_bank: 0,
        }
    }

    fn prg_32k_bank_count(&self) -> usize {
        (self.prg_rom.len() / (32 * 1024)).max(1)
    }

    fn chr_8k_bank_count(&self) -> usize {
        if self.chr_rom.is_empty() {
            1
        } else {
            (self.chr_rom.len() / (8 * 1024)).max(1)
        }
    }
}

impl Mapper for ColorDreams11 {
    fn read_chr(&self, addr: u16) -> u8 {
        let offset = addr as usize & 0x1FFF;
        if self.chr_rom.is_empty() {
            self.chr_ram[offset % self.chr_ram.len()]
        } else {
            let bank = (self.chr_bank as usize) % self.chr_8k_bank_count();
            self.chr_rom[bank * 8 * 1024 + offset]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if self.chr_rom.is_empty() {
            let offset = addr as usize & 0x1FFF;
            let index = offset % self.chr_ram.len();
            self.chr_ram[index] = value;
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                let offset = (addr as usize - 0x6000) & 0x1FFF;
                self.prg_ram[offset]
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
                let offset = (addr as usize - 0x6000) & 0x1FFF;
                self.prg_ram[offset] = value;
            }
            0x8000..=0xFFFF => {
                // Color Dreams (mapper 11): low nibble selects 32 KiB PRG bank,
                // high nibble selects 8 KiB CHR bank.
                self.prg_bank = value & 0x0F;
                self.chr_bank = (value >> 4) & 0x0F;
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        None
    }
}
