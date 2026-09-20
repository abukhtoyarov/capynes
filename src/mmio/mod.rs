use log::trace;
use crate::rom::RomRef;
use crate::ppu::Ppu;
use crate::apu::Apu;
use crate::ctrl::Controller;

use std::rc::Rc;
use std::cell::RefCell;
use std::cell::Cell;

// http://nesdev.com/NESDoc.pdf
/*
 *        +------------------------------------------------------+
 *        |                    CPU MEMORY MAP                    |
 *        +------------------------------------------------------+
 *
 *        Address Range                  Description
 *        --------------          -------------------------
 *
 *        $0000-$00FF    +---------------------------------------+
 *                       |               Zero Page               |
 *        $0100-$01FF    +---------------------------------------+
 *                       |                Stack                  |
 *        $0200-$07FF    +---------------------------------------+
 *                       |                 RAM                   |
 *        $0800-$0FFF    |              Mirrors of RAM           |
 *        $1000-$17FF    |              Mirrors of RAM           |
 *        $1800-$1FFF    +---------------------------------------+
 *
 *        $2000-$2007    +---------------------------------------+
 *                       |            I/O Registers              |
 *        $2008-$3FFF    |     Mirrors of $2000–$2007 (every 8)  |
 *                       +---------------------------------------+
 *
 *        $4000-$401F    +---------------------------------------+
 *                       |            I/O Registers              |
 *        $4020-$5FFF    +---------------------------------------+
 *                       |             Expansion ROM             |
 *        $6000-$7FFF    +---------------------------------------+
 *                       |                 SRAM                  |
 *        $8000-$BFFF    +---------------------------------------+
 *                       |         PRG-ROM Lower Bank            |
 *        $C000-$FFFF    +---------------------------------------+
 *                       |         PRG-ROM Upper Bank            |
 *                       +---------------------------------------+
 *
 */
pub trait MmioInterface: {
    fn read(&self, addr: u16) -> u8;
    fn read_range(&self, from: u16, to: u16, cbk: &mut dyn FnMut(usize, u8));
    fn write(&mut self, addr: u16, data: u8);
    fn poll_irq(&mut self) -> bool { false }
    fn peek(&self, addr: u16) -> u8 {
        self.read(addr)
    }
}

pub type Mmio = Box<dyn MmioInterface>;

pub struct NesMmio {
    pub vmem: [u8; 0x800],
    pub rom: RomRef,
    pub apu: Rc<RefCell<Apu>>,
    pub ppu: Rc<RefCell<Ppu>>,
    pub j1: Rc<RefCell<Controller>>,
    pub j2: Rc<RefCell<Controller>>,
    open_bus: Cell<u8>,
    cpu_bus_cycle: Cell<u64>,
    ppu_open_bus: Cell<u8>,
    ppu_open_bus_refresh: [Cell<u64>; 8],
}

#[derive(Debug, PartialEq, Eq)]
enum MemoryRegion {
    ZeroPage,
    Stack,
    Ram,
    RamMirror,
    PpuRegister,
    PpuMirror,
    OtherIoRegisters,
    UnallocatedIo,
    ExpansionRom,
    Sram,
    PrgRom,
}

impl MemoryRegion {
    fn range(addr: u16) -> MemoryRegion {
        match addr {
            0x0000..=0x00FF => MemoryRegion::ZeroPage,
            0x0100..=0x01FF => MemoryRegion::Stack,
            0x0200..=0x07FF => MemoryRegion::Ram,
            0x0800..=0x1FFF => MemoryRegion::RamMirror,
            0x2000..=0x2007 => MemoryRegion::PpuRegister,
            0x2008..=0x3FFF => MemoryRegion::PpuMirror,
            0x4000..=0x4017 => MemoryRegion::OtherIoRegisters,
            0x4018..=0x40FF => MemoryRegion::UnallocatedIo,
            0x4100..=0x5FFF => MemoryRegion::ExpansionRom,
            0x6000..=0x7FFF => MemoryRegion::Sram,
            0x8000..=0xFFFF => MemoryRegion::PrgRom,
        }
    }
}

impl NesMmio {
    pub fn new(
        rom: RomRef, 
        apu: Rc<RefCell<Apu>>, 
        ppu: Rc<RefCell<Ppu>>,
        j1: Rc<RefCell<Controller>>,
        j2: Rc<RefCell<Controller>>,
        ) -> Box<dyn MmioInterface>
    {
        Box::new(NesMmio {
            vmem: [0; 0x800],
            rom,
            apu,
            ppu,
            j1,
            j2,
            open_bus: Cell::new(0),
            cpu_bus_cycle: Cell::new(0),
            ppu_open_bus: Cell::new(0),
            ppu_open_bus_refresh: std::array::from_fn(|_| Cell::new(0)),
        })
    }

