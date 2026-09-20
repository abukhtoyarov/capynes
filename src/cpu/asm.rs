use crate::cpu::cpu::{ Cpu, CpuFlags, BRK_INT };
use crate::cpu::op::{ Operand, OperandContext, OperandAddress };
use super::stack::StackOps;
use super::flags::FlagOps;
use std::fmt;

#[derive(Clone, PartialEq, Debug)]
pub enum Addressing {
    Implied,
    Accumulator,
    Immediate,
    Relative,
    ZeroPage,
    ZeroPageX,
    ZeroPageY,
    Absolute,
    AbsoluteXPageCrossed,
    AbsoluteX,
    AbsoluteYPageCrossed,
    AbsoluteY,
    Indirect,
    IndirectX,
    IndirectY,
    IndirectYPageCrossed,
}

#[derive(Clone)]
pub struct Instruction {
    pub op: u8,
    pub cmd: &'static str,
    pub len: u8,
    pub cycles: u8,
    pub mode: Addressing,
    pub invoke: fn(cpu: &mut Cpu, operand: &mut OperandContext),
}

macro_rules! instr {
    ($op:expr, $cmd:expr, $len:expr, $cycles:expr, $mode:expr, $invoke:expr) => {
        Instruction { op: $op, cmd: $cmd, len: $len, cycles: $cycles, mode: $mode, invoke: $invoke }
    };
}

macro_rules! instructions {
    ($( $op:expr => $instr:expr ),* $(,)?) => {{
        let mut table = [
            const { 
                Instruction { op: 0, cmd: "TBD", len: 1, cycles: 2, mode: Addressing::Implied, invoke: tbd } 
            }; 0x100];
        $(
            table[$op] = $instr;
        )*
        table
    }};
}

