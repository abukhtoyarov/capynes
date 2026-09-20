use std::fmt;

#[derive(Clone, Copy, Debug)]
pub enum OperandAddress {
    None,
    Immediate(u8),
    Address(u16),
    AddressWithBase { base: u16, index: char, addr: u16, },
    AddressWithBasePageCrossed { base: u16, index: char, addr: u16, },
    Indirect { ptr: u16, target: u16, },
    IndirectX { zp: u8, ptr: u8, addr: u16, },
    IndirectY { zp: u8, base: u16, addr: u16, },
    IndirectYPageCrossed { zp: u8, base: u16, addr: u16, },
}

#[derive(Clone, Copy, Debug)]
pub enum Operand {
    None,
    Byte(u8),
    Address(u16),
}

pub struct OperandContext {
    pub value: Operand,
    pub extra_cycles: u8,
}

impl Operand {
    pub fn as_address(&self) -> Option<u16> {
        match self {
            Operand::Address(a) => Some(*a),
            _ => None,
        }
    }
}

impl OperandAddress {
    pub fn to_operand(&self) -> Operand {
        match self {
            OperandAddress::None => Operand::None,
            OperandAddress::Immediate(v) => Operand::Byte(*v),
            OperandAddress::Address(a) => Operand::Address(*a),
            OperandAddress::AddressWithBase { addr, .. } => Operand::Address(*addr),
            OperandAddress::AddressWithBasePageCrossed { addr, .. } => Operand::Address(*addr),
            OperandAddress::Indirect { target, .. } => Operand::Address(*target),
            OperandAddress::IndirectX { addr, .. } => Operand::Address(*addr),
            OperandAddress::IndirectY { addr, .. } => Operand::Address(*addr),
            OperandAddress::IndirectYPageCrossed { addr, .. } => Operand::Address(*addr),
        }
    }

    pub fn format_trace<F>(&self, read_fn: F, cmd: &str) -> String
    where
        F: Fn(u16) -> u8,
    {
        match self {
            OperandAddress::None => String::new(),

            OperandAddress::Immediate(v) => format!("#${:02X}", v),

            OperandAddress::Address(addr) => {
                // Jump and Branch instructions (JMP, JSR, BCC, BCS, BEQ, BNE, BMI, BPL, BVC, BVS)
                let is_jump_or_branch = matches!(cmd, "JMP" | "JSR" | "BCC" | "BCS" | "BEQ" | "BNE" | "BMI" | "BPL" | "BVC" | "BVS");

                if is_jump_or_branch {
                    format!("${:04X}", addr)
                } else if *addr < 0x100 {
                    // ZeroPage
                    format!("${:02X} = {:02X}", addr, read_fn(*addr))
                } else {
                    // Absolute
                    format!("${:04X} = {:02X}", addr, read_fn(*addr))
                }
            }

            OperandAddress::AddressWithBase { base, index, addr }
                | OperandAddress::AddressWithBasePageCrossed { base, index, addr } => 
            {
                if *base < 0x100 {
                    // ZeroPage indexed
                    format!(
                        "${:02X},{} @ {:02X} = {:02X}",
                        base, index, addr, read_fn(*addr)
                    )
                } else {
                    // Absolute indexed
                    format!(
                        "${:04X},{} @ {:04X} = {:02X}",
                        base, index, addr, read_fn(*addr)
                    )
                }
            }

            OperandAddress::Indirect { ptr, target } => {
                format!("(${:04X}) = {:04X}", ptr, target)
            }

            OperandAddress::IndirectX { zp, ptr, addr } => {
                format!(
                    "(${:02X},X) @ {:02X} = {:04X} = {:02X}",
                    zp, ptr, addr, read_fn(*addr)
                )
            }

            OperandAddress::IndirectY { zp, base, addr } | OperandAddress::IndirectYPageCrossed { zp, base, addr } => {
                format!(
                    "(${:02X}),Y = {:04X} @ {:04X} = {:02X}",
                    zp, base, addr, read_fn(*addr)
                )
            }
        }
    }
}

