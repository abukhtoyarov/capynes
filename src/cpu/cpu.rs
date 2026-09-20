use crate::cpu::asm::{Addressing, Instruction, INSTRUCTIONS};
use crate::cpu::op::OperandContext;
use crate::cpu::memory::MemIO;
use crate::mmio::Mmio;
use super::stack::StackOps;
use bitflags::bitflags;
use log::{trace, debug};
use std::fmt;

bitflags! {
    #[derive(Debug, PartialEq, Clone)]
    pub struct CpuFlags: u8 {
        const CARRY = (1 << 0);
        const ZERO = (1 << 1);
        const INTERRUPT_DISABLE = (1 << 2);
        const DECIMAL_MODE = (1 << 3);
        const BREAK = (1 << 4);
        const BREAK2 = (1 << 5);
        const OVERFLOW = (1 << 6);
        const NEGATIV = (1 << 7);
    }
}

#[derive(Clone, Copy, Debug)]
pub enum InterruptType {
    BRK,
    NMI,
    IRQ
}

pub struct Interrupt {
    pub interrupt_type: InterruptType,
    pub addr: u16,
    pub flags: CpuFlags,
    pub cycles: u8,
}

pub const BRK_INT: Interrupt = Interrupt{
    interrupt_type: InterruptType::BRK,
    addr: 0xfffe,
    flags: CpuFlags::from_bits_truncate(0b00110000),
    cycles: 0
};

pub const NMI_INT: Interrupt = Interrupt{
    interrupt_type: InterruptType::NMI,
    addr: 0xfffa,
    flags: CpuFlags::BREAK2,
    cycles: 7
};

pub const IRQ_INT: Interrupt = Interrupt {
    interrupt_type: InterruptType::IRQ,
    addr: 0xfffe,
    flags: CpuFlags::BREAK2,
    cycles: 7
};

pub struct Cpu {
    pub pc: u16,
    pub sp: u8,
    pub acc: u8,
    pub rx: u8,
    pub ry: u8,
    pub flags: CpuFlags,
    pub bus: Mmio,
    pub cycles: usize,
    pub stall: usize,
    pub last_opcode: u8,
    irq_inhibit_override: Option<bool>,
}

impl Cpu {
    fn new(bus: Option<Mmio>) -> Self {
        debug!("Initializing CPU...");
        return Cpu {
            pc: 0,
            sp: 0xfd,
            acc: 0,
            rx: 0,
            ry: 0,
            flags: CpuFlags::empty(),
            bus: bus.unwrap(),
            cycles: 0,
            stall: 0,
            last_opcode: 0,
            irq_inhibit_override: None,
        };
    }

    pub fn with_bus(bus: Mmio) -> Self {
        Cpu::new(Some(bus))
    }

    pub fn reset(&mut self) {
        self.acc = 0;
        self.rx = 0;
        self.ry = 0;
        self.sp = 0xfd;
        self.pc = self.read::<u16>(0xFFFC);
        self.flags = CpuFlags::from_bits_truncate(0b100100);
        self.cycles = 0;
        self.stall = 0;
        self.last_opcode = 0;
        self.irq_inhibit_override = None;
    }

    pub fn console_reset(&mut self) {
        self.sp = self.sp.wrapping_sub(3);
        self.flags.insert(CpuFlags::INTERRUPT_DISABLE);
        self.pc = self.read::<u16>(0xFFFC);
        self.stall = 0;
        self.last_opcode = 0;
        self.irq_inhibit_override = None;
    }

    pub fn load_program(&mut self, addr: u16, program: &Vec<u8>) -> u16 {
        debug!("Loading program: {:02X?}", &program);

        let mut pc = addr;
        for &byte in program {
            self.write(pc, byte);
            pc = pc.wrapping_add(1);
        }

        self.write(0xFFFC, addr);
        addr + program.len() as u16
    }