pub static INSTRUCTIONS: [Instruction; 0x100] = instructions! {
    0x00 => instr!(0x00, "BRK", 1, 7, Addressing::Implied, brk),
    0x01 => instr!(0x01, "ORA", 2, 6, Addressing::IndirectX, ora),
    0x02 => instr!(0x02, "*KIL", 1, 0, Addressing::Implied, kil),
    0x03 => instr!(0x03, "*SLO", 2, 8, Addressing::IndirectX, slo),
    0x04 => instr!(0x04, "*NOP", 2, 3, Addressing::ZeroPage, nop),
    0x05 => instr!(0x05, "ORA", 2, 3, Addressing::ZeroPage, ora),
    0x06 => instr!(0x06, "ASL", 2, 5, Addressing::ZeroPage, asl),
    0x07 => instr!(0x07, "*SLO", 2, 5, Addressing::ZeroPage, slo),
    0x08 => instr!(0x08, "PHP", 1, 3, Addressing::Implied, php),
    0x09 => instr!(0x09, "ORA", 2, 2, Addressing::Immediate, ora),
    0x0a => instr!(0x0a, "ASL", 1, 2, Addressing::Accumulator, asl),
    0x0b => instr!(0x0b, "*ANC", 2, 2, Addressing::Immediate, anc),
    0x0c => instr!(0x0c, "*NOP", 3, 4, Addressing::Absolute, nop),
    0x0d => instr!(0x0d, "ORA", 3, 4, Addressing::Absolute, ora),
    0x0e => instr!(0x0e, "ASL", 3, 6, Addressing::Absolute, asl),
    0x0f => instr!(0x0f, "*SLO", 3, 6, Addressing::Absolute, slo),
    0x10 => instr!(0x10, "BPL", 2, 2, Addressing::Relative, bpl),
    0x11 => instr!(0x11, "ORA", 2, 5, Addressing::IndirectYPageCrossed, ora),
    0x12 => instr!(0x12, "*KIL", 1, 0, Addressing::Implied, kil),
    0x13 => instr!(0x13, "*SLO", 2, 8, Addressing::IndirectY, slo),
    0x14 => instr!(0x14, "*NOP", 2, 4, Addressing::ZeroPageX, nop),
    0x15 => instr!(0x15, "ORA", 2, 4, Addressing::ZeroPageX, ora),
    0x16 => instr!(0x16, "ASL", 2, 6, Addressing::ZeroPageX, asl),
    0x17 => instr!(0x17, "*SLO", 2, 6, Addressing::ZeroPageX, slo),
    0x18 => instr!(0x18, "CLC", 1, 2, Addressing::Implied, clc),
    0x19 => instr!(0x19, "ORA", 3, 4, Addressing::AbsoluteYPageCrossed, ora),
    0x1a => instr!(0x1a, "*NOP", 1, 2, Addressing::Implied, nop),
    0x1b => instr!(0x1b, "*SLO", 3, 7, Addressing::AbsoluteY, slo),
    0x1c => instr!(0x1c, "*NOP", 3, 4, Addressing::AbsoluteXPageCrossed, nop),
    0x1d => instr!(0x1d, "ORA", 3, 4, Addressing::AbsoluteXPageCrossed, ora),
    0x1e => instr!(0x1e, "ASL", 3, 7, Addressing::AbsoluteX, asl),
    0x1f => instr!(0x1f, "*SLO", 3, 7, Addressing::AbsoluteX, slo),
    0x20 => instr!(0x20, "JSR", 3, 6, Addressing::Absolute, jsr),
    0x21 => instr!(0x21, "AND", 2, 6, Addressing::IndirectX, and),
    0x22 => instr!(0x22, "*KIL", 1, 0, Addressing::Implied, kil),
    0x23 => instr!(0x23, "*RLA", 2, 8, Addressing::IndirectX, rla),
    0x24 => instr!(0x24, "BIT", 2, 3, Addressing::ZeroPage, bit),
    0x25 => instr!(0x25, "AND", 2, 3, Addressing::ZeroPage, and),
    0x26 => instr!(0x26, "ROL", 2, 5, Addressing::ZeroPage, rol),
    0x27 => instr!(0x27, "*RLA", 2, 5, Addressing::ZeroPage, rla),
    0x28 => instr!(0x28, "PLP", 1, 4, Addressing::Implied, plp),
    0x29 => instr!(0x29, "AND", 2, 2, Addressing::Immediate, and),
    0x2a => instr!(0x2a, "ROL", 1, 2, Addressing::Accumulator, rol),
    0x2b => instr!(0x2b, "*ANC", 2, 2, Addressing::Immediate, anc),
    0x2c => instr!(0x2c, "BIT", 3, 4, Addressing::Absolute, bit),
    0x2d => instr!(0x2d, "AND", 3, 4, Addressing::Absolute, and),
    0x2e => instr!(0x2e, "ROL", 3, 6, Addressing::Absolute, rol),
    0x2f => instr!(0x2f, "*RLA", 3, 6, Addressing::Absolute, rla),
    0x30 => instr!(0x30, "BMI", 2, 2, Addressing::Relative, bmi),
    0x31 => instr!(0x31, "AND", 2, 5, Addressing::IndirectYPageCrossed, and),
    0x32 => instr!(0x32, "*KIL", 1, 0, Addressing::Implied, kil),
    0x33 => instr!(0x33, "*RLA", 2, 8, Addressing::IndirectY, rla),
    0x34 => instr!(0x34, "*NOP", 2, 4, Addressing::ZeroPageX, nop),
    0x35 => instr!(0x35, "AND", 2, 4, Addressing::ZeroPageX, and),
    0x36 => instr!(0x36, "ROL", 2, 6, Addressing::ZeroPageX, rol),
    0x37 => instr!(0x37, "*RLA", 2, 6, Addressing::ZeroPageX, rla),
    0x38 => instr!(0x38, "SEC", 1, 2, Addressing::Implied, sec),
    0x39 => instr!(0x39, "AND", 3, 4, Addressing::AbsoluteYPageCrossed, and),
    0x3a => instr!(0x3a, "*NOP", 1, 2, Addressing::Implied, nop),
    0x3b => instr!(0x3b, "*RLA", 3, 7, Addressing::AbsoluteY, rla),
    0x3c => instr!(0x3c, "*NOP", 3, 4, Addressing::AbsoluteXPageCrossed, nop),
    0x3d => instr!(0x3d, "AND", 3, 4, Addressing::AbsoluteXPageCrossed, and),
    0x3e => instr!(0x3e, "ROL", 3, 7, Addressing::AbsoluteX, rol),
    0x3f => instr!(0x3f, "*RLA", 3, 7, Addressing::AbsoluteX, rla),
    0x40 => instr!(0x40, "RTI", 1, 6, Addressing::Implied, rti),
    0x41 => instr!(0x41, "EOR", 2, 6, Addressing::IndirectX, eor),
    0x42 => instr!(0x42, "*KIL", 1, 0, Addressing::Implied, kil),
    0x43 => instr!(0x43, "*SRE", 2, 8, Addressing::IndirectX, sre),
    0x44 => instr!(0x44, "*NOP", 2, 3, Addressing::ZeroPage, nop),
    0x45 => instr!(0x45, "EOR", 2, 3, Addressing::ZeroPage, eor),
    0x46 => instr!(0x46, "LSR", 2, 5, Addressing::ZeroPage, lsr),
    0x47 => instr!(0x47, "*SRE", 2, 5, Addressing::ZeroPage, sre),
    0x48 => instr!(0x48, "PHA", 1, 3, Addressing::Implied, pha),
    0x49 => instr!(0x49, "EOR", 2, 2, Addressing::Immediate, eor),
    0x4a => instr!(0x4a, "LSR", 1, 2, Addressing::Accumulator, lsr),
    0x4b => instr!(0x4b, "*ALR", 2, 2, Addressing::Immediate, alr),
    0x4c => instr!(0x4c, "JMP", 3, 3, Addressing::Absolute, jmp_absolute),
    0x4d => instr!(0x4d, "EOR", 3, 4, Addressing::Absolute, eor),
    0x4e => instr!(0x4e, "LSR", 3, 6, Addressing::Absolute, lsr),
    0x4f => instr!(0x4f, "*SRE", 3, 6, Addressing::Absolute, sre),
    0x50 => instr!(0x50, "BVC", 2, 2, Addressing::Relative, bvc),
    0x51 => instr!(0x51, "EOR", 2, 5, Addressing::IndirectYPageCrossed, eor),
    0x52 => instr!(0x52, "*KIL", 1, 0, Addressing::Implied, kil),
    0x53 => instr!(0x53, "*SRE", 2, 8, Addressing::IndirectY, sre),
    0x54 => instr!(0x54, "*NOP", 2, 4, Addressing::ZeroPageX, nop),
    0x55 => instr!(0x55, "EOR", 2, 4, Addressing::ZeroPageX, eor),
    0x56 => instr!(0x56, "LSR", 2, 6, Addressing::ZeroPageX, lsr),
    0x57 => instr!(0x57, "*SRE", 2, 6, Addressing::ZeroPageX, sre),
    0x58 => instr!(0x58, "CLI", 1, 2, Addressing::Implied, cli),
    0x59 => instr!(0x59, "EOR", 3, 4, Addressing::AbsoluteYPageCrossed, eor),
    0x5a => instr!(0x5a, "*NOP", 1, 2, Addressing::Implied, nop),
    0x5b => instr!(0x5b, "*SRE", 3, 7, Addressing::AbsoluteY, sre),
    0x5c => instr!(0x5c, "*NOP", 3, 4, Addressing::AbsoluteXPageCrossed, nop),
    0x5d => instr!(0x5d, "EOR", 3, 4, Addressing::AbsoluteXPageCrossed, eor),
    0x5e => instr!(0x5e, "LSR", 3, 7, Addressing::AbsoluteX, lsr),
    0x5f => instr!(0x5f, "*SRE", 3, 7, Addressing::AbsoluteX, sre),
    0x60 => instr!(0x60, "RTS", 1, 6, Addressing::Implied, rts),
    0x61 => instr!(0x61, "ADC", 2, 6, Addressing::IndirectX, adc),
    0x62 => instr!(0x62, "*KIL", 1, 0, Addressing::Implied, kil),
    0x63 => instr!(0x63, "*RRA", 2, 8, Addressing::IndirectX, rra),
    0x64 => instr!(0x64, "*NOP", 2, 3, Addressing::ZeroPage, nop),
    0x65 => instr!(0x65, "ADC", 2, 3, Addressing::ZeroPage, adc),
    0x66 => instr!(0x66, "ROR", 2, 5, Addressing::ZeroPage, ror),
    0x67 => instr!(0x67, "*RRA", 2, 5, Addressing::ZeroPage, rra),
    0x68 => instr!(0x68, "PLA", 1, 4, Addressing::Implied, pla),
    0x69 => instr!(0x69, "ADC", 2, 2, Addressing::Immediate, adc),
    0x6a => instr!(0x6a, "ROR", 1, 2, Addressing::Accumulator, ror),
    0x6b => instr!(0x6b, "*ARR", 2, 2, Addressing::Immediate, arr),
    0x6c => instr!(0x6c, "JMP", 3, 5, Addressing::Indirect, jmp_indirect),
    0x6d => instr!(0x6d, "ADC", 3, 4, Addressing::Absolute, adc),
    0x6e => instr!(0x6e, "ROR", 3, 6, Addressing::Absolute, ror),
    0x6f => instr!(0x6f, "*RRA", 3, 6, Addressing::Absolute, rra),
    0x70 => instr!(0x70, "BVS", 2, 2, Addressing::Relative, bvs),
    0x71 => instr!(0x71, "ADC", 2, 5, Addressing::IndirectYPageCrossed, adc),
    0x72 => instr!(0x72, "*KIL", 1, 0, Addressing::Implied, kil),
    0x73 => instr!(0x73, "*RRA", 2, 8, Addressing::IndirectY, rra),
    0x74 => instr!(0x74, "*NOP", 2, 4, Addressing::ZeroPageX, nop),
    0x75 => instr!(0x75, "ADC", 2, 4, Addressing::ZeroPageX, adc),
    0x76 => instr!(0x76, "ROR", 2, 6, Addressing::ZeroPageX, ror),
    0x77 => instr!(0x77, "*RRA", 2, 6, Addressing::ZeroPageX, rra),
    0x78 => instr!(0x78, "SEI", 1, 2, Addressing::Implied, sei),
    0x79 => instr!(0x79, "ADC", 3, 4, Addressing::AbsoluteYPageCrossed, adc),
    0x7a => instr!(0x7a, "*NOP", 1, 2, Addressing::Implied, nop),
    0x7b => instr!(0x7b, "*RRA", 3, 7, Addressing::AbsoluteY, rra),
    0x7c => instr!(0x7c, "*NOP", 3, 4, Addressing::AbsoluteXPageCrossed, nop),
    0x7d => instr!(0x7d, "ADC", 3, 4, Addressing::AbsoluteXPageCrossed, adc),
    0x7e => instr!(0x7e, "ROR", 3, 7, Addressing::AbsoluteX, ror),
    0x7f => instr!(0x7f, "*RRA", 3, 7, Addressing::AbsoluteX, rra),
    0x80 => instr!(0x80, "*NOP", 2, 2, Addressing::Immediate, nop),
    0x81 => instr!(0x81, "STA", 2, 6, Addressing::IndirectX, sta),
    0x82 => instr!(0x82, "*NOP", 2, 2, Addressing::Immediate, nop),
    0x83 => instr!(0x83, "*SAX", 2, 6, Addressing::IndirectX, sax),
    0x84 => instr!(0x84, "STY", 2, 3, Addressing::ZeroPage, sty),
    0x85 => instr!(0x85, "STA", 2, 3, Addressing::ZeroPage, sta),
    0x86 => instr!(0x86, "STX", 2, 3, Addressing::ZeroPage, stx),
    0x87 => instr!(0x87, "*SAX", 2, 3, Addressing::ZeroPage, sax),
    0x88 => instr!(0x88, "DEY", 1, 2, Addressing::Implied, dey),
    0x89 => instr!(0x89, "*NOP", 2, 2, Addressing::Immediate, nop),
    0x8a => instr!(0x8a, "TXA", 1, 2, Addressing::Implied, txa),
    0x8b => instr!(0x8b, "*XAA", 2, 2, Addressing::Immediate, xaa),
    0x8c => instr!(0x8c, "STY", 3, 4, Addressing::Absolute, sty),
    0x8d => instr!(0x8d, "STA", 3, 4, Addressing::Absolute, sta),
    0x8e => instr!(0x8e, "STX", 3, 4, Addressing::Absolute, stx),
    0x8f => instr!(0x8f, "*SAX", 3, 4, Addressing::Absolute, sax),
    0x90 => instr!(0x90, "BCC", 2, 2, Addressing::Relative, bcc),
    0x91 => instr!(0x91, "STA", 2, 6, Addressing::IndirectY, sta),
    0x92 => instr!(0x92, "*KIL", 1, 0, Addressing::Implied, kil),
    0x93 => instr!(0x93, "*AHX", 2, 6, Addressing::IndirectY, sha),
    0x94 => instr!(0x94, "STY", 2, 4, Addressing::ZeroPageX, sty),
    0x95 => instr!(0x95, "STA", 2, 4, Addressing::ZeroPageX, sta),
    0x96 => instr!(0x96, "STX", 2, 4, Addressing::ZeroPageY, stx),
    0x97 => instr!(0x97, "*SAX", 2, 4, Addressing::ZeroPageY, sax),
    0x98 => instr!(0x98, "TYA", 1, 2, Addressing::Implied, tya),
    0x99 => instr!(0x99, "STA", 3, 5, Addressing::AbsoluteY, sta),
    0x9a => instr!(0x9a, "TXS", 1, 2, Addressing::Implied, txs),
    0x9b => instr!(0x9b, "*TAS", 3, 5, Addressing::AbsoluteY, tas),
    0x9c => instr!(0x9c, "*SHY", 3, 5, Addressing::AbsoluteX, shy),
    0x9d => instr!(0x9d, "STA", 3, 5, Addressing::AbsoluteX, sta),
    0x9e => instr!(0x9e, "*SHX", 3, 5, Addressing::AbsoluteY, shx),
    0x9f => instr!(0x9f, "*AHX", 3, 5, Addressing::AbsoluteY, sha),
    0xa0 => instr!(0xa0, "LDY", 2, 2, Addressing::Immediate, ldy),
    0xa1 => instr!(0xa1, "LDA", 2, 6, Addressing::IndirectX, lda),
    0xa2 => instr!(0xa2, "LDX", 2, 2, Addressing::Immediate, ldx),
    0xa3 => instr!(0xa3, "*LAX", 2, 6, Addressing::IndirectX, lax),
    0xa4 => instr!(0xa4, "LDY", 2, 3, Addressing::ZeroPage, ldy),
    0xa5 => instr!(0xa5, "LDA", 2, 3, Addressing::ZeroPage, lda),
    0xa6 => instr!(0xa6, "LDX", 2, 3, Addressing::ZeroPage, ldx),
    0xa7 => instr!(0xa7, "*LAX", 2, 3, Addressing::ZeroPage, lax),
    0xa8 => instr!(0xa8, "TAY", 1, 2, Addressing::Implied, tay),
    0xa9 => instr!(0xa9, "LDA", 2, 2, Addressing::Immediate, lda),
    0xab => instr!(0xab, "*LAX", 2, 2, Addressing::Immediate, lax),
    0xaf => instr!(0xaf, "*LAX", 3, 4, Addressing::Absolute, lax),
    0xaa => instr!(0xaa, "TAX", 1, 2, Addressing::Implied, tax),
    0xac => instr!(0xac, "LDY", 3, 4, Addressing::Absolute, ldy),
    0xad => instr!(0xad, "LDA", 3, 4, Addressing::Absolute, lda),
    0xae => instr!(0xae, "LDX", 3, 4, Addressing::Absolute, ldx),
    0xb0 => instr!(0xb0, "BCS", 2, 2, Addressing::Relative, bcs),
    0xb1 => instr!(0xb1, "LDA", 2, 5, Addressing::IndirectYPageCrossed, lda),
    0xb2 => instr!(0xb2, "*KIL", 1, 0, Addressing::Implied, kil),
    0xb3 => instr!(0xb3, "*LAX", 2, 5, Addressing::IndirectYPageCrossed, lax),
    0xb4 => instr!(0xb4, "LDY", 2, 4, Addressing::ZeroPageX, ldy),
    0xb5 => instr!(0xb5, "LDA", 2, 4, Addressing::ZeroPageX, lda),
    0xb6 => instr!(0xb6, "LDX", 2, 4, Addressing::ZeroPageY, ldx),
    0xb7 => instr!(0xb7, "*LAX", 2, 4, Addressing::ZeroPageY, lax),
    0xb8 => instr!(0xb8, "CLV", 1, 2, Addressing::Implied, clv),
    0xb9 => instr!(0xb9, "LDA", 3, 4, Addressing::AbsoluteYPageCrossed, lda),
    0xba => instr!(0xba, "TSX", 1, 2, Addressing::Implied, tsx),
    0xbb => instr!(0xbb, "*LAS", 3, 4, Addressing::AbsoluteYPageCrossed, las),
    0xbc => instr!(0xbc, "LDY", 3, 4, Addressing::AbsoluteXPageCrossed, ldy),
    0xbd => instr!(0xbd, "LDA", 3, 4, Addressing::AbsoluteXPageCrossed, lda),
    0xbe => instr!(0xbe, "LDX", 3, 4, Addressing::AbsoluteYPageCrossed, ldx),
    0xbf => instr!(0xbf, "*LAX", 3, 4, Addressing::AbsoluteYPageCrossed, lax),
    0xc0 => instr!(0xc0, "CPY", 2, 2, Addressing::Immediate, cpy),
    0xc1 => instr!(0xc1, "CMP", 2, 6, Addressing::IndirectX, cmp),
    0xc2 => instr!(0xc2, "*NOP", 2, 2, Addressing::Immediate, nop),
    0xc3 => instr!(0xc3, "*DCP", 2, 8, Addressing::IndirectX, dcp),
    0xc4 => instr!(0xc4, "CPY", 2, 3, Addressing::ZeroPage, cpy),
    0xc5 => instr!(0xc5, "CMP", 2, 3, Addressing::ZeroPage, cmp),
    0xc6 => instr!(0xc6, "DEC", 2, 5, Addressing::ZeroPage, dec),
    0xc7 => instr!(0xc7, "*DCP", 2, 5, Addressing::ZeroPage, dcp),
    0xc8 => instr!(0xc8, "INY", 1, 2, Addressing::Implied, iny),
    0xc9 => instr!(0xc9, "CMP", 2, 2, Addressing::Immediate, cmp),
    0xca => instr!(0xca, "DEX", 1, 2, Addressing::Implied, dex),
    0xcb => instr!(0xcb, "*AXS", 2, 2, Addressing::Immediate, axs),
    0xcc => instr!(0xcc, "CPY", 3, 4, Addressing::Absolute, cpy),
    0xcd => instr!(0xcd, "CMP", 3, 4, Addressing::Absolute, cmp),
    0xce => instr!(0xce, "DEC", 3, 6, Addressing::Absolute, dec),
    0xcf => instr!(0xcf, "*DCP", 3, 6, Addressing::Absolute, dcp),
    0xd0 => instr!(0xd0, "BNE", 2, 2, Addressing::Relative, bne),
    0xd1 => instr!(0xd1, "CMP", 2, 5, Addressing::IndirectYPageCrossed, cmp),
    0xd2 => instr!(0xd2, "*KIL", 1, 0, Addressing::Implied, kil),
    0xd3 => instr!(0xd3, "*DCP", 2, 8, Addressing::IndirectY, dcp),
    0xd4 => instr!(0xd4, "*NOP", 2, 4, Addressing::ZeroPageX, nop),
    0xd5 => instr!(0xd5, "CMP", 2, 4, Addressing::ZeroPageX, cmp),
    0xd6 => instr!(0xd6, "DEC", 2, 6, Addressing::ZeroPageX, dec),
    0xd7 => instr!(0xd7, "*DCP", 2, 6, Addressing::ZeroPageX, dcp),
    0xd8 => instr!(0xd8, "CLD", 1, 2, Addressing::Implied, cld),
    0xd9 => instr!(0xd9, "CMP", 3, 4, Addressing::AbsoluteYPageCrossed, cmp),
    0xda => instr!(0xda, "*NOP", 1, 2, Addressing::Implied, nop),
    0xdb => instr!(0xdb, "*DCP", 3, 7, Addressing::AbsoluteY, dcp),
    0xdc => instr!(0xdc, "*NOP", 3, 4, Addressing::AbsoluteXPageCrossed, nop),
    0xdd => instr!(0xdd, "CMP", 3, 4, Addressing::AbsoluteXPageCrossed, cmp),
    0xde => instr!(0xde, "DEC", 3, 7, Addressing::AbsoluteX, dec),
    0xdf => instr!(0xdf, "*DCP", 3, 7, Addressing::AbsoluteX, dcp),
    0xe0 => instr!(0xe0, "CPX", 2, 2, Addressing::Immediate, cpx),
    0xe1 => instr!(0xe1, "SBC", 2, 6, Addressing::IndirectX, sbc),
    0xe2 => instr!(0xe2, "*NOP", 2, 2, Addressing::Immediate, nop),
    0xe3 => instr!(0xe3, "*ISB", 2, 8, Addressing::IndirectX, isb),
    0xe4 => instr!(0xe4, "CPX", 2, 3, Addressing::ZeroPage, cpx),
    0xe5 => instr!(0xe5, "SBC", 2, 3, Addressing::ZeroPage, sbc),
    0xe6 => instr!(0xe6, "INC", 2, 5, Addressing::ZeroPage, inc),
    0xe7 => instr!(0xe7, "*ISB", 2, 5, Addressing::ZeroPage, isb),
    0xe8 => instr!(0xe8, "INX", 1, 2, Addressing::Implied, inx),
    0xe9 => instr!(0xe9, "SBC", 2, 2, Addressing::Immediate, sbc),
    0xea => instr!(0xea, "NOP", 1, 2, Addressing::Implied, nop),
    0xeb => instr!(0xeb, "*SBC", 2, 2, Addressing::Immediate, sbc),
    0xec => instr!(0xec, "CPX", 3, 4, Addressing::Absolute, cpx),
    0xed => instr!(0xed, "SBC", 3, 4, Addressing::Absolute, sbc),
    0xee => instr!(0xee, "INC", 3, 6, Addressing::Absolute, inc),
    0xef => instr!(0xef, "*ISB", 3, 6, Addressing::Absolute, isb),
    0xf0 => instr!(0xf0, "BEQ", 2, 2, Addressing::Relative, beq),
    0xf1 => instr!(0xf1, "SBC", 2, 5, Addressing::IndirectYPageCrossed, sbc),
    0xf2 => instr!(0xf2, "*KIL", 1, 0, Addressing::Implied, kil),
    0xf3 => instr!(0xf3, "*ISB", 2, 8, Addressing::IndirectY, isb),
    0xf4 => instr!(0xf4, "*NOP", 2, 4, Addressing::ZeroPageX, nop),
    0xf5 => instr!(0xf5, "SBC", 2, 4, Addressing::ZeroPageX, sbc),
    0xf6 => instr!(0xf6, "INC", 2, 6, Addressing::ZeroPageX, inc),
    0xf7 => instr!(0xf7, "*ISB", 2, 6, Addressing::ZeroPageX, isb),
    0xf8 => instr!(0xf8, "SED", 1, 2, Addressing::Implied, sed),
    0xf9 => instr!(0xf9, "SBC", 3, 4, Addressing::AbsoluteYPageCrossed, sbc),
    0xfa => instr!(0xfa, "*NOP", 1, 2, Addressing::Implied, nop),
    0xfb => instr!(0xfb, "*ISB", 3, 7, Addressing::AbsoluteY, isb),
    0xfc => instr!(0xfc, "*NOP", 3, 4, Addressing::AbsoluteXPageCrossed, nop),
    0xfd => instr!(0xfd, "SBC", 3, 4, Addressing::AbsoluteXPageCrossed, sbc),
    0xfe => instr!(0xfe, "INC", 3, 7, Addressing::AbsoluteX, inc),
    0xff => instr!(0xff, "*ISB", 3, 7, Addressing::AbsoluteX, isb),
};

