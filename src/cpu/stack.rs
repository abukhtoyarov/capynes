use super::cpu::Cpu;

pub trait StackPush {
    fn push(self, cpu: &mut Cpu);
}

pub trait StackPop: Sized {
    fn pop(cpu: &mut Cpu) -> Self;
}

pub trait StackOps {
    fn push<T: StackPush>(&mut self, value: T);
    fn pop<T: StackPop>(&mut self) -> T;
    fn stack_addr(&self) -> u16;
    #[allow(dead_code)] fn stack_size(&self) -> u8;
}

impl StackOps for Cpu {
    fn push<T: StackPush>(&mut self, value: T) {
        value.push(self);
    }

    fn pop<T: StackPop>(&mut self) -> T {
        T::pop(self)
    }

    fn stack_addr(&self) -> u16 { 0x100 }
    fn stack_size(&self) -> u8 { 0xff }
}

impl StackPush for u8 {
    fn push(self, cpu: &mut Cpu) {
        cpu.write(cpu.stack_addr() + cpu.sp as u16, self);
        cpu.sp = cpu.sp.wrapping_sub(1);
    }
}

impl StackPop for u8 {
    fn pop(cpu: &mut Cpu) -> Self {
        cpu.sp = cpu.sp.wrapping_add(1);
        cpu.read::<u8>(cpu.stack_addr() + cpu.sp as u16)
    }
}

impl StackPush for u16 {
    fn push(self, cpu: &mut Cpu) {
        ((self >> 8) as u8).push(cpu);
        ((self & 0xFF) as u8).push(cpu);
    }
}

impl StackPop for u16 {
    fn pop(cpu: &mut Cpu) -> Self {
        let lo = u8::pop(cpu) as u16;
        let hi = u8::pop(cpu) as u16;
        (hi << 8) | lo
    }
}