    fn ppu_open_bus_value(&self) -> u8 {
        const PPU_OPEN_BUS_DECAY_CYCLES: u64 = 3_000_000;

        let now = self.ppu.borrow().absolute_cycle();
        let mut value = self.ppu_open_bus.get();

        for bit in 0..8 {
            if now.saturating_sub(self.ppu_open_bus_refresh[bit].get()) >= PPU_OPEN_BUS_DECAY_CYCLES {
                value &= !(1 << bit);
            }
        }

        value
    }

    fn refresh_ppu_open_bus(&self, value: u8, refresh_mask: u8) {
        let now = self.ppu.borrow().absolute_cycle();
        let mut bus = self.ppu_open_bus.get();

        for bit in 0..8 {
            let mask = 1 << bit;
            if refresh_mask & mask != 0 {
                if value & mask != 0 {
                    bus |= mask;
                } else {
                    bus &= !mask;
                }
                self.ppu_open_bus_refresh[bit].set(now);
            }
        }

        self.ppu_open_bus.set(bus);
    }

    fn read_ppu_register(&self, addr: u16) -> u8 {
        let register = addr & 0x2007;
        trace!("Read PPU register: ${:04X}", register);

        let (value, refresh_mask) = match register {
            0x2002 => {
                let open_bus = self.ppu_open_bus_value();
                (self.ppu.borrow_mut().read_status_with_open_bus(open_bus), 0xE0)
            }
            0x2004 => (self.ppu.borrow_mut().read_oam_data(), 0xFF),
            0x2007 => {
                let open_bus = self.ppu_open_bus_value();
                let refresh_mask = self.ppu.borrow().data_read_refresh_mask();
                let mut value = self.ppu.borrow_mut().read();
                if refresh_mask == 0x3F {
                    value = (value & 0x3F) | (open_bus & 0xC0);
                }
                (value, refresh_mask)
            }
            _ => {
                trace!("Read from write-only PPU register ${:04X}", register);
                (self.ppu_open_bus_value(), 0x00)
            }
        };

        self.refresh_ppu_open_bus(value, refresh_mask);
        self.open_bus.set(value);
        value
    }

    fn write_ppu_register(&mut self, addr: u16, data: u8) {
        let register = addr & 0x2007;
        trace!("Write PPU register: ${:04X} = ${:02X}", register, data);
        self.open_bus.set(data);
        self.refresh_ppu_open_bus(data, 0xFF);

        match register {
            0x2000 => self.ppu.borrow_mut().ctrl(data),
            0x2001 => self.ppu.borrow_mut().write_mask(data),
            0x2003 => self.ppu.borrow_mut().write_oam_addr(data),
            0x2004 => self.ppu.borrow_mut().write_oam_data(data),
            0x2005 => self.ppu.borrow_mut().write_scroll(data),
            0x2006 => self.ppu.borrow_mut().write_addr(data),
            0x2007 => self.ppu.borrow_mut().write(data),
            _ => {}
        }
    }

    fn read_from_reg(&self, addr: u16) -> u8 {
        trace!("Read register: ${:04X}", addr);
        
        match addr {
            0x4015 => self.apu.borrow_mut().read_status(),
            0x4016 => self.j1.borrow_mut().read(),
            0x4017 => self.j2.borrow_mut().read(),
            _ => {
                trace!("Read from write-only APU register ${:04X}", addr);
                self.open_bus.get()
            }
        }
    }

    fn write_to_reg(&mut self, addr: u16, data: u8) {
        trace!("Write register: ${:04X} = ${:02X}", addr, data);
        
        let mut apu = self.apu.borrow_mut();
        match addr {
            // Pulse 1
            0x4000 => apu.write_pulse1_ctrl(data),
            0x4001 => apu.write_pulse1_sweep(data),
            0x4002 => apu.write_pulse1_timer_low(data),
            0x4003 => apu.write_pulse1_length(data),
            
            // Pulse 2
            0x4004 => apu.write_pulse2_ctrl(data),
            0x4005 => apu.write_pulse2_sweep(data),
            0x4006 => apu.write_pulse2_timer_low(data),
            0x4007 => apu.write_pulse2_length(data),
            
            // Triangle
            0x4008 => apu.write_triangle_ctrl(data),
            0x4009 => {}, // Unused
            0x400A => apu.write_triangle_timer_low(data),
            0x400B => apu.write_triangle_length(data),
            
            // Noise
            0x400C => apu.write_noise_ctrl(data),
            0x400D => {}, // Unused
            0x400E => apu.write_noise_period(data),
            0x400F => apu.write_noise_length(data),
            
            // DMC
            0x4010 => apu.write_dmc_ctrl(data),
            0x4011 => apu.write_dmc_output(data),
            0x4012 => apu.write_dmc_address(data),
            0x4013 => apu.write_dmc_length(data),
            
            // OAM DMA
            0x4014 => {
                log::trace!("OAM DMA: ${:02X}00-${:02X}FF -> PPU OAM", data, data);
                let base = (data as u16) << 8;
                let mut bytes = [0u8; 256];
                for (i, byte) in bytes.iter_mut().enumerate() {
                    *byte = self.read(base + i as u16);
                }
                self.ppu.borrow_mut().write_oam_dma_bytes(&bytes);
            }
            
            // Status
            0x4015 => apu.write_status(data),
            
            // Strobe
            0x4016 => {
                self.j1.borrow_mut().write(data);
                self.j2.borrow_mut().write(data);
            }

            // Frame counter
            0x4017 => apu.write_frame_counter(data),
            
            _ => {
                trace!("Write to unused APU/IO register ${:04X}", addr);
            }
        }
    }
}

