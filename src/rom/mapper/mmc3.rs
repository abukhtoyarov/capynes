//
// https://www.nesdev.org/wiki/MMC3
// Mapper 4 (TxROM)
//
use super::Mapper;
use super::Mirroring;
use super::Mmc3IrqRevision;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MMC3Board {
    Mapper4,
    Mapper118,
    Mapper119,
}

pub struct MMC3 {
    prg_rom: Vec<u8>,
    prg_ram: Vec<u8>,
    chr_rom: Vec<u8>,
    chr_ram: Vec<u8>,
    board: MMC3Board,

    bank_select: u8,
    bank_registers: [u8; 8],

    mirroring: Option<Mirroring>,
    prg_ram_enabled: bool,
    prg_ram_write_protect: bool,

    irq_latch: u8,
    irq_counter: u8,
    irq_reload: bool,
    irq_enabled: bool,
    irq_pending: bool,
    irq_visible_delay_polls: u8,
    irq_revision: Mmc3IrqRevision,
    a12_last_state: bool,
    a12_low_cycle: u64,
}

impl MMC3 {
    fn update_a12_state(&mut self, a12: bool) {
        self.a12_last_state = a12;
    }

    pub fn new(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, MMC3Board::Mapper4)
    }

    pub fn new_mapper118(prg_rom: Vec<u8>, chr_rom: Vec<u8>, chr_ram: Vec<u8>) -> Self {
        Self::new_with_board(prg_rom, chr_rom, chr_ram, MMC3Board::Mapper118)
    }

    pub fn new_mapper119(prg_rom: Vec<u8>, chr_rom: Vec<u8>, mut chr_ram: Vec<u8>) -> Self {
        // TQROM has both CHR ROM and CHR RAM; keep 8KB CHR RAM available.
        if chr_ram.is_empty() {
            chr_ram = vec![0; 8 * 1024];
        }
        Self::new_with_board(prg_rom, chr_rom, chr_ram, MMC3Board::Mapper119)
    }

    fn new_with_board(
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        chr_ram: Vec<u8>,
        board: MMC3Board,
    ) -> Self {
        Self {
            prg_rom,
            prg_ram: vec![0; 8 * 1024],
            chr_rom,
            chr_ram,
            board,
            bank_select: 0,
            bank_registers: [0; 8],
            mirroring: None,
            prg_ram_enabled: true,
            prg_ram_write_protect: false,
            irq_latch: 0,
            irq_counter: 0,
            irq_reload: false,
            irq_enabled: false,
            irq_pending: false,
            irq_visible_delay_polls: 0,
            irq_revision: Mmc3IrqRevision::RevB,
            a12_last_state: false,
            a12_low_cycle: 0,
        }
    }

    fn prg_bank_mode(&self) -> bool {
        (self.bank_select & 0x40) != 0
    }

    fn chr_inversion(&self) -> bool {
        (self.bank_select & 0x80) != 0
    }

    fn selected_reg(&self) -> usize {
        (self.bank_select & 0x07) as usize
    }

    fn prg_8k_banks(&self) -> usize {
        (self.prg_rom.len() / (8 * 1024)).max(1)
    }

    fn read_prg_bank(&self, bank: usize, offset: usize) -> u8 {
        let banks = self.prg_8k_banks();
        let bank = bank % banks;
        self.prg_rom[bank * 8 * 1024 + offset]
    }

    fn prg_bank_number(&self, slot: u16) -> usize {
        let last = self.prg_8k_banks() - 1;
        let second_last = last.saturating_sub(1);

        let r6 = (self.bank_registers[6] as usize) % self.prg_8k_banks();
        let r7 = (self.bank_registers[7] as usize) % self.prg_8k_banks();

        if self.prg_bank_mode() {
            match slot {
                0 => second_last,
                1 => r7,
                2 => r6,
                3 => last,
                _ => last,
            }
        } else {
            match slot {
                0 => r6,
                1 => r7,
                2 => second_last,
                3 => last,
                _ => last,
            }
        }
    }

    fn chr_1k_banks_rom(&self) -> usize {
        (self.chr_rom.len() / 1024).max(1)
    }

    fn chr_1k_banks_ram(&self) -> usize {
        (self.chr_ram.len() / 1024).max(1)
    }

    fn chr_raw_bank(&self, segment: u16) -> u8 {
        let r0 = self.bank_registers[0] & 0xFE;
        let r1 = self.bank_registers[1] & 0xFE;
        let r2 = self.bank_registers[2];
        let r3 = self.bank_registers[3];
        let r4 = self.bank_registers[4];
        let r5 = self.bank_registers[5];

        if !self.chr_inversion() {
            match segment {
                0 => r0,
                1 => r0 + 1,
                2 => r1,
                3 => r1 + 1,
                4 => r2,
                5 => r3,
                6 => r4,
                7 => r5,
                _ => 0,
            }
        } else {
            match segment {
                0 => r2,
                1 => r3,
                2 => r4,
                3 => r5,
                4 => r0,
                5 => r0 + 1,
                6 => r1,
                7 => r1 + 1,
                _ => 0,
            }
        }
    }

    fn chr_mapping(&self, addr: u16) -> (bool, usize) {
        let addr = addr as usize & 0x1FFF;
        let segment = (addr / 0x0400) as u16;
        let offset = addr & 0x03FF;
        let raw_bank = self.chr_raw_bank(segment);

        if self.board == MMC3Board::Mapper119 {
            let use_ram = (raw_bank & 0x40) != 0;
            if use_ram {
                let bank = (raw_bank as usize & 0x3F) % self.chr_1k_banks_ram();
                return (true, bank * 1024 + offset);
            }
            let bank = (raw_bank as usize & 0x3F) % self.chr_1k_banks_rom();
            return (false, bank * 1024 + offset);
        }

        if self.chr_rom.is_empty() {
            let bank = (raw_bank as usize) % self.chr_1k_banks_ram();
            return (true, bank * 1024 + offset);
        }

        let bank = (raw_bank as usize) % self.chr_1k_banks_rom();
        (false, bank * 1024 + offset)
    }

    fn clock_irq_counter(&mut self) {
        let reloading = self.irq_counter == 0 || self.irq_reload;
        let reloading_due_to_clear = self.irq_reload;

        if reloading {
            self.irq_counter = self.irq_latch;
            self.irq_reload = false;
        } else {
            self.irq_counter = self.irq_counter.wrapping_sub(1);
        }

        if self.irq_counter == 0 && self.irq_enabled {
            let should_assert = match self.irq_revision {
                Mmc3IrqRevision::RevB => true,
                Mmc3IrqRevision::RevA => !reloading || reloading_due_to_clear,
            };

            if should_assert {
                self.irq_pending = true;
                self.irq_visible_delay_polls = 3;
            }
        }
    }
}

