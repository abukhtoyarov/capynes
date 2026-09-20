pub mod nrom;   // Mapper 0
pub mod mmc1;   // Mapper 1
pub mod unrom;  // Mapper 2
pub mod cnrom;  // Mapper 3
pub mod mmc3;   // Mapper 4
pub mod mmc5;   // Mapper 5
pub mod color_dreams_11; // Mapper 11
pub mod action53_28; // Mapper 28 (minimal)
pub mod vrc2a_22; // Mapper 22
pub mod vrc2b_23; // Mapper 23
pub mod bnrom_34; // Mapper 34 (BNROM)
pub mod axrom;  // Mapper 7
use super::info::Mirroring;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mmc3IrqRevision {
    RevA,
    RevB,
}

pub trait Mapper {
    fn read_chr(&self, addr: u16) -> u8;
    fn write_chr(&mut self, addr: u16, value: u8);

    fn read_prg(&self, addr: u16) -> u8;
    fn write_prg(&mut self, addr: u16, value: u8);
    fn write_prg_with_cycle(&mut self, addr: u16, value: u8, _cpu_bus_cycle: u64) {
        self.write_prg(addr, value);
    }
    fn mirroring(&self) -> Option<Mirroring>;

    fn on_ppu_addr(&mut self, _addr: u16, _ppu_cycle: u64) {}
    fn on_cpu_ppu_addr(&mut self, _addr: u16, _ppu_cycle: u64) {}
    fn poll_irq(&mut self) -> bool { false }
    fn read_nametable(&self, _addr: u16) -> Option<u8> { None }
    fn write_nametable(&mut self, _addr: u16, _value: u8) -> bool { false }
    fn map_nametable_addr(&self, _addr: u16) -> Option<usize> { None }
    fn set_mmc3_irq_revision(&mut self, _revision: Mmc3IrqRevision) {}
}