impl fmt::Display for Instruction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({:02X}) {}:{} - {:?}", self.cmd, self.op, self.len, self.cycles, self.mode)
    }
}

fn page_crossed(lh: u16, rh: u16) -> bool {
    const PAGE_MASK: u16 = 0xFF00;
    (lh & PAGE_MASK) != (rh & PAGE_MASK)
}

impl Instruction {
    pub fn resolve_operand_address(&self, cpu: &Cpu) -> OperandAddress {
        let pc = cpu.pc;

        match self.mode {
            Addressing::Implied | Addressing::Accumulator =>
                OperandAddress::None,

            Addressing::Immediate =>
                OperandAddress::Immediate(cpu.read::<u8>(pc + 1)),

            Addressing::ZeroPage => {
                let zp = cpu.read::<u8>(pc + 1);
                OperandAddress::Address(zp as u16)
            }

            Addressing::ZeroPageX => {
                let base = cpu.read::<u8>(pc + 1);
                let addr = base.wrapping_add(cpu.rx);
                OperandAddress::AddressWithBase {
                    base: base as u16,
                    index: 'X',
                    addr: addr as u16,
                }
            }

            Addressing::ZeroPageY => {
                let base = cpu.read::<u8>(pc + 1);
                let addr = base.wrapping_add(cpu.ry);
                OperandAddress::AddressWithBase {
                    base: base as u16,
                    index: 'Y',
                    addr: addr as u16,
                }
            }

            Addressing::Relative => {
                let off = cpu.read::<u8>(pc + 1) as i8;
                let target = (pc as i16 + 2 + off as i16) as u16;
                OperandAddress::Address(target)
            }

            Addressing::Absolute => {
                let lo = cpu.read::<u8>(pc + 1) as u16;
                let hi = cpu.read::<u8>(pc + 2) as u16;
                OperandAddress::Address((hi << 8) | lo)
            }

            Addressing::AbsoluteX => {
                let lo = cpu.read::<u8>(pc + 1) as u16;
                let hi = cpu.read::<u8>(pc + 2) as u16;
                let base = (hi << 8) | lo;
                OperandAddress::AddressWithBase {
                    base,
                    index: 'X',
                    addr: base.wrapping_add(cpu.rx as u16),
                }
            }

            Addressing::AbsoluteXPageCrossed => {
                let lo = cpu.read::<u8>(pc + 1) as u16;
                let hi = cpu.read::<u8>(pc + 2) as u16;
                let base = (hi << 8) | lo;
                OperandAddress::AddressWithBasePageCrossed {
                    base,
                    index: 'X',
                    addr: base.wrapping_add(cpu.rx as u16),
                }
            }

            Addressing::AbsoluteY => {
                let lo = cpu.read::<u8>(pc + 1) as u16;
                let hi = cpu.read::<u8>(pc + 2) as u16;
                let base = (hi << 8) | lo;
                OperandAddress::AddressWithBase {
                    base,
                    index: 'Y',
                    addr: base.wrapping_add(cpu.ry.into()) as u16,
                }
            }

            Addressing::AbsoluteYPageCrossed => {
                let lo = cpu.read::<u8>(pc + 1) as u16;
                let hi = cpu.read::<u8>(pc + 2) as u16;
                let base = (hi << 8) | lo;
                OperandAddress::AddressWithBasePageCrossed {
                    base,
                    index: 'Y',
                    addr: base.wrapping_add(cpu.ry.into()) as u16,
                }
            }

            Addressing::Indirect => {
                let lo = cpu.read::<u8>(pc + 1) as u16;
                let hi = cpu.read::<u8>(pc + 2) as u16;
                let ptr = (hi << 8) | lo;

                let lo2 = cpu.read::<u8>(ptr) as u16;
                let hi2 = cpu.read::<u8>((ptr & 0xFF00) | ((ptr + 1) & 0x00FF)) as u16;
                OperandAddress::Indirect {
                    ptr,
                    target: (hi2 << 8) | lo2,
                }
            }

            Addressing::IndirectX => {
                let zp = cpu.read::<u8>(pc + 1);
                let ptr = zp.wrapping_add(cpu.rx);
                let lo = cpu.read::<u8>(ptr as u16) as u16;
                let hi = cpu.read::<u8>(ptr.wrapping_add(1) as u16) as u16;
                OperandAddress::IndirectX {
                    zp,
                    ptr,
                    addr: (hi << 8) | lo,
                }
            }

            Addressing::IndirectY => {
                let zp = cpu.read::<u8>(pc + 1);
                let lo = cpu.read::<u8>(zp as u16) as u16;
                let hi = cpu.read::<u8>(zp.wrapping_add(1) as u16) as u16;
                let base = (hi << 8) | lo;
                OperandAddress::IndirectY {
                    zp,
                    base,
                    addr: base.wrapping_add(cpu.ry.into()) as u16,
                }
            }
            Addressing::IndirectYPageCrossed => {
                let zp = cpu.read::<u8>(pc + 1);
                let lo = cpu.read::<u8>(zp as u16) as u16;
                let hi = cpu.read::<u8>(zp.wrapping_add(1) as u16) as u16;
                let base = (hi << 8) | lo;
                OperandAddress::IndirectYPageCrossed {
                    zp,
                    base,
                    addr: base.wrapping_add(cpu.ry.into()) as u16,
                }
            }
        }
    }

