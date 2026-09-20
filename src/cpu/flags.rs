use super::cpu::Cpu;
use super::cpu::CpuFlags;

pub trait FlagOps {
    fn update_zero_flg(&mut self, operand: u8);
    fn update_neg_flg(&mut self, operand: u8);
    fn update_ovfl_flg(&mut self, operand: u8);
    fn update_zero_and_neg_flags(&mut self, operand: u8);
}

impl FlagOps for Cpu {
    fn update_zero_flg(&mut self, operand: u8) {
        self.flags.set(CpuFlags::ZERO, operand == 0);
    }

    fn update_neg_flg(&mut self, operand: u8) {
        self.flags.set(CpuFlags::NEGATIV, operand & 0b10000000 > 0);
    }

    fn update_ovfl_flg(&mut self,operand: u8) {
        self.flags.set(CpuFlags::OVERFLOW, operand & 0b01000000 > 0);
    }

    fn update_zero_and_neg_flags(&mut self, operand: u8) {
        self.update_zero_flg(operand);
        self.update_neg_flg(operand);
    }
}
