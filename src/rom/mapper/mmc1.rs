//
// https://www.nesdev.org/wiki/MMC1
//
use super::Mapper;
use super::Mirroring;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MMC1Board {
    Mapper1,
    Submapper5,
    Submapper6,
    Submapper7,
}

pub struct MMC1 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    board: MMC1Board,

    // MMC1 internal
    shift_register: u8,
    write_count: u8,

    // Registers
    control: u8,
    chr_bank_0: u8,
    chr_bank_1: u8,
    prg_bank: u8,
    last_prg_write_cycle: Option<u64>,
}

impl MMC1 {
    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, MMC1Board::Mapper1)
    }

    pub fn new_submapper5(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, MMC1Board::Submapper5)
    }

    pub fn new_submapper6(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, MMC1Board::Submapper6)
    }

    pub fn new_submapper7(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, MMC1Board::Submapper7)
    }

    fn new_with_board(
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        chr_ram: Vec<u8>,
        board: MMC1Board,
    ) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            board,
            shift_register: 0x10,
            write_count: 0,
            control: 0x0C, // PRG mode 3 by default
            chr_bank_0: 0,
            chr_bank_1: 0,
            prg_bank: 0,
            last_prg_write_cycle: None,
        }
    }

    fn prg_bank_mode(&self) -> u8 {
        (self.control >> 2) & 0x03
    }

    fn chr_bank_mode(&self) -> bool {
        (self.control & 0x10) != 0 // true = 4KB, false = 8KB
    }

    fn get_prg_bank(&self, addr: u16) -> usize {
        // SEROM/SHROM/SH1ROM: 32KB PRG ROM hardwired, no PRG bank switching.
        if self.board == MMC1Board::Submapper5 {
            return if addr < 0xC000 { 0 } else { 1 };
        }

        let num_banks = self.prg_rom.len() / (16 * 1024);
        let prg = (self.prg_bank & 0x0F) as usize;

        let bank = match self.prg_bank_mode() {
            0 | 1 => {
                // 32KB mode
                if addr < 0xC000 {
                    prg & !1
                } else {
                    (prg & !1) | 1
                }
            }
            2 => {
                // fix first
                if addr < 0xC000 {
                    0
                } else {
                    prg
                }
            }
            3 => {
                // fix last
                if addr < 0xC000 {
                    prg
                } else {
                    num_banks - 1
                }
            }
            _ => unreachable!(),
        };

        bank % num_banks
    }

    fn prg_ram_enabled(&self) -> bool {
        // MMC1B: bit4 of PRG bank register disables PRG-RAM when set.
        // Keep submapper-specific boards always enabled unless proven otherwise.
        if self.board == MMC1Board::Mapper1 {
            (self.prg_bank & 0x10) == 0
        } else {
            true
        }
    }
}