    pub fn fetch_operand(&self, cpu: &Cpu) -> (Operand, bool) {
        let oa = self.resolve_operand_address(cpu);

        if self.len == 1
            && matches!(self.mode, Addressing::Implied | Addressing::Accumulator)
            && !matches!(self.cmd, "BRK" | "RTI" | "RTS")
        {
            let _ = cpu.read::<u8>(cpu.pc.wrapping_add(1));
        }

        let page_cross = match oa {
            OperandAddress::AddressWithBasePageCrossed { base, addr, .. } 
                | OperandAddress::IndirectYPageCrossed { base, addr, .. } => 
            {
                page_crossed(base, addr)
            }
            _ => false,
        };

        if let Some(dummy_addr) = self.dummy_read_addr(oa, page_cross) {
            let _ = cpu.read::<u8>(dummy_addr);
        }

        (oa.to_operand(), page_cross)
    }

    fn dummy_read_addr(&self, operand: OperandAddress, page_cross: bool) -> Option<u16> {
        fn indexed_dummy_addr(base: u16, addr: u16) -> u16 {
            (base & 0xFF00) | (addr & 0x00FF)
        }

        fn is_rmw(cmd: &str) -> bool {
            matches!(
                cmd,
                "ASL" | "LSR" | "ROL" | "ROR" | "INC" | "DEC"
                    | "*SLO" | "*SRE" | "*RLA" | "*RRA" | "*DCP" | "*ISB"
            )
        }

        fn is_store(cmd: &str) -> bool {
            matches!(
                cmd,
                "STA" | "STX" | "STY" | "*SAX" | "*AHX" | "*TAS" | "*SHX" | "*SHY"
            )
        }

        match operand {
            OperandAddress::AddressWithBase { base, addr, .. }
            | OperandAddress::AddressWithBasePageCrossed { base, addr, .. } => {
                if is_rmw(self.cmd) {
                    return Some(indexed_dummy_addr(base, addr));
                }

                if is_store(self.cmd)
                    && matches!(
                        self.mode,
                        Addressing::AbsoluteX
                            | Addressing::AbsoluteY
                            | Addressing::AbsoluteXPageCrossed
                            | Addressing::AbsoluteYPageCrossed
                    )
                {
                    return Some(indexed_dummy_addr(base, addr));
                }

                if page_cross
                    && matches!(
                        self.cmd,
                        "LDA" | "LDX" | "LDY" | "ADC" | "AND" | "CMP" | "EOR" | "ORA" | "SBC"
                            | "*LAX" | "*LAS" | "*NOP"
                    )
                {
                    return Some(indexed_dummy_addr(base, addr));
                }
            }
            OperandAddress::IndirectY { base, addr, .. }
            | OperandAddress::IndirectYPageCrossed { base, addr, .. } => {
                if is_rmw(self.cmd) {
                    return Some(indexed_dummy_addr(base, addr));
                }

                if is_store(self.cmd) {
                    return Some(indexed_dummy_addr(base, addr));
                }

                if page_cross
                    && matches!(
                        self.cmd,
                        "LDA" | "ADC" | "AND" | "CMP" | "EOR" | "ORA" | "SBC" | "*LAX" | "*LAS"
                    )
                {
                    return Some(indexed_dummy_addr(base, addr));
                }
            }
            _ => {}
        }

        None
    }
}

