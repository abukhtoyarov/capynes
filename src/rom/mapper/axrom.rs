//
// https://www.nesdev.org/wiki/AxROM
// Mapper 7
//
use super::Mapper;
use super::Mirroring;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AXROMBoard {
    Mapper7,
    Submapper1NoBusConflicts,
    Submapper2BusConflicts,
}

pub struct AXROM {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    board: AXROMBoard,

    prg_bank: u8,
    single_screen_upper: bool,
}

impl AXROM {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, AXROMBoard::Mapper7)
    }

    pub fn new_submapper1(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, AXROMBoard::Submapper1NoBusConflicts)
    }

    pub fn new_submapper2(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, AXROMBoard::Submapper2BusConflicts)
    }

    fn new_with_board(
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        chr_ram: Vec<u8>,
        board: AXROMBoard,
    ) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            board,
            prg_bank: 0,
            single_screen_upper: false,
        }
    }

    fn prg_index(&self, addr: u16) -> usize {
        if self.prg_rom.is_empty() {
            return 0;
        }

        let bank_size = 32 * 1024;
        let num_banks = (self.prg_rom.len() / bank_size).max(1);
        let bank = (self.prg_bank as usize) % num_banks;
        let offset = addr as usize & 0x7FFF;
        let bank_base = (bank * bank_size) % self.prg_rom.len();
        (bank_base + offset) % self.prg_rom.len()
    }

    fn bus_conflicts(&self) -> bool {
        self.board == AXROMBoard::Submapper2BusConflicts
    }
}

impl Mapper for AXROM {
    fn read_chr(&self, addr: u16) -> u8 {
        let index = addr as usize & 0x1FFF;
        if self.chr_rom.is_empty() {
            self.chr_ram[index]
        } else {
            self.chr_rom[index]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        if self.chr_rom.is_empty() {
            let index = addr as usize & 0x1FFF;
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
                let effective_value = if self.bus_conflicts() {
                    value & self.read_prg(addr)
                } else {
                    value
                };
                self.prg_bank = effective_value & 0x07;
                self.single_screen_upper = (effective_value & 0x10) != 0;
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        if self.single_screen_upper {
            Some(Mirroring::SingleScreenUpper)
        } else {
            Some(Mirroring::SingleScreenLower)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switches_32k_prg_bank() {
        let mut prg = vec![0u8; 2 * 32 * 1024];
        prg[0] = 0x11;
        prg[32 * 1024] = 0x22;

        let mut mapper = AXROM::new(prg, vec![], vec![0; 8 * 1024]);
        assert_eq!(mapper.read_prg(0x8000), 0x11);

        mapper.write_prg(0x8000, 0x01);
        assert_eq!(mapper.read_prg(0x8000), 0x22);
    }

    #[test]
    fn switches_single_screen_mirroring() {
        let mut mapper = AXROM::new(vec![0; 32 * 1024], vec![], vec![0; 8 * 1024]);

        assert_eq!(mapper.mirroring(), Some(Mirroring::SingleScreenLower));

        mapper.write_prg(0x8000, 0x10);
        assert_eq!(mapper.mirroring(), Some(Mirroring::SingleScreenUpper));
    }

    #[test]
    fn writes_chr_ram_when_no_chr_rom() {
        let mut mapper = AXROM::new(vec![0; 32 * 1024], vec![], vec![0; 8 * 1024]);
        mapper.write_chr(0x0012, 0xAB);
        assert_eq!(mapper.read_chr(0x0012), 0xAB);
    }

    #[test]
    fn submapper2_applies_bus_conflicts() {
        // Current bank 0 at $8000 contains 0x00, so write value is ANDed to 0x00.
        let mut prg = vec![0u8; 2 * 32 * 1024];
        prg[0] = 0x00;
        prg[32 * 1024] = 0x22;

        let mut mapper = AXROM::new_submapper2(prg, vec![], vec![0; 8 * 1024]);
        mapper.write_prg(0x8000, 0x01);
        assert_eq!(mapper.read_prg(0x8000), 0x00);
    }

    #[test]
    fn submapper1_ignores_bus_conflicts() {
        let mut prg = vec![0u8; 2 * 32 * 1024];
        prg[0] = 0x00;
        prg[32 * 1024] = 0x22;

        let mut mapper = AXROM::new_submapper1(prg, vec![], vec![0; 8 * 1024]);
        mapper.write_prg(0x8000, 0x01);
        assert_eq!(mapper.read_prg(0x8000), 0x22);
    }

    #[test]
    fn reads_and_writes_prg_ram_window() {
        let mut mapper = AXROM::new(vec![0; 32 * 1024], vec![], vec![0; 8 * 1024]);
        mapper.write_prg(0x6000, 0x5A);
        assert_eq!(mapper.read_prg(0x6000), 0x5A);
    }

    #[test]
    fn prg_index_wraps_safely_with_16k_prg_images() {
        let mut prg = vec![0u8; 16 * 1024];
        prg[0] = 0x11;
        prg[0x3FFF] = 0x22;
        let mapper = AXROM::new(prg, vec![], vec![0; 8 * 1024]);

        assert_eq!(mapper.read_prg(0x8000), 0x11);
        assert_eq!(mapper.read_prg(0xBFFF), 0x22);
        assert_eq!(mapper.read_prg(0xC000), 0x11);
        assert_eq!(mapper.read_prg(0xFFFF), 0x22);
    }
}