    pub fn step(&mut self) -> usize {
        if self.stall > 0 {
            self.stall -= 1;
            self.cycles += 1;
            return 1;
        }

        self.trace_cpu_state();
        self.last_opcode = self.read::<u8>(self.pc);
        let (ins, mut operand) = self.read_instruction();
        let c = ins.cycles.clone();
        (ins.invoke)(self, &mut operand);

        let passed_cycles = c.wrapping_add(operand.extra_cycles) as usize;
        self.cycles = self.cycles.wrapping_add(passed_cycles);
        passed_cycles
    }
    
    pub fn read<T: MemIO>(&self, addr: u16) -> T
    {
        T::read(self, addr)
    }

    pub fn write<T: MemIO>(&mut self, addr: u16, data: T)
    {
        if addr == 0x4014 {
            self.stall += if self.cycles % 2 == 0 { 513 } else { 514 };
        }
        data.write(self, addr)
    }

    fn read_instruction(&mut self) -> (&Instruction, OperandContext) {
        let op = self.read::<u8>(self.pc);
        
        let ins = &INSTRUCTIONS[op as usize];
        let (operand, page_cross) = ins.fetch_operand(self);
        self.pc = self.pc.wrapping_add(ins.len as u16);

        (ins, OperandContext{ 
            value: operand, 
            extra_cycles: if page_cross { 1 } else { 0 } })
    }

    #[allow(dead_code)] pub fn run(&mut self, end: u16) {
        debug!("Running program... {:02X}", end);

        while self.pc < end {
            self.step();
        }
    }

    #[allow(dead_code)] pub fn run_program(&mut self, program: Vec<u8>) {
        let end = self.load_program(0x8000, &program);
        self.reset();
        self.run(end);
    }

    pub fn interrupt(&mut self, int: &Interrupt) -> u8 {
        self.push(self.pc);

        let mut flg = self.flags.clone();
        flg.set(CpuFlags::BREAK, matches!(int.interrupt_type, InterruptType::BRK));
        // Bit 5 is always set when pushing flags
        flg.insert(CpuFlags::BREAK2);
        self.push(flg.bits());
        self.flags.insert(CpuFlags::INTERRUPT_DISABLE);

        self.cycles = self.cycles.wrapping_add(int.cycles as usize);
        self.pc = self.read(int.addr);
        int.cycles
    }

    pub fn nmi_interrupt(&mut self) -> usize {
        self.interrupt(&NMI_INT) as usize
    }

    pub fn irq_interrupt(&mut self) -> usize {
        self.interrupt(&IRQ_INT) as usize
    }

    pub fn hijack_interrupt_vector_to_nmi(&mut self) {
        self.pc = self.read::<u16>(NMI_INT.addr);
    }

    pub fn irq_inhibited(&self) -> bool {
        self.irq_inhibit_override
            .unwrap_or_else(|| self.flags.contains(CpuFlags::INTERRUPT_DISABLE))
    }

    pub fn delay_irq_flag_effect(&mut self, previous_i_flag: bool) {
        self.irq_inhibit_override = Some(previous_i_flag);
    }

    pub fn finish_interrupt_poll(&mut self) {
        self.irq_inhibit_override = None;
    }

    fn trace_cpu_state(&self) {
        trace!("{}", self);
    }

    fn peek_for_trace(&self, addr: u16) -> u8 {
        match addr {
            // RAM (включая зеркала)
            0x0000..=0x1FFF => {
                self.bus.read(addr)
            },
            0x8000..=0xFFFF => {
                self.bus.read(addr)
            },
            _ => 0xff,
        }
    }
}

impl fmt::Display for Cpu {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let pc = self.pc;
        let op_code = self.read::<u8>(pc);
        let instr = &INSTRUCTIONS[op_code as usize];

        // 1. Собираем байты (Hex дамп)
        let mut raw_bytes = Vec::new();
        raw_bytes.push(op_code);
        for i in 1..instr.len {
            raw_bytes.push(self.read::<u8>(pc.wrapping_add(i as u16)));
        }

        let hex_str = raw_bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ");