// Branch
fn branch(cpu: &mut Cpu, operand: &mut OperandContext, condition: bool) {
    if let Some(addr) = operand.value.as_address() {
        if condition {
            operand.extra_cycles = if page_crossed(cpu.pc, addr) { 2 } else { 1 };
            cpu.pc = addr;
        }
    } else {
        panic!("branch instruction must use relative addressing");
    }
}

fn bcs(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, cpu.flags.contains(CpuFlags::CARRY));
}

fn bcc(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, !cpu.flags.contains(CpuFlags::CARRY));
}

fn beq(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, cpu.flags.contains(CpuFlags::ZERO));
}

fn bne(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, !cpu.flags.contains(CpuFlags::ZERO));
}

fn bmi(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, cpu.flags.contains(CpuFlags::NEGATIV));
}

fn bpl(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, !cpu.flags.contains(CpuFlags::NEGATIV));
}

fn bvs(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, cpu.flags.contains(CpuFlags::OVERFLOW));
}

fn bvc(cpu: &mut Cpu, operand: &mut OperandContext) {
    branch(cpu, operand, !cpu.flags.contains(CpuFlags::OVERFLOW));
}

fn jsr(cpu: &mut Cpu, operand: &mut OperandContext) {
    cpu.push(cpu.pc - 1);

    if let Some(addr) = operand.value.as_address() {
        cpu.pc = addr;
    } else {
        panic!("JSR requires absolute addressing");
    }
}

