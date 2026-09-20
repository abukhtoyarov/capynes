use crate::cpu::cpu::Cpu;

pub trait MemIO: Sized {
    fn read(cpu: &Cpu, addr: u16) -> Self;
    fn write(self, cpu: &mut Cpu, addr: u16);
}

impl MemIO for u8 {
    fn read(cpu: &Cpu, addr: u16) -> Self {
        cpu.bus.read(addr)
    }

    fn write(self, cpu: &mut Cpu, addr: u16) {
        cpu.bus.write(addr, self)
    }
}

impl MemIO for u16 {
    fn read(cpu: &Cpu, addr: u16) -> Self {
        let l = cpu.bus.read(addr) as u16; 
        let h = cpu.bus.read(addr + 1) as u16;
        (h << 8) | l
    }

    fn write(self, cpu: &mut Cpu, addr: u16) {
        cpu.bus.write(addr, (self & 0xFF) as u8);
        cpu.bus.write(addr + 1, (self >> 8) as u8);
    }
}