        let operand_addr = instr.resolve_operand_address(self);

        let asm_op = match instr.mode {
            Addressing::Accumulator => "A".to_string(),
            _ => operand_addr.format_trace(|addr| self.peek_for_trace(addr), instr.cmd),
        };

        let asm = format!("{:04X}  {:8} {: >4} {}", pc, hex_str, instr.cmd, asm_op);

        let p_status = self.flags.bits() | 0x20;

        let result = format!(
            "{:47} A:{:02X} X:{:02X} Y:{:02X} P:{:02X} SP:{:02X}",
            asm, self.acc, self.rx, self.ry, p_status, self.sp
        );

        write!(f, "{}", result.to_uppercase())
    }
}

#[cfg(test)]
mod cpu_test {
    use std::sync::Once;
    use crate::mmio::MmioInterface;
    use crate::cpu::cpu::Cpu;
    use crate::cpu::cpu::CpuFlags;
    use crate::cpu::stack::StackOps;

    static INIT: Once = Once::new();

    pub fn setup() {
        INIT.call_once(|| {
            env_logger::builder()
                .is_test(true)
                .try_init()
                .ok();
        });
    }

    #[cfg(test)]
    #[ctor::ctor]
    fn init() {
        setup();
    }

    pub struct TestMmio {
        vmem: [u8; 0x10000]
    }

    impl TestMmio {
        pub fn new() -> Self {
            TestMmio { vmem: [0;0x10000] }
        }

        pub fn create_ptr() -> Box<dyn MmioInterface> {
            Box::new(TestMmio::new())
        }
    }

    impl MmioInterface for TestMmio {
        fn read(&self, addr: u16) -> u8
        {
            self.vmem[addr as usize]
        }
        fn read_range(&self, from: u16, to: u16, cbk: &mut dyn FnMut(usize, u8)) {
            let from = from as usize;
            let to = to as usize;
            self.vmem[from..to].iter().enumerate().for_each(|(i, b)| cbk(i, *b));
        }
        fn write(&mut self, addr: u16, data: u8)
        {
            self.vmem[addr as usize] = data;
        }
    }

    #[test]
    fn simple_program_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        cpu.run_program(vec![0xa9, 0xc0, 0xaa, 0xe8, 0xea]);