fn jmp_absolute(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        cpu.pc = addr;
    } else {
        panic!("JMP absolute requires absolute addressing");
    }
}

fn jmp_indirect(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        cpu.pc = addr;
    } else {
        panic!("JMP indirect требует косвенной адресации");
    }
}

// Compare
fn compare(cpu: &mut Cpu, operand: &mut OperandContext, reg: u8) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("Compare requires an operand"),
    };

    if reg >= value {
        cpu.flags.insert(CpuFlags::CARRY);
    } else {
        cpu.flags.remove(CpuFlags::CARRY);
    }

    let diff = reg.wrapping_sub(value);
    cpu.update_zero_and_neg_flags(diff);
}

fn cmp(cpu: &mut Cpu, operand: &mut OperandContext) {
    compare(cpu, operand, cpu.acc);
}

fn cpx(cpu: &mut Cpu, operand: &mut OperandContext) {
    compare(cpu, operand, cpu.rx);
}

fn cpy(cpu: &mut Cpu, operand: &mut OperandContext) {
    compare(cpu, operand, cpu.ry);
}

// Stack
fn pha(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.push(cpu.acc);
}

fn pla(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.acc = cpu.pop();
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn php(cpu: &mut Cpu, _: &mut OperandContext) {
    let mut flags: CpuFlags = cpu.flags.clone();

    // http://wiki.nesdev.com/w/index.php/CPU_status_flag_behavior
    flags.insert( CpuFlags::BREAK );
    flags.insert( CpuFlags::BREAK2 );
    cpu.push(flags.bits());
}

fn plp(cpu: &mut Cpu, _: &mut OperandContext) {
    let previous_i_flag = cpu.flags.contains(CpuFlags::INTERRUPT_DISABLE);
    cpu.flags = CpuFlags::from_bits(cpu.pop()).expect("Cannot parse flags from stack");

    // http://wiki.nesdev.com/w/index.php/CPU_status_flag_behavior
    cpu.flags.remove(CpuFlags::BREAK);
    cpu.flags.insert(CpuFlags::BREAK2);
    cpu.delay_irq_flag_effect(previous_i_flag);
}

// Transfer
fn tax(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.rx = cpu.acc;
    cpu.update_zero_and_neg_flags(cpu.rx);
}

fn txa(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.acc = cpu.rx;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn tay(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.ry = cpu.acc;
    cpu.update_zero_and_neg_flags(cpu.ry);
}

fn tya(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.acc = cpu.ry;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn txs(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.sp = cpu.rx;
}

fn tsx(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.rx = cpu.sp;
    cpu.update_zero_and_neg_flags(cpu.rx);
}


// Shift
fn lshift(val: u8) -> (bool, u8) {
    let carry = (val & 0x80) != 0;
    (carry, val << 1)
}

fn rshift(val: u8) -> (bool, u8) {
    let carry = (val & 1) == 1;
    (carry, val >> 1)
}

#[inline]
fn rmw_write(cpu: &mut Cpu, addr: u16, original: u8, updated: u8) {
    cpu.write(addr, original);
    cpu.write(addr, updated);
}

fn asl(cpu: &mut Cpu, operand: &mut OperandContext) {
    match operand.value {
        Operand::None => { // Accumulator
            let (c, v) = lshift(cpu.acc);
            cpu.flags.set(CpuFlags::CARRY, c);
            cpu.acc = v;
            cpu.update_zero_and_neg_flags(cpu.acc);
        }
        Operand::Address(addr) => {
            let value: u8 = cpu.read(addr);
            let (c, v) = lshift(value);
            cpu.flags.set(CpuFlags::CARRY, c);
            rmw_write(cpu, addr, value, v);
            cpu.update_zero_and_neg_flags(v);
        }
        _ => panic!("ASL does not support this addressing mode"),
    }
}

fn lsr(cpu: &mut Cpu, operand: &mut OperandContext) {
    match operand.value {
        Operand::None => {
            let (c, v) = rshift(cpu.acc);
            cpu.flags.set(CpuFlags::CARRY, c);
            cpu.acc = v;
            cpu.update_zero_and_neg_flags(cpu.acc);
        }
        Operand::Address(addr) => {
            let value: u8 = cpu.read(addr);
            let (c, v) = rshift(value);
            cpu.flags.set(CpuFlags::CARRY, c);
            rmw_write(cpu, addr, value, v);
            cpu.update_zero_and_neg_flags(v);
        }
        _ => panic!("LSR does not support this addressing mode"),
    }
}

fn rol(cpu: &mut Cpu, operand: &mut OperandContext) {
    let carry_in = cpu.flags.contains(CpuFlags::CARRY);

    match operand.value {
        Operand::None => {
            let (c, mut v) = lshift(cpu.acc);
            cpu.flags.set(CpuFlags::CARRY, c);
            if carry_in {
                v |= 1;
            }
            cpu.acc = v;
            cpu.update_zero_and_neg_flags(cpu.acc);
        }
        Operand::Address(addr) => {
            let value: u8 = cpu.read(addr);
            let (c, mut v) = lshift(value);
            cpu.flags.set(CpuFlags::CARRY, c);
            if carry_in {
                v |= 1;
            }
            rmw_write(cpu, addr, value, v);
            cpu.update_zero_and_neg_flags(v);
        }
        _ => panic!("ROL does not support this addressing mode"),
    }
}

fn ror(cpu: &mut Cpu, operand: &mut OperandContext) {
    let carry_in = cpu.flags.contains(CpuFlags::CARRY);

    match operand.value {
        Operand::None => {
            let (c, mut v) = rshift(cpu.acc);
            cpu.flags.set(CpuFlags::CARRY, c);
            if carry_in {
                v |= 0b1000_0000;
            }
            cpu.acc = v;
            cpu.update_zero_and_neg_flags(cpu.acc);
        }
        Operand::Address(addr) => {
            let value: u8 = cpu.read(addr);
            let (c, mut v) = rshift(value);
            cpu.flags.set(CpuFlags::CARRY, c);
            if carry_in {
                v |= 0b1000_0000;
            }
            rmw_write(cpu, addr, value, v);
            cpu.update_zero_and_neg_flags(v);
        }
        _ => panic!("ROR does not support this addressing mode"),
    }
}

//
fn sec(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.flags |= CpuFlags::CARRY;
}

fn dex(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.rx = cpu.rx.wrapping_sub(1);
    cpu.update_zero_and_neg_flags(cpu.rx);
}

fn bit(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("BIT requires an operand"),
    };

    cpu.update_zero_flg(cpu.acc & value);
    cpu.update_neg_flg(value);
    cpu.update_ovfl_flg(value);
}

fn ora(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("ORA requires an operand"),
    };

    cpu.acc |= value;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn clc(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.flags.remove(CpuFlags::CARRY);
}

fn inc(cpu: &mut Cpu, operand: &mut OperandContext) {

    match operand.value {
        Operand::Address(addr) => {
            let value: u8 = cpu.read(addr);
            let updated = value.wrapping_add(1);
            rmw_write(cpu, addr, value, updated);
            cpu.update_zero_and_neg_flags(updated);
        }
        _ => panic!("INC only supports memory addressing"),
    }
}

fn isb(cpu: &mut Cpu, operand: &mut OperandContext) {
    match operand.value {
        Operand::Address(addr) => {
            let value: u8 = cpu.read(addr);
            let updated = value.wrapping_add(1);
            rmw_write(cpu, addr, value, updated);
            add_to_acc(cpu, updated.wrapping_neg().wrapping_sub(1));
        }
        _ => panic!("INC only supports memory addressing"),
    }
}

fn and(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("AND requires an operand"),
    };

    cpu.acc &= value;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn iny(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.ry = cpu.ry.wrapping_add(1);
    cpu.update_zero_and_neg_flags(cpu.ry);
}

fn sty(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        cpu.write(addr, cpu.ry);
    } else {
        panic!("STY requires a memory address operand");
    }
}

fn inx(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.rx = cpu.rx.wrapping_add(1);

    cpu.update_zero_and_neg_flags(cpu.rx);
}

fn sed(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.flags.insert(CpuFlags::DECIMAL_MODE);
}

fn clv(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.flags.remove(CpuFlags::OVERFLOW);
}

fn cld(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.flags.remove(CpuFlags::DECIMAL_MODE);
}

fn sei(cpu: &mut Cpu, _: &mut OperandContext) {
    let previous_i_flag = cpu.flags.contains(CpuFlags::INTERRUPT_DISABLE);
    cpu.flags.insert(CpuFlags::INTERRUPT_DISABLE);
    cpu.delay_irq_flag_effect(previous_i_flag);
}

fn cli(cpu: &mut Cpu, _: &mut OperandContext) {
    let previous_i_flag = cpu.flags.contains(CpuFlags::INTERRUPT_DISABLE);
    cpu.flags.remove(CpuFlags::INTERRUPT_DISABLE);
    cpu.delay_irq_flag_effect(previous_i_flag);
}

fn lda(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("LDA requires an operand"),
    };
    cpu.acc = value;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn ldy(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("LDY requires an operand"),
    };
    cpu.ry = value;
    cpu.update_zero_and_neg_flags(cpu.ry);
}