impl fmt::Display for Operand {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Operand::None => write!(f, ""),
            Operand::Byte(v) => write!(f, "#${:02X}", v),
            Operand::Address(addr) => write!(f, "(${:04X})", addr),
        }
    }
}

impl From<OperandAddress> for Operand {
    fn from(addr: OperandAddress) -> Self {
        addr.to_operand()
    }
}

#[cfg(test)]
mod operand_tests {
    use super::*;

    #[test]
    fn test_to_operand_none() {
        let operand = OperandAddress::None.to_operand();
        assert!(matches!(operand, Operand::None));
        assert_eq!(operand.as_address(), None);
    }

    #[test]
    fn test_to_operand_immediate() {
        let operand = OperandAddress::Immediate(0x42).to_operand();
        assert!(matches!(operand, Operand::Byte(0x42)));
        assert_eq!(operand.as_address(), None);
    }

    #[test]
    fn test_to_operand_address() {
        let operand = OperandAddress::Address(0x1234).to_operand();
        assert!(matches!(operand, Operand::Address(0x1234)));
        assert_eq!(operand.as_address(), Some(0x1234));
    }

    #[test]
    fn test_to_operand_with_base() {
        let operand = OperandAddress::AddressWithBase {
            base: 0x2000,
            index: 'X',
            addr: 0x2010,
        }.to_operand();
        assert!(matches!(operand, Operand::Address(0x2010)));
        assert_eq!(operand.as_address(), Some(0x2010));
    }

    #[test]
    fn test_to_operand_indirect() {
        let operand = OperandAddress::Indirect {
            ptr: 0x00FF,
            target: 0x1234,
        }.to_operand();
        assert!(matches!(operand, Operand::Address(0x1234)));
        assert_eq!(operand.as_address(), Some(0x1234));
    }

    #[test]
    fn test_to_operand_indirect_x() {
        let operand = OperandAddress::IndirectX {
            zp: 0x20,
            ptr: 0x25,
            addr: 0x3000,
        }.to_operand();
        assert!(matches!(operand, Operand::Address(0x3000)));
        assert_eq!(operand.as_address(), Some(0x3000));
    }

    #[test]
    fn test_to_operand_indirect_y() {
        let operand = OperandAddress::IndirectY {
            zp: 0x30,
            base: 0x4000,
            addr: 0x4050,
        }.to_operand();
        assert!(matches!(operand, Operand::Address(0x4050)));
        assert_eq!(operand.as_address(), Some(0x4050));
    }

    #[test]
    fn test_as_address_returns_none_for_byte() {
        let operand = Operand::Byte(0xFF);
        assert_eq!(operand.as_address(), None);
    }

    #[test]
    fn test_as_address_returns_none_for_none() {
        let operand = Operand::None;
        assert_eq!(operand.as_address(), None);
    }

    #[test]
    fn test_display_none() {
        let operand = Operand::None;
        assert_eq!(format!("{}", operand), "");
    }

    #[test]
    fn test_display_immediate() {
        let operand = Operand::Byte(0x42);
        assert_eq!(format!("{}", operand), "#$42");
    }

    #[test]
    fn test_display_immediate_single_digit() {
        let operand = Operand::Byte(0x05);
        assert_eq!(format!("{}", operand), "#$05");
    }

    #[test]
    fn test_display_address() {
        let operand = Operand::Address(0x1234);
        assert_eq!(format!("{}", operand), "($1234)");
    }

    #[test]
    fn test_display_address_with_base() {
        let operand = OperandAddress::AddressWithBase {
            base: 0x2000,
            index: 'X',
            addr: 0x2010,
        }.to_operand();
        assert_eq!(format!("{}", operand), "($2010)");
    }

    #[test]
    fn test_from_trait() {
        let operand: Operand = OperandAddress::Immediate(0x99).into();
        assert!(matches!(operand, Operand::Byte(0x99)));
    }

    #[test]
    fn test_from_trait_with_address() {
        let operand: Operand = OperandAddress::Address(0xABCD).into();
        assert_eq!(operand.as_address(), Some(0xABCD));
        assert_eq!(format!("{}", operand), "($ABCD)");
    }
}