        assert_eq!(cpu.pc, 0x8005);
        assert_eq!(cpu.sp, 0xfd);
        assert_eq!(cpu.acc, 0xc0);
        assert_eq!(cpu.rx, 0xc1);
        assert_eq!(cpu.ry, 0x00);
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
    }

    #[test]
    fn zero_acc_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        cpu.acc = 0xff;
        cpu.run_program(vec![0xa9, 0x00, 0xea]);

        assert_eq!(cpu.acc, 0x00);
        assert!(cpu.flags.contains(CpuFlags::ZERO));
    }

    #[test]
    fn rx_overflow_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xe8, 0xe8, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.rx = 0xff;
        cpu.run(end);

        assert_eq!(cpu.rx, 0x01);
        assert_eq!(cpu.flags.bits(), 0);
    }

    #[test]
    fn mem_rw_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        cpu.write(0xFFFC, 0x8000 as u16);
        cpu.reset();
        assert_eq!(cpu.pc, 0x8000);
    }

    #[test]
    fn sec_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x38, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.run(end);

        assert_eq!(cpu.flags, CpuFlags::CARRY);
    }

    #[test]
    fn bpl_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x10, 0x02, 0xFF, 0xFF, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.run(end);

        assert_eq!(cpu.flags.bits(), 0);
        assert_eq!(cpu.pc, 0x8005);
    }

    #[test]
    fn dex_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xCA, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.run(end);

        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert_eq!(cpu.rx, 0xFF);
    }

    #[test]
    fn bit_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x24, 0x10, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 2;
        cpu.write(0x10, 0b11000000 as u8);
        cpu.run(end);

        assert_eq!(
            cpu.flags,
            CpuFlags::ZERO | CpuFlags::OVERFLOW | CpuFlags::NEGATIV
        );
    }

    #[test]
    fn bcs_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xb0, 0x02, 0xFF, 0xFF, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.flags, CpuFlags::CARRY);
        assert_eq!(cpu.pc, 0x8005);
    }

    #[test]
    fn ora_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x0d, 0x00, 0x20, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.write(0x2000, 0b11000000 as u8);
        cpu.acc = 128;

        cpu.run(end);

        assert!(cpu.flags.contains(CpuFlags::NEGATIV));
        assert_eq!(cpu.acc, 192);
    }

    #[test]
    fn clc_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x38, 0x18, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.run(end);

        assert_eq!(cpu.flags.bits(), 0);
    }

    #[test]
    fn tay_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xA8, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.acc = 192;
        cpu.run(end);

        assert_eq!(cpu.flags, CpuFlags::NEGATIV);
        assert_eq!(cpu.acc, 192);
        assert_eq!(cpu.ry, 192);
    }

    #[test]
    fn tya_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x98, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.ry = 192;
        cpu.run(end);

        assert_eq!(cpu.flags, CpuFlags::NEGATIV);
        assert_eq!(cpu.acc, 192);
        assert_eq!(cpu.ry, 192);
    }

    #[test]
    fn inc_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        // http://wiki.nesdev.com/w/index.php/CPU_status_flag_behavior
        let end = cpu.load_program(0x8000, &vec![0xee, 0x00, 0x20, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.write(0x2000, 0xFF as u8);
        cpu.run(end);

        assert_eq!(cpu.flags, CpuFlags::ZERO);
        assert_eq!(cpu.read::<u8>(0x2000), 0x00);
    }

    #[test]
    fn and_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x2D, 0x00, 0x20, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.acc = 255;
        cpu.write(0x2000, 128 as u8);
        cpu.run(end);

        assert_eq!(cpu.flags, CpuFlags::NEGATIV);
        assert_eq!(cpu.acc, 128);
    }

    #[test]
    fn sty_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x8c, 0x00, 0x20, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.ry = 123;
        cpu.run(end);

        assert_eq!(cpu.flags.bits(), 0);
        assert_eq!(cpu.read::<u8>(0x2000), 123);
    }

    #[test]
    fn txa_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x8A, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.rx = 129;
        cpu.run(end);

        assert_eq!(cpu.flags, CpuFlags::NEGATIV);
        assert_eq!(cpu.acc, 129);
    }

    #[test]
    fn bne_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xd0, 0x02, 0xff, 0xff, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.run(end);

        assert_eq!(cpu.flags.bits(), 0);
        assert_eq!(cpu.pc, 0x8005);
    }

    #[test]
    fn cmp_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xc9, 0x05, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x06;
        cpu.run(end);
        assert_eq!(cpu.flags, CpuFlags::CARRY);

        let end = cpu.load_program(0x8000, &vec![0xc9, 0x06, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x06;
        cpu.run(end);
        assert_eq!(
            cpu.flags,
            CpuFlags::CARRY | CpuFlags::ZERO
        );

        let end = cpu.load_program(0x8000, &vec![0xc9, 0x07, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x06;
        cpu.run(end);
        assert_eq!(cpu.flags, CpuFlags::NEGATIV);

        let end = cpu.load_program(0x8000, &vec![0xc9, 0x02, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0xff;
        cpu.run(end);
        assert_eq!(
            cpu.flags,
            CpuFlags::CARRY | CpuFlags::NEGATIV
        );

        let end = cpu.load_program(0x8000, &vec![0xc9, 0x90, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x06;
        cpu.run(end);
        assert_eq!(cpu.flags.bits(), 0);
    }

    #[test]
    fn pha_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x48]);
        cpu.reset();
        cpu.sp = 0xff;
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x06;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.sp, 0xfe);

        assert_eq!(cpu.read::<u8>(cpu.stack_addr() + cpu.sp as u16 + 1), 0x06);
    }

    #[test]
    fn pla_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xa9, 0xff, 0x48, 0xa9, 0xea, 0x68]);
        cpu.reset();
        cpu.sp = 0xff;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8006);
        assert_eq!(cpu.sp, 0xff);
        assert_eq!(cpu.acc, 0xff);
    }

    #[test]
    fn php_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x08]);
        cpu.reset();
        cpu.sp = 0xff;
        cpu.flags = CpuFlags::CARRY | CpuFlags::ZERO;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.sp, 0xfe);
        assert_eq!(cpu.flags.bits(), 0x03);
        assert_eq!(cpu.read::<u8>(cpu.stack_addr() + cpu.sp as u16 + 1), 0x33);
    }

    // TODO: check and restore test right way
    #[test]
    fn plp_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x38, 0xf8, 0x08, 0xd8, 0x18, 0x28]);
        cpu.reset();
        cpu.sp = 0xff;
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.run(end);

        assert_eq!(cpu.pc, 0x8006);
        assert_eq!(cpu.sp, 0xff);
        assert!(cpu.flags.contains(CpuFlags::DECIMAL_MODE | CpuFlags::CARRY | CpuFlags::BREAK2));
    }

    #[test]
    fn jsr_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x20, 0x05, 0x80, 0xff, 0xff, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.run(end);

        assert_eq!(cpu.pc, 0x8006);
        assert_eq!(cpu.sp, 0xfb);
        assert_eq!(cpu.read::<u16>(cpu.stack_addr() + cpu.sp as u16 + 1), 0x8002);
    }

    #[test]
    fn jmp_absolute_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x4c, 0x05, 0x80, 0xff, 0xff, 0xea]);
        cpu.reset();
        cpu.sp = 0xff;
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.run(end);

        assert_eq!(cpu.pc, 0x8006);
        assert_eq!(cpu.sp, 0xff);
    }

    #[test]
    fn jmp_indirect_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x6c, 0x03, 0x22, 0xff, 0xff, 0xea]);
        cpu.write(0x2203, 0x8005 as u16);
        cpu.reset();
        cpu.sp = 0xff;
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8006);
        assert_eq!(cpu.sp, 0xff);
    }

    #[test]
    fn txs_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0x9a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.rx = 0xff;
        cpu.flags = CpuFlags::CARRY;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.sp, 0xff);
        assert_eq!(cpu.flags, CpuFlags::CARRY);
    }

    #[test]
    fn tsx_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());
        let end = cpu.load_program(0x8000, &vec![0xba]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.rx = 0xff;
        cpu.sp = 0xff;
        cpu.flags = CpuFlags::CARRY;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.sp, 0xff);
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::NEGATIV);

        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.rx = 0xff;
        cpu.sp = 0x00;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.sp, 0x00);
        assert_eq!(cpu.flags, CpuFlags::ZERO);
    }

    #[test]
    fn asl_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x0a]);
        cpu.reset();
        cpu.acc = 0b10010100;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b00101000 );
        assert!(cpu.flags.contains(CpuFlags::CARRY));

        let end = cpu.load_program(0x8000, &vec![0x0a, 0x0a, 0x0a]);
        cpu.reset();
        cpu.acc = 0b10010100;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.acc, 0b10100000 );
        assert!(cpu.flags.contains(CpuFlags::NEGATIV));

        let end = cpu.load_program(0x8000, &vec![0x0a]);
        cpu.reset();
        cpu.acc = 0b10000000;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b00000000 );
        assert!(cpu.flags.contains(CpuFlags::CARRY | CpuFlags::ZERO));

        let end = cpu.load_program(0x8000, &vec![0x06, 0x10]);
        cpu.reset();
        cpu.write(0x10, 0b10000001 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b00000010 );
        assert!(cpu.flags.contains(CpuFlags::CARRY));

        let end = cpu.load_program(0x8000, &vec![0x06, 0x10]);
        cpu.reset();
        cpu.write(0x10, 0b10000000 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b00000000 );
        assert!(cpu.flags.contains(CpuFlags::CARRY | CpuFlags::ZERO));
    }

    #[test]
    fn lsr_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x4a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b10010100;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b01001010 );
        assert_eq!(cpu.flags.bits(), 0 );

        let end = cpu.load_program(0x8000, &vec![0x4a, 0x4a, 0x4a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b10010100;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.acc, 0b00010010 );
        assert_eq!(cpu.flags, CpuFlags::CARRY );

        let end = cpu.load_program(0x8000, &vec![0x4a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b00000001;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b00000000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::ZERO );

        let end = cpu.load_program(0x8000, &vec![0x46, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b10000001 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b01000000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY );

        let end = cpu.load_program(0x8000, &vec![0x46, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b01000001 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b00100000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY );

        let end = cpu.load_program(0x8000, &vec![0x46, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b00000001 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b00000000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::ZERO );
    }

    #[test]
    fn rol_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x2a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b10010100;
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b00101001 );
        assert_eq!(cpu.flags, CpuFlags::CARRY );

        let end = cpu.load_program(0x8000, &vec![0x2a, 0x2a, 0x2a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b10010100;
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.acc, 0b10100110 );
        assert_eq!(cpu.flags, CpuFlags::NEGATIV );

        let end = cpu.load_program(0x8000, &vec![0x2a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b10000000;
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b00000001 );
        assert_eq!(cpu.flags, CpuFlags::CARRY );

        let end = cpu.load_program(0x8000, &vec![0x26, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b10000001 as u8);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b00000011 );
        assert_eq!(cpu.flags, CpuFlags::CARRY );

        let end = cpu.load_program(0x8000, &vec![0x26, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b01000001 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b10000010 );
        assert_eq!(cpu.flags, CpuFlags::NEGATIV );

        let end = cpu.load_program(0x8000, &vec![0x26, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b10000000 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b00000000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::ZERO);
    }

    #[test]
    fn ror_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x6a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b10010100;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b01001010 );
        assert_eq!(cpu.flags.bits(), 0 );

        let end = cpu.load_program(0x8000, &vec![0x6a, 0x6a, 0x6a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b10010100;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.acc, 0b00010010 );
        assert_eq!(cpu.flags, CpuFlags::CARRY );

        let end = cpu.load_program(0x8000, &vec![0x6a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b00000001;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8001);
        assert_eq!(cpu.acc, 0b00000000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::ZERO);

        let end = cpu.load_program(0x8000, &vec![0x66, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b10000001 as u8);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b11000000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::NEGATIV );

        let end = cpu.load_program(0x8000, &vec![0x66, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b01000001 as u8);
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b10100000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::NEGATIV);

        let end = cpu.load_program(0x8000, &vec![0x66, 0x10]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.write(0x10, 0b00000001 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.read::<u8>(0x10), 0b00000000 );
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::ZERO);
    }

    #[test]
    fn rti_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x40, 0xff, 0xff, 0xff, 0xff, 0xea]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);

        cpu.push(0x8005 as u16);

        cpu.push((CpuFlags::CARRY | CpuFlags::ZERO | CpuFlags::BREAK).bits());

        cpu.flags = CpuFlags::from_bits(0).expect("Cannot parse flags from stack");

        cpu.run(end);

        assert_eq!(cpu.pc, 0x8006);
        assert_eq!(cpu.flags, CpuFlags::CARRY | CpuFlags::ZERO | CpuFlags::BREAK2);
    }

    #[test]
    fn rts_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x60, 0xff, 0xff, 0xff, 0xff, 0xea]);
        cpu.reset();
        cpu.push(0x8005 as u16);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8006);
    }

    #[test]
    fn sbc_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0xe9, 0x02]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x10;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0x0d);
        assert_eq!(cpu.flags, CpuFlags::CARRY);

        let end = cpu.load_program(0x8000, &vec![0xe9, 0x03]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x02;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0xfe);
        assert_eq!(cpu.flags, CpuFlags::NEGATIV);

        let end = cpu.load_program(0x8000, &vec![0xe9, 0xb0]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x50;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0x9f);
        assert_eq!(cpu.flags, CpuFlags::NEGATIV | CpuFlags::OVERFLOW);
    }

    #[test]
    fn adc_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x69, 0x02]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x10;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0x12);

        let end = cpu.load_program(0x8000, &vec![0x69, 0x7f]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x81;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0x0);
        assert!(cpu.flags.contains(CpuFlags::CARRY | CpuFlags::ZERO));

        let end = cpu.load_program(0x8000, &vec![0x69, 0x8a]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x8a;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0x14);
        assert!(cpu.flags.contains(CpuFlags::CARRY | CpuFlags::OVERFLOW));
    }

    #[test]
    fn brk_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x100,
            &vec![0x00, 0x00, 0x4c, 0x10, 0x01, 0xff, 0xff, 0xff, 0xff, 0xff, 0xa2, 0x05, 0x40]);
        cpu.reset();
        cpu.sp = 0xff;
        cpu.flags.remove(CpuFlags::INTERRUPT_DISABLE);
        cpu.write(0xfffe, 0x10A as u16);
        cpu.run(end);

        assert_eq!(cpu.rx, 0x05);
    }

    #[test]
    fn anc_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x0b, 0b1010_1010]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b1111_0000;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0b1010_0000);
        assert_eq!(cpu.flags, CpuFlags::NEGATIV | CpuFlags::CARRY);
    }

    #[test]
    fn alr_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x4b, 0b1111_1111]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b0000_0011;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0b0000_0001);
        assert_eq!(cpu.flags, CpuFlags::CARRY);
    }

    #[test]
    fn arr_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x6b, 0b1111_1111]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0b1100_0000;
        cpu.flags.insert(CpuFlags::CARRY);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.acc, 0b1110_0000);
        assert_eq!(cpu.flags, CpuFlags::NEGATIV | CpuFlags::CARRY);
    }

    #[test]
    fn axs_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0xcb, 0x05]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.acc = 0x0f;
        cpu.rx = 0x0a;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8002);
        assert_eq!(cpu.rx, 0x05);
        assert_eq!(cpu.flags, CpuFlags::CARRY);
    }

    #[test]
    fn las_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0xbb, 0x00, 0x20]);
        cpu.reset();
        cpu.flags = CpuFlags::from_bits_truncate(0);
        cpu.sp = 0b1111_0000;
        cpu.write(0x2000, 0b10101010 as u8);
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.acc, 0b1010_0000);
        assert_eq!(cpu.rx, 0b1010_0000);
        assert_eq!(cpu.sp, 0b1010_0000);
        assert_eq!(cpu.flags, CpuFlags::NEGATIV);
    }

    #[test]
    fn shx_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x9e, 0x00, 0x20]);
        cpu.reset();
        cpu.rx = 0xff;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.read::<u8>(0x2000), 0x21);
    }

    #[test]
    fn shy_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x9c, 0x00, 0x30]);
        cpu.reset();
        cpu.ry = 0xff;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.read::<u8>(0x3000), 0x31);
    }

    #[test]
    fn sha_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x9f, 0x00, 0x40]);
        cpu.reset();
        cpu.acc = 0xff;
        cpu.rx = 0x0f;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.read::<u8>(0x4000), 0x41 & 0x0f);
    }

    #[test]
    fn tas_test() {
        let mut cpu = Cpu::with_bus(TestMmio::create_ptr());

        let end = cpu.load_program(0x8000, &vec![0x9b, 0x00, 0x50]);
        cpu.reset();
        cpu.acc = 0xff;
        cpu.rx = 0x0f;
        cpu.run(end);

        assert_eq!(cpu.pc, 0x8003);
        assert_eq!(cpu.sp, 0x0f);

        let value = cpu.read::<u8>(0x5000);
        assert!(value == 0x00 || value == 0x01);
    }
}