impl MmioInterface for NesMmio {
    fn read(&self, addr: u16) -> u8
    {
        self.cpu_bus_cycle
            .set(self.cpu_bus_cycle.get().saturating_add(1));
        trace!("Read from {:04X}: {:?}", addr, MemoryRegion::range(addr));
        let value = match MemoryRegion::range(addr) {
            MemoryRegion::ZeroPage | MemoryRegion::Stack | MemoryRegion::Ram => {
                self.vmem[addr as usize]
            },
            MemoryRegion::RamMirror => {
                self.vmem[addr as usize & 0x7FF]
            },
            MemoryRegion::PpuRegister => {
                self.read_ppu_register(addr)
            },
            MemoryRegion::PpuMirror => {
                self.read_ppu_register(addr)
            },
            MemoryRegion::OtherIoRegisters => {
                self.read_from_reg(addr)
            },
            MemoryRegion::UnallocatedIo => {
                self.open_bus.get()
            }
            MemoryRegion::ExpansionRom => {
                self.rom.mapper.borrow_mut().read_prg(addr)
            },
            MemoryRegion::Sram => {
                self.rom.mapper.borrow_mut().read_prg(addr)
            },
            MemoryRegion::PrgRom => {
                self.rom.mapper.borrow_mut().read_prg(addr)
            },
        };

        self.open_bus.set(value);
        value
    }

    fn read_range(&self, from: u16, to: u16, cbk: &mut dyn FnMut(usize, u8)) {
        let from = from as usize;
        let to = to as usize;
        if to >= 0x2000 {
            panic!("Unexpected range for read: {:04X}..{:04X}", from, to);
        }

        self.vmem[from..to].iter().enumerate().for_each(|(i, b)| cbk(i, *b));
    }

    fn write(&mut self, addr: u16, data: u8)
    {
        self.cpu_bus_cycle
            .set(self.cpu_bus_cycle.get().saturating_add(1));
        let cpu_bus_cycle = self.cpu_bus_cycle.get();
        trace!("Write to {:04X}: {:?}", addr, MemoryRegion::range(addr));
        self.open_bus.set(data);
        match MemoryRegion::range(addr) {
            MemoryRegion::ZeroPage | MemoryRegion::Stack | MemoryRegion::Ram => {
                self.vmem[addr as usize] = data
            },
            MemoryRegion::RamMirror => {
                self.vmem[addr as usize & 0x7FF] = data
            },
            MemoryRegion::PpuRegister => {
                self.write_ppu_register(addr, data)
            },
            MemoryRegion::PpuMirror => {
                self.write_ppu_register(addr, data)
            },
            MemoryRegion::OtherIoRegisters => {
                self.write_to_reg(addr, data)
            },
            MemoryRegion::UnallocatedIo => {
                trace!("Write to unallocated I/O register ${:04X}", addr);
            }
            MemoryRegion::ExpansionRom => {
                self.rom
                    .mapper
                    .borrow_mut()
                    .write_prg_with_cycle(addr, data, cpu_bus_cycle);
            },
            MemoryRegion::Sram => {
                self.rom
                    .mapper
                    .borrow_mut()
                    .write_prg_with_cycle(addr, data, cpu_bus_cycle);
            },
            MemoryRegion::PrgRom => {
                self.rom
                    .mapper
                    .borrow_mut()
                    .write_prg_with_cycle(addr, data, cpu_bus_cycle);
            },
        }
    }

    fn poll_irq(&mut self) -> bool {
        if self.apu.borrow().get_irq() {
            return true;
        }
        self.rom.mapper.borrow_mut().poll_irq()
    }

}