impl Mapper for MMC3 {
    fn read_chr(&self, addr: u16) -> u8 {
        let (use_ram, index) = self.chr_mapping(addr);

        if use_ram {
            self.chr_ram[index]
        } else {
            self.chr_rom[index]
        }
    }

    fn write_chr(&mut self, addr: u16, value: u8) {
        let (use_ram, index) = self.chr_mapping(addr);
        if use_ram {
            self.chr_ram[index] = value;
        }
    }

    fn read_prg(&self, addr: u16) -> u8 {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled {
                    self.prg_ram[(addr as usize - 0x6000) & 0x1FFF]
                } else {
                    0
                }
            }
            0x8000..=0x9FFF => {
                let offset = (addr as usize - 0x8000) & 0x1FFF;
                self.read_prg_bank(self.prg_bank_number(0), offset)
            }
            0xA000..=0xBFFF => {
                let offset = (addr as usize - 0xA000) & 0x1FFF;
                self.read_prg_bank(self.prg_bank_number(1), offset)
            }
            0xC000..=0xDFFF => {
                let offset = (addr as usize - 0xC000) & 0x1FFF;
                self.read_prg_bank(self.prg_bank_number(2), offset)
            }
            0xE000..=0xFFFF => {
                let offset = (addr as usize - 0xE000) & 0x1FFF;
                self.read_prg_bank(self.prg_bank_number(3), offset)
            }
            _ => 0,
        }
    }

    fn write_prg(&mut self, addr: u16, value: u8) {
        match addr {
            0x6000..=0x7FFF => {
                if self.prg_ram_enabled && !self.prg_ram_write_protect {
                    self.prg_ram[(addr as usize - 0x6000) & 0x1FFF] = value;
                }
            }
            0x8000..=0x9FFE if addr & 1 == 0 => {
                self.bank_select = value;
            }
            0x8001..=0x9FFF if addr & 1 == 1 => {
                let reg = self.selected_reg();
                self.bank_registers[reg] = value;
            }
            0xA000..=0xBFFE if addr & 1 == 0 => {
                // TxSROM (mapper 118) has custom nametable wiring and does not use MMC3
                // H/V mirroring control at $A000.
                if self.board != MMC3Board::Mapper118 {
                    self.mirroring = Some(if value & 0x01 == 0 {
                        Mirroring::Vertical
                    } else {
                        Mirroring::Horizontal
                    });
                }
            }
            0xA001..=0xBFFF if addr & 1 == 1 => {
                self.prg_ram_enabled = value & 0x80 != 0;
                self.prg_ram_write_protect = value & 0x40 != 0;
            }
            0xC000..=0xDFFE if addr & 1 == 0 => {
                self.irq_latch = value;
            }
            0xC001..=0xDFFF if addr & 1 == 1 => {
                self.irq_counter = 0;
                self.irq_reload = true;
            }
            0xE000..=0xFFFE if addr & 1 == 0 => {
                self.irq_enabled = false;
                self.irq_pending = false;
                self.irq_visible_delay_polls = 0;
            }
            0xE001..=0xFFFF if addr & 1 == 1 => {
                self.irq_enabled = true;
            }
            _ => {}
        }
    }

    fn mirroring(&self) -> Option<Mirroring> {
        self.mirroring
    }

    fn on_ppu_addr(&mut self, addr: u16, ppu_cycle: u64) {
        let a12 = (addr & 0x1000) != 0;
        if !a12 {
            if self.a12_last_state {
                self.a12_low_cycle = ppu_cycle;
            }
            self.update_a12_state(false);
            return;
        }

        if !self.a12_last_state {
            let low_period = ppu_cycle.saturating_sub(self.a12_low_cycle);
            if low_period >= 8 {
                self.clock_irq_counter();
            }
        }
        self.update_a12_state(true);
    }

    fn on_cpu_ppu_addr(&mut self, addr: u16, _ppu_cycle: u64) {
        let a12 = (addr & 0x1000) != 0;
        if a12 && !self.a12_last_state {
            self.clock_irq_counter();
        }
        self.update_a12_state(a12);
    }

    fn poll_irq(&mut self) -> bool {
        if self.irq_pending && self.irq_visible_delay_polls > 0 {
            self.irq_visible_delay_polls -= 1;
            return false;
        }
        self.irq_pending
    }

    fn map_nametable_addr(&self, addr: u16) -> Option<usize> {
        if self.board != MMC3Board::Mapper118 {
            return None;
        }

        let addr = (addr - 0x2000) % 0x1000;
        let table = (addr / 0x400) as usize;
        let offset = (addr % 0x400) as usize;

        // TxSROM/TKSROM uses CHR bank register bit 7 to select CIRAM page per nametable.
        let source_reg = match table {
            0 => 0,
            1 => 1,
            2 => 2,
            _ => 3,
        };
        let ciram_page = ((self.bank_registers[source_reg] >> 7) & 0x01) as usize;
        Some(ciram_page * 0x400 + offset)
    }

    fn set_mmc3_irq_revision(&mut self, revision: Mmc3IrqRevision) {
        self.irq_revision = revision;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prg_bank_switching_mode_0() {
        let mut prg = vec![0u8; 8 * 8 * 1024];
        for bank in 0..8 {
            prg[bank * 8 * 1024] = bank as u8;
        }

        let mut mapper = MMC3::new(prg, vec![0; 8 * 1024], vec![]);
        mapper.write_prg(0x8000, 0x06);
        mapper.write_prg(0x8001, 0x03);

        assert_eq!(mapper.read_prg(0x8000), 3);
        assert_eq!(mapper.read_prg(0xC000), 6);
        assert_eq!(mapper.read_prg(0xE000), 7);
    }

    #[test]
    fn chr_bank_switching() {
        let prg = vec![0xEA; 4 * 16 * 1024];
        let mut chr = vec![0u8; 16 * 1024];
        chr[0] = 0x11;
        chr[4 * 1024] = 0x22;

        let mut mapper = MMC3::new(prg, chr, vec![]);
        mapper.write_prg(0x8000, 0x00);
        mapper.write_prg(0x8001, 0x04);

        assert_eq!(mapper.read_chr(0x0000), 0x22);
    }

    #[test]
    fn mapper119_uses_chr_ram_when_bit6_is_set() {
        let prg = vec![0xEA; 4 * 16 * 1024];
        let mut chr_rom = vec![0u8; 8 * 1024];
        chr_rom[0] = 0xAA;

        let mut mapper = MMC3::new_mapper119(prg, chr_rom, vec![]);
        assert_eq!(mapper.read_chr(0x0000), 0xAA);

        mapper.write_prg(0x8000, 0x00); // select R0
        mapper.write_prg(0x8001, 0x40); // RAM bank 0 in $0000-$07FF
        mapper.write_chr(0x0000, 0x55);
        assert_eq!(mapper.read_chr(0x0000), 0x55);
    }

    #[test]
    fn irq_raises_on_filtered_a12_rising_edges() {
        let prg = vec![0xEA; 4 * 16 * 1024];
        let mut mapper = MMC3::new(prg, vec![0; 8 * 1024], vec![]);

        mapper.write_prg(0xC000, 0x01); // latch
        mapper.write_prg(0xE001, 0x00); // enable

        mapper.on_ppu_addr(0x0000, 0);
        mapper.on_ppu_addr(0x1000, 10); // first clock -> counter=1
        assert!(!mapper.poll_irq());

        mapper.on_ppu_addr(0x0000, 20);
        mapper.on_ppu_addr(0x1000, 30); // second clock -> counter=0, IRQ
        assert!(!mapper.poll_irq());
        assert!(!mapper.poll_irq());
        assert!(!mapper.poll_irq());
        assert!(mapper.poll_irq());

        mapper.write_prg(0xE000, 0x00); // ack/disable
        assert!(!mapper.poll_irq());
    }

    #[test]
    fn mapper118_maps_nametables_per_register_bit7() {
        let prg = vec![0xEA; 4 * 16 * 1024];
        let mut mapper = MMC3::new_mapper118(prg, vec![0; 8 * 1024], vec![]);

        mapper.write_prg(0x8000, 0x00);
        mapper.write_prg(0x8001, 0x80); // R0 bit7=1 => NT0 upper
        mapper.write_prg(0x8000, 0x01);
        mapper.write_prg(0x8001, 0x00); // R1 bit7=0 => NT1 lower

        assert_eq!(mapper.map_nametable_addr(0x2000), Some(0x400));
        assert_eq!(mapper.map_nametable_addr(0x2400), Some(0x000));
    }
}