impl Mapper for MMC1 {
    fn read_chr(&self, addr: u16) -> u8 {
        let addr = addr as usize & 0x1FFF;

        let (bank, bank_size, offset) = if self.chr_bank_mode() {
            // 4KB mode
            if addr < 0x1000 {
                ((self.chr_bank_0 & 0x1F) as usize, 4 * 1024, addr)
            } else {
                (
                    (self.chr_bank_1 & 0x1F) as usize,
                    4 * 1024,
                    addr - 0x1000,
                )
            }
        } else {
            // 8KB mode
            (((self.chr_bank_0 & 0x1E) as usize) >> 1, 8 * 1024, addr)
        };

        if self.chr_rom.is_empty() {
            let num_banks = (self.chr_ram.len() / bank_size).max(1);
            self.chr_ram[(bank % num_banks) * bank_size + offset]
        } else {
            let num_banks = self.chr_rom.len() / bank_size;
            self.chr_rom[(bank % num_banks) * bank_size + offset]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        // Writable only if CHR RAM
        if self.chr_rom.is_empty() {
            let addr = addr as usize & 0x1FFF;
            let (bank, bank_size, offset) = if self.chr_bank_mode() {
                if addr < 0x1000 {
                    ((self.chr_bank_0 & 0x1F) as usize, 4 * 1024, addr)
                } else {
                    (
                        (self.chr_bank_1 & 0x1F) as usize,
                        4 * 1024,
                        addr - 0x1000,
                    )
                }
            } else {
                (((self.chr_bank_0 & 0x1E) as usize) >> 1, 8 * 1024, addr)
            };
            let num_banks = (self.chr_ram.len() / bank_size).max(1);
            let index = (bank % num_banks) * bank_size + offset;
            self.chr_ram[index] = value;
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled() {
                    let addr = (addr as usize - 0x6000) & 0x1FFF;
                    self.prg_ram[addr]
                } else {
                    0
                }
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
        self.write_prg_inner(addr, value, None);
    }

    fn write_prg_with_cycle(&mut self, addr: u16, value: u8, cpu_bus_cycle: u64) {
        self.write_prg_inner(addr, value, Some(cpu_bus_cycle));
    }

    fn mirroring(&self) -> Option<Mirroring> {
        // KS-7058: nametable wiring is hard-wired on board.
        // Keep ROM header mirroring instead of MMC1-controlled mirroring register.
        if self.board == MMC1Board::Submapper7 {
            return None;
        }

        match self.control & 0x03 {
            0 => Some(Mirroring::SingleScreenLower),
            1 => Some(Mirroring::SingleScreenUpper),
            2 => Some(Mirroring::Vertical),
            3 => Some(Mirroring::Horizontal),
            _ => None,
        }
    }
}

impl MMC1 {
    fn write_prg_inner(&mut self, addr: u16, value: u8, cpu_bus_cycle: Option<u64>) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled() {
                    let addr = (addr as usize - 0x6000) & 0x1FFF;
                    self.prg_ram[addr] = value;
                }
            }
            0x8000..=0xFFFF => {
                if let Some(cycle) = cpu_bus_cycle {
                    if self
                        .last_prg_write_cycle
                        .is_some_and(|prev| prev.saturating_add(1) == cycle)
                    {
                        self.last_prg_write_cycle = Some(cycle);
                        return;
                    }
                    self.last_prg_write_cycle = Some(cycle);
                } else {
                    self.last_prg_write_cycle = None;
                }

                if value & 0x80 != 0 {
                    // Reset shift register
                    self.shift_register = 0x10;
                    self.write_count = 0;
                    self.control |= 0x0C;
                } else {
                    self.shift_register =
                        (self.shift_register >> 1) | ((value & 1) << 4);
                    self.write_count += 1;

                    if self.write_count == 5 {
                        let reg = self.shift_register;

                        match addr {
                            0x8000..=0x9FFF => self.control = reg,
                            0xA000..=0xBFFF => self.chr_bank_0 = reg,
                            0xC000..=0xDFFF => self.chr_bank_1 = reg,
                            0xE000..=0xFFFF => self.prg_bank = reg,
                            _ => {}
                        }

                        self.shift_register = 0x10;
                        self.write_count = 0;
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_serial_lsb(mapper: &mut MMC1, addr: u16, value: u8) {
        for i in 0..5 {
            mapper.write_prg(addr, (value >> i) & 1);
        }
    }

    #[test]
    fn submapper5_uses_fixed_32k_prg_layout() {
        let mut prg = vec![0u8; 2 * 16 * 1024];
        prg[0] = 0x11;
        prg[16 * 1024] = 0x22;
        let mut mapper = MMC1::new_submapper5(prg, vec![0; 8 * 1024], vec![]);

        assert_eq!(mapper.read_prg(0x8000), 0x11);
        assert_eq!(mapper.read_prg(0xC000), 0x22);

        mapper.write_prg(0xE000, 0x0F);
        assert_eq!(mapper.read_prg(0x8000), 0x11);
        assert_eq!(mapper.read_prg(0xC000), 0x22);
    }

    #[test]
    fn submapper7_uses_header_mirroring() {
        let mapper = MMC1::new_submapper7(vec![0; 2 * 16 * 1024], vec![0; 8 * 1024], vec![]);
        assert_eq!(mapper.mirroring(), None);
    }

    #[test]
    fn chr_8k_mode_uses_full_8k_window() {
        let mut chr = vec![0u8; 8 * 1024];
        chr[0x0000] = 0x11;
        chr[0x1000] = 0x22;
        chr[0x1FFF] = 0x33;

        let mapper = MMC1::new(vec![0; 2 * 16 * 1024], chr, vec![]);
        assert_eq!(mapper.read_chr(0x0000), 0x11);
        assert_eq!(mapper.read_chr(0x1000), 0x22);
        assert_eq!(mapper.read_chr(0x1FFF), 0x33);
    }

    #[test]
    fn chr_4k_mode_uses_separate_banks() {
        let mut chr = vec![0u8; 8 * 4 * 1024];
        // Bank 2 @ $0000-$0FFF
        chr[2 * 4 * 1024 + 0x000] = 0x2A;
        // Bank 5 @ $1000-$1FFF
        chr[5 * 4 * 1024 + 0x123] = 0x5B;

        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], chr, vec![]);
        mapper.control = 0x10; // 4KB CHR mode
        mapper.chr_bank_0 = 2;
        mapper.chr_bank_1 = 5;

        assert_eq!(mapper.read_chr(0x0000), 0x2A);
        assert_eq!(mapper.read_chr(0x1123), 0x5B);
    }

    #[test]
    fn mapper1_prg_ram_disable_bit_blocks_access() {
        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], vec![0; 8 * 1024], vec![]);
        mapper.write_prg(0x6000, 0xA5);
        assert_eq!(mapper.read_prg(0x6000), 0xA5);

        mapper.prg_bank = 0x10;
        mapper.write_prg(0x6000, 0x5A);
        assert_eq!(mapper.read_prg(0x6000), 0x00);
    }

