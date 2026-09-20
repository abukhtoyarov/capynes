//
// https://www.nesdev.org/wiki/CNROM
// Mapper 3
//
use super::Mapper;
use super::Mirroring;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CNROMBoard {
    Mapper3,
    Mapper185,
}

pub struct CNROM {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    chr_bank: u8,
    board: CNROMBoard,
}

impl CNROM {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, CNROMBoard::Mapper3)
    }

    pub fn new_mapper185(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, CNROMBoard::Mapper185)
    }

    fn new_with_board(
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        chr_ram: Vec<u8>,
        board: CNROMBoard,
    ) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            chr_bank: 0,
            board,
        }
    }

    fn prg_index(&self, addr: u16) -> usize {
        let mut offset = addr as usize - 0x8000;

        // 16KB PRG is mirrored into $C000-$FFFF.
        if self.prg_rom.len() == 16 * 1024 {
            offset %= 16 * 1024;
        }

        offset
    }

    fn chr_index(&self, addr: u16) -> usize {
        let offset = addr as usize & 0x1FFF;

        if self.chr_rom.is_empty() {
            // CHR-RAM boards ignore bank switching.
            return offset % self.chr_ram.len();
        }

        let bank_size = 8 * 1024;
        let num_banks = self.chr_rom.len() / bank_size;
        let bank = (self.chr_bank as usize) % num_banks.max(1);

        bank * bank_size + offset
    }

    fn chr_enabled(&self) -> bool {
        if self.board != CNROMBoard::Mapper185 || self.chr_rom.is_empty() {
            return true;
        }
        // Mapper 185 copy-protection variant: with 8 KiB CHR-ROM only bank #0 is valid.
        self.chr_bank == 0
    }
}

impl Mapper for CNROM {
    fn read_chr(&self, addr: u16) -> u8 {
        if !self.chr_enabled() {
            return 0x00;
        }

        let index = self.chr_index(addr);

        if self.chr_rom.is_empty() {
            self.chr_ram[index]
        } else {
            self.chr_rom[index]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if self.chr_rom.is_empty() {
            let index = self.chr_index(addr);
            self.chr_ram[index] = value;
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => self.prg_ram[(addr as usize - 0x6000) & 0x1FFF],
            0x8000..=0xFFFF => self.prg_rom[self.prg_index(addr)],
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match addr {
            0x6000..=0x7FFF => {
                self.prg_ram[(addr as usize - 0x6000) & 0x1FFF] = value;
            }
            0x8000..=0xFFFF => {
                self.chr_bank = value & 0x03;
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_chr_bank() {
        let prg = vec![0xEA; 16 * 1024];
        let mut chr = vec![0u8; 4 * 8 * 1024];
        chr[0] = 0x10;
        chr[8 * 1024] = 0x20;

        let mut mapper = CNROM::new(prg, chr, vec![]);
        assert_eq!(mapper.read_chr(0x0000), 0x10);

        mapper.write_prg(0x8000, 0x01);
        assert_eq!(mapper.read_chr(0x0000), 0x20);
    }

    #[test]
    fn mapper185_disables_chr_for_nonzero_bank() {
        let prg = vec![0xEA; 16 * 1024];
        let mut chr = vec![0u8; 8 * 1024];
        chr[0] = 0x33;

        let mut mapper = CNROM::new_mapper185(prg, chr, vec![]);
        assert_eq!(mapper.read_chr(0x0000), 0x33);

        mapper.write_prg(0x8000, 0x01);
        assert_eq!(mapper.read_chr(0x0000), 0x00);

        mapper.write_prg(0x8000, 0x00);
        assert_eq!(mapper.read_chr(0x0000), 0x33);
    }
}