fn ldx(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("LDX requires an operand"),
    };
    cpu.rx = value;
    cpu.update_zero_and_neg_flags(cpu.rx);
}

fn lax(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("LAX requires an operand"),
    };
    cpu.acc = value;
    cpu.rx = cpu.acc;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn sax(cpu: &mut Cpu, operand: &mut OperandContext) {
    match operand.value {
        Operand::Address(addr) => {
            cpu.write(addr, cpu.rx & cpu.acc);
        }
        _ => panic!("SAX only supports memory addressing"),
    }
}

fn sta(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        cpu.write(addr, cpu.acc);
    } else {
        panic!("STA requires a memory address operand");
    }
}

fn stx(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        cpu.write(addr, cpu.rx);
    } else {
        panic!("STX requires a memory address operand");
    }
}

fn eor(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("EOR requires an operand"),
    };
    cpu.acc ^= value;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn dey(cpu: &mut Cpu, _: &mut OperandContext) {
    cpu.ry = cpu.ry.wrapping_sub(1);
    cpu.update_zero_and_neg_flags(cpu.ry);
}

fn dec(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        let value: u8 = cpu.read(addr);
        let updated = value.wrapping_sub(1);
        rmw_write(cpu, addr, value, updated);
        cpu.update_zero_and_neg_flags(updated);
    } else {
        panic!("DEC requires a memory address operand");
    }
}

fn dcp(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        let value: u8 = cpu.read(addr);
        let result: u8 = value.wrapping_sub(1);

        rmw_write(cpu, addr, value, result);
        cpu.flags.set(CpuFlags::CARRY, cpu.acc >= result);
        cpu.update_zero_and_neg_flags(cpu.acc.wrapping_sub(result));
    } else {
        panic!("DCP requires a memory address operand");
    }
}

fn rti(cpu: &mut Cpu, _: &mut OperandContext) {
    let _ = cpu.read::<u8>(cpu.pc);
    cpu.flags = CpuFlags::from_bits(cpu.pop::<u8>()).expect("Cannot parse flags from stack");
    cpu.flags.remove(CpuFlags::BREAK);
    cpu.flags.insert(CpuFlags::BREAK2);

    cpu.pc = cpu.pop();
}

fn rts(cpu: &mut Cpu, _: &mut OperandContext) {
    let _ = cpu.read::<u8>(cpu.pc);
    cpu.pc = cpu.pop::<u16>() + 1;
}

// http://www.righto.com/2012/12/the-6502-overflow-flag-explained.html
fn add_to_acc(cpu: &mut Cpu, val: u8) {
    let s = (cpu.acc as u16)
        .wrapping_add(val as u16)
        .wrapping_add(if cpu.flags.contains(CpuFlags::CARRY) { 1 } else { 0 });

    cpu.flags.set(CpuFlags::CARRY, s > 0xff);
    cpu.flags.set(CpuFlags::OVERFLOW, (val as u8 ^ s as u8) & (s as u8 ^ cpu.acc) & 0x80 != 0);
    cpu.acc = s as u8;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn sbc(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("SBC requires an operand"),
    };

    add_to_acc(cpu, value.wrapping_neg().wrapping_sub(1));
}

fn adc(cpu: &mut Cpu, operand: &mut OperandContext) {
    let value: u8 = match operand.value {
        Operand::Byte(v) => v,
        Operand::Address(addr) => cpu.read(addr),
        Operand::None => panic!("ADC requires an operand"),
    };

    add_to_acc(cpu, value);
}

fn brk(cpu: &mut Cpu, _: &mut OperandContext) {
    let _ = cpu.read::<u8>(cpu.pc);
    cpu.pc = cpu.pc.wrapping_add(1);
    cpu.interrupt(&BRK_INT);
}