    #[test]
    fn consecutive_cycle_write_to_prg_register_is_ignored() {
        let mut mapper = MMC1::new(vec![0; 8 * 16 * 1024], vec![], vec![0; 8 * 1024]);

        // First write should be accepted.
        mapper.write_prg_with_cycle(0xE000, 0x01, 100);
        let after_first = mapper.shift_register;

        // Consecutive-cycle write should be ignored.
        mapper.write_prg_with_cycle(0xE000, 0x00, 101);
        assert_eq!(mapper.shift_register, after_first);

        // Non-consecutive write should be accepted again.
        mapper.write_prg_with_cycle(0xE000, 0x00, 103);
        assert_ne!(mapper.shift_register, after_first);
    }

    #[test]
    fn chr_ram_can_be_banked_in_4k_mode() {
        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], vec![], vec![0; 8 * 1024]);
        mapper.control = 0x10; // 4KB CHR mode
        mapper.chr_bank_0 = 1;
        mapper.chr_bank_1 = 2;

        mapper.write_chr(0x000A, 0xA1);
        mapper.write_chr(0x100A, 0xB2);

        assert_eq!(mapper.read_chr(0x000A), 0xA1);
        assert_eq!(mapper.read_chr(0x100A), 0xB2);
    }

    #[test]
    fn chr_ram_8k_mode_selects_all_banks() {
        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], vec![], vec![0; 32 * 1024]);
        mapper.control = 0x00; // 8KB CHR mode

        for (bank_reg, marker) in [(0u8, 0xA1u8), (2u8, 0xB2u8), (4u8, 0xC3u8), (6u8, 0xD4u8)] {
            mapper.chr_bank_0 = bank_reg;
            mapper.write_chr(0x0034, marker);
        }

        for (bank_reg, marker) in [(0u8, 0xA1u8), (2u8, 0xB2u8), (4u8, 0xC3u8), (6u8, 0xD4u8)] {
            mapper.chr_bank_0 = bank_reg;
            assert_eq!(mapper.read_chr(0x0034), marker);
        }
    }

    #[test]
    fn prg_mode_3_fixes_last_bank_at_c000() {
        let mut prg = vec![0u8; 4 * 16 * 1024];
        prg[0x0000] = 0x10;
        prg[16 * 1024] = 0x11;
        prg[2 * 16 * 1024] = 0x12;
        prg[3 * 16 * 1024] = 0x13;

        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], vec![], vec![0; 32 * 1024]);
        mapper.prg_rom = prg;
        mapper.control = 0x0C; // PRG mode 3
        mapper.prg_bank = 1;

        assert_eq!(mapper.read_prg(0x8000), 0x11);
        assert_eq!(mapper.read_prg(0xC000), 0x13);
    }

    #[test]
    fn prg_mode_2_fixes_first_bank_at_8000() {
        let mut prg = vec![0u8; 4 * 16 * 1024];
        prg[0x0000] = 0x20;
        prg[16 * 1024] = 0x21;
        prg[2 * 16 * 1024] = 0x22;
        prg[3 * 16 * 1024] = 0x23;

        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], vec![], vec![0; 32 * 1024]);
        mapper.prg_rom = prg;
        mapper.control = 0x08; // PRG mode 2
        mapper.prg_bank = 2;

        assert_eq!(mapper.read_prg(0x8000), 0x20);
        assert_eq!(mapper.read_prg(0xC000), 0x22);
    }

    #[test]
    fn prg_mode_0_uses_32k_pairs() {
        let mut prg = vec![0u8; 4 * 16 * 1024];
        prg[0x0000] = 0x30;
        prg[16 * 1024] = 0x31;
        prg[2 * 16 * 1024] = 0x32;
        prg[3 * 16 * 1024] = 0x33;

        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], vec![], vec![0; 32 * 1024]);
        mapper.prg_rom = prg;
        mapper.control = 0x00; // PRG mode 0
        mapper.prg_bank = 3;

        assert_eq!(mapper.read_prg(0x8000), 0x32);
        assert_eq!(mapper.read_prg(0xC000), 0x33);
    }

    #[test]
    fn serial_writes_program_target_register_by_address_range() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);

        write_serial_lsb(&mut mapper, 0x8000, 0x1B);
        write_serial_lsb(&mut mapper, 0xA000, 0x15);
        write_serial_lsb(&mut mapper, 0xC000, 0x0E);
        write_serial_lsb(&mut mapper, 0xE000, 0x07);

        assert_eq!(mapper.control, 0x1B);
        assert_eq!(mapper.chr_bank_0, 0x15);
        assert_eq!(mapper.chr_bank_1, 0x0E);
        assert_eq!(mapper.prg_bank, 0x07);
    }

    #[test]
    fn reset_write_reinitializes_shift_register_and_forces_prg_mode_3_bits() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);

        mapper.control = 0x00;
        mapper.write_prg(0xE000, 0x01);
        mapper.write_prg(0xE000, 0x00);
        assert_ne!(mapper.shift_register, 0x10);
        assert_eq!(mapper.write_count, 2);

        mapper.write_prg(0xE000, 0x80);
        assert_eq!(mapper.shift_register, 0x10);
        assert_eq!(mapper.write_count, 0);
        assert_eq!(mapper.control & 0x0C, 0x0C);
    }

    #[test]
    fn prg_ram_writes_are_not_filtered_by_consecutive_cycle_quirk() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);

        mapper.write_prg_with_cycle(0x6000, 0xAA, 100);
        mapper.write_prg_with_cycle(0x6000, 0x55, 101);

        assert_eq!(mapper.read_prg(0x6000), 0x55);
    }

    #[test]
    fn mirroring_follows_control_bits_for_mapper1() {
        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], vec![0; 8 * 1024], vec![]);

        mapper.control = 0x00;
        assert_eq!(mapper.mirroring(), Some(Mirroring::SingleScreenLower));
        mapper.control = 0x01;
        assert_eq!(mapper.mirroring(), Some(Mirroring::SingleScreenUpper));
        mapper.control = 0x02;
        assert_eq!(mapper.mirroring(), Some(Mirroring::Vertical));
        mapper.control = 0x03;
        assert_eq!(mapper.mirroring(), Some(Mirroring::Horizontal));
    }

    #[test]
    fn register_write_applies_only_on_fifth_serial_write() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);
        mapper.prg_bank = 0x00;

        mapper.write_prg(0xE000, 1);
        mapper.write_prg(0xE000, 1);
        mapper.write_prg(0xE000, 1);
        mapper.write_prg(0xE000, 0);
        assert_eq!(mapper.prg_bank, 0x00);

        mapper.write_prg(0xE000, 0);
        assert_eq!(mapper.prg_bank, 0x07);
        assert_eq!(mapper.write_count, 0);
        assert_eq!(mapper.shift_register, 0x10);
    }

    #[test]
    fn fifth_write_address_selects_target_register() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);
        mapper.control = 0x00;
        mapper.prg_bank = 0x00;

        mapper.write_prg(0x8000, 1);
        mapper.write_prg(0x8000, 0);
        mapper.write_prg(0x8000, 1);
        mapper.write_prg(0x8000, 0);
        mapper.write_prg(0xE000, 1);

        assert_eq!(mapper.control, 0x00);
        assert_eq!(mapper.prg_bank, 0x15);
    }

    #[test]
    fn chr_rom_ignores_write_attempts() {
        let mut chr = vec![0u8; 8 * 1024];
        chr[0x0123] = 0xAB;

        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], chr, vec![]);
        assert_eq!(mapper.read_chr(0x0123), 0xAB);
        mapper.write_chr(0x0123, 0x55);
        assert_eq!(mapper.read_chr(0x0123), 0xAB);
    }

    #[test]
    fn submapper6_does_not_disable_prg_ram_via_prg_bank_bit4() {
        let mut mapper = MMC1::new_submapper6(vec![0; 2 * 16 * 1024], vec![0; 8 * 1024], vec![]);
        mapper.prg_bank = 0x10;
        mapper.write_prg(0x6000, 0x5A);
        assert_eq!(mapper.read_prg(0x6000), 0x5A);
    }

    #[test]
    fn consecutive_cycle_filter_ignores_writes_even_across_register_ranges() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);

        mapper.write_prg_with_cycle(0x8000, 1, 100);
        let before = mapper.shift_register;
        mapper.write_prg_with_cycle(0xA000, 1, 101);
        assert_eq!(mapper.shift_register, before);
    }

    #[test]
    fn prg_ram_write_between_prg_writes_does_not_trigger_false_consecutive_ignore() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);

        mapper.write_prg_with_cycle(0xE000, 1, 100);
        let after_first = mapper.shift_register;

        mapper.write_prg_with_cycle(0x6000, 0xAA, 101);

        mapper.write_prg_with_cycle(0xE000, 1, 102);
        assert_ne!(mapper.shift_register, after_first);
    }

    #[test]
    fn prg_bank_index_wraps_when_register_exceeds_rom_bank_count() {
        let mut prg = vec![0u8; 2 * 16 * 1024];
        prg[0x0000] = 0x40;
        prg[16 * 1024] = 0x41;

        let mut mapper = MMC1::new(prg, vec![0; 8 * 1024], vec![]);
        mapper.control = 0x0C; // mode 3
        mapper.prg_bank = 0x0F; // exceeds available 16KB banks

        assert_eq!(mapper.read_prg(0x8000), 0x41);
        assert_eq!(mapper.read_prg(0xC000), 0x41);
    }

    #[test]
    fn chr_rom_bank_index_wraps_in_4k_mode() {
        let mut chr = vec![0u8; 2 * 4 * 1024];
        chr[0x0000] = 0x50;
        chr[4 * 1024] = 0x51;

        let mut mapper = MMC1::new(vec![0; 2 * 16 * 1024], chr, vec![]);
        mapper.control = 0x10; // 4KB mode
        mapper.chr_bank_0 = 5; // wraps to bank 1
        mapper.chr_bank_1 = 4; // wraps to bank 0

        assert_eq!(mapper.read_chr(0x0000), 0x51);
        assert_eq!(mapper.read_chr(0x1000), 0x50);
    }

    #[test]
    fn reset_write_does_not_clear_already_programmed_registers() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);
        mapper.prg_bank = 0x09;
        mapper.chr_bank_0 = 0x12;
        mapper.chr_bank_1 = 0x07;

        mapper.write_prg(0x8000, 0x80);

        assert_eq!(mapper.prg_bank, 0x09);
        assert_eq!(mapper.chr_bank_0, 0x12);
        assert_eq!(mapper.chr_bank_1, 0x07);
    }

    #[test]
    fn manual_write_path_clears_last_cycle_filter_state() {
        let mut mapper = MMC1::new(vec![0; 4 * 16 * 1024], vec![], vec![0; 8 * 1024]);

        mapper.write_prg_with_cycle(0xE000, 1, 100);
        assert_eq!(mapper.last_prg_write_cycle, Some(100));

        mapper.write_prg(0xE000, 0);
        assert_eq!(mapper.last_prg_write_cycle, None);
    }
}