fn slo(cpu: &mut Cpu, operand: &mut OperandContext) {
    if let Some(addr) = operand.value.as_address() {
        let value = cpu.read(addr);
        let (c, v) = lshift(value);
        cpu.flags.set(CpuFlags::CARRY, c);
        rmw_write(cpu, addr, value, v);
        cpu.update_zero_and_neg_flags(v);
        cpu.acc |= v;
        cpu.update_zero_and_neg_flags(cpu.acc);
    } else {
        panic!("SLO absolute requires absolute addressing");
    }
}

fn sre(cpu: &mut Cpu, operand: &mut OperandContext) {
    let addr = operand.value
        .as_address()
        .expect("SRE absolute requires absolute addressing");

    // LSR
    let value = cpu.read(addr);
    let (c, v) = rshift(value);
    cpu.flags.set(CpuFlags::CARRY, c);
    rmw_write(cpu, addr, value, v);
    cpu.update_zero_and_neg_flags(v);

    // EOR
    cpu.acc ^= v;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn rla(cpu: &mut Cpu, operand: &mut OperandContext) {
    let carry_in = cpu.flags.contains(CpuFlags::CARRY);

    if let Some(addr) = operand.value.as_address() {
        let value = cpu.read(addr);
        let (c, mut v) = lshift(value);
        cpu.flags.set(CpuFlags::CARRY, c);
        if carry_in {
            v |= 1;
        }
        rmw_write(cpu, addr, value, v);
        cpu.update_zero_and_neg_flags(v);
        cpu.acc &= v;
        cpu.update_zero_and_neg_flags(cpu.acc);
    } else {
        panic!("RLA absolute requires absolute addressing");
    }
}

fn rra(cpu: &mut Cpu, operand: &mut OperandContext) {
    let carry_in = cpu.flags.contains(CpuFlags::CARRY);

    if let Some(addr) = operand.value.as_address() {
        let value = cpu.read(addr);
        let (c, mut v) = rshift(value);
        cpu.flags.set(CpuFlags::CARRY, c);
        if carry_in {
            v |= 0b1000_0000;
        }
        rmw_write(cpu, addr, value, v);
        cpu.update_zero_and_neg_flags(v);

        add_to_acc(cpu, v);
    } else {
        panic!("RLA absolute requires absolute addressing");
    }
}

fn nop(_cpu: &mut Cpu, _: &mut OperandContext) {}

fn kil(cpu: &mut Cpu, _: &mut OperandContext) {
    panic!("KIL executed at {:04X}", cpu.pc);
}

// Wrotten by AI
fn axs(cpu: &mut Cpu, operand: &mut OperandContext) {
    let imm = match operand.value {
        Operand::Byte(v) => v,
        _ => panic!("AXS requires immediate operand"),
    };

    let ax = cpu.acc & cpu.rx;
    let result = ax.wrapping_sub(imm);

    cpu.flags.set(CpuFlags::CARRY, ax >= imm);
    cpu.rx = result;
    cpu.update_zero_and_neg_flags(cpu.rx);
}

fn xaa(cpu: &mut Cpu, operand: &mut OperandContext) {
    let imm = match operand.value {
        Operand::Byte(v) => v,
        _ => panic!("XAA requires immediate operand"),
    };

    let value = cpu.rx & imm;
    cpu.acc = value;
    cpu.update_zero_and_neg_flags(value);
}

fn anc(cpu: &mut Cpu, operand: &mut OperandContext) {
    let imm = match operand.value {
        Operand::Byte(v) => v,
        _ => panic!("ANC requires immediate operand"),
    };

    cpu.acc &= imm;
    cpu.update_zero_and_neg_flags(cpu.acc);
    cpu.flags.set(CpuFlags::CARRY, cpu.acc & 0x80 != 0);
}

fn alr(cpu: &mut Cpu, operand: &mut OperandContext) {
    let imm = match operand.value {
        Operand::Byte(v) => v,
        _ => panic!("ALR requires immediate operand"),
    };

    let value = cpu.acc & imm;
    cpu.flags.set(CpuFlags::CARRY, value & 1 != 0);

    cpu.acc = value >> 1;
    cpu.update_zero_and_neg_flags(cpu.acc);
}

fn arr(cpu: &mut Cpu, operand: &mut OperandContext) {
    let imm = match operand.value {
        Operand::Byte(v) => v,
        _ => panic!("ARR requires immediate operand"),
    };

    let carry_in = cpu.flags.contains(CpuFlags::CARRY);
    let mut value = cpu.acc & imm;

    value >>= 1;
    if carry_in {
        value |= 0x80;
    }

    cpu.acc = value;
    cpu.update_zero_and_neg_flags(cpu.acc);

    let bit6 = (cpu.acc >> 6) & 1;
    let bit5 = (cpu.acc >> 5) & 1;

    cpu.flags.set(CpuFlags::CARRY, bit6 == 1);
    cpu.flags
        .set(CpuFlags::OVERFLOW, (bit6 ^ bit5) == 1);
}

fn unstable_abs_xy_store(cpu: &Cpu, index: u8, data: u8) -> (u16, u8) {
    let lo = cpu.read::<u8>(cpu.pc.wrapping_sub(2)) as u16;
    let hi = cpu.read::<u8>(cpu.pc.wrapping_sub(1)) as u16;
    let base = (hi << 8) | lo;
    let addr = base.wrapping_add(index as u16);
    let masked = data & ((base >> 8) as u8).wrapping_add(1);

    if page_crossed(base, addr) {
        (((masked as u16) << 8) | (addr & 0x00FF), masked)
    } else {
        (addr, masked)
    }
}

fn shx(cpu: &mut Cpu, operand: &mut OperandContext) {
    if operand.value.as_address().is_none() {
        panic!("SHX requires memory addressing");
    }

    let (addr, value) = unstable_abs_xy_store(cpu, cpu.ry, cpu.rx);
    cpu.write(addr, value);
}

fn shy(cpu: &mut Cpu, operand: &mut OperandContext) {
    if operand.value.as_address().is_none() {
        panic!("SHY requires memory addressing");
    }

    let (addr, value) = unstable_abs_xy_store(cpu, cpu.rx, cpu.ry);
    cpu.write(addr, value);
}

fn sha(cpu: &mut Cpu, operand: &mut OperandContext) {
    if operand.value.as_address().is_none() {
        panic!("SHA requires memory addressing");
    }

    let index = if cpu.last_opcode == 0x93 { cpu.ry } else { cpu.ry };
    let (addr, value) = unstable_abs_xy_store(cpu, index, cpu.acc & cpu.rx);
    cpu.write(addr, value);
}

fn las(cpu: &mut Cpu, operand: &mut OperandContext) {
    let addr = operand.value
        .as_address()
        .expect("LAS requires memory addressing");

    let value = cpu.read::<u8>(addr) & cpu.sp;

    cpu.acc = value;
    cpu.rx = value;
    cpu.sp = value;

    cpu.update_zero_and_neg_flags(value);
}

fn tas(cpu: &mut Cpu, operand: &mut OperandContext) {
    if operand.value.as_address().is_none() {
        panic!("TAS requires memory addressing");
    }

    let sp = cpu.acc & cpu.rx;
    cpu.sp = sp;
    let (addr, value) = unstable_abs_xy_store(cpu, cpu.ry, sp);
    cpu.write(addr, value);
}

fn tbd(_cpu: &mut Cpu, _: &mut OperandContext) {
    panic!(
        "Instruction is not implemented: opcode {:02X} at {:04X}",
        _cpu.last_opcode,
        _cpu.pc.wrapping_sub(1)
    );
}
