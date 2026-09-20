use super::{MaskFlags, Address, CtrlFlags, Memory};

#[derive(Default, Clone, Copy)]
pub struct PixelData {
    pub id: u8,
    pub attr: u8,
    pub lsb: u8,
    pub msb: u8,
}

#[derive(Default)]
pub struct Background {
    next_tile: PixelData,
    shifter: Shifter,
}

#[derive(Default)]
struct BitRegister {
    lo: u16,
    hi: u16,
}

#[derive(Default)]
pub struct Shifter {
    pattern: BitRegister,
    attr: BitRegister,
}

impl Background {
    pub fn reset(&mut self) { *self = Self::default(); }

    pub fn update(&mut self, 
        cycle: &usize, 
        scanline: &u16,
        ctrl: &CtrlFlags,
        _mask: &MaskFlags, 
        addr: &mut Address, 
        mem: &mut Memory)
    {
        if matches!(*cycle, 1..=256 | 321..=336) {
            self.update_tile_state(cycle, ctrl, addr, mem);
        }

        let is_visible_or_prerender = *scanline < 240 || *scanline == 261;

        match (cycle, scanline) {
            // End of visible scanline
            (256, _) if is_visible_or_prerender => { addr.increment_scroll_y(); }
            // Reset X position
            (257, _) if is_visible_or_prerender => {
                self.shifter.load(&self.next_tile);
                addr.transfer_address_x();
            }
            // Pre-render scanline: copy Y
            (280..=304, 261) => { addr.transfer_address_y(); }
            // Unused nametable fetches (for prefetch)
            (338 | 340, _) => { self.fetch_nametable_byte(addr, mem); }
            _ => {}
        }
    }

    pub fn get_pixel(&self, mask: &MaskFlags, cycle: &usize, addr: &mut Address) -> (u8, u8) {
        if !mask.contains(MaskFlags::ENABLE_BACKGROUND_RENDERING) {
            return (0, 0);
        }

        // Mask leftmost 8 pixels if needed
        if !mask.contains(MaskFlags::BACKGROUND_LEFTMOST) && *cycle <= 8 {
            return (0, 0);
        }

        self.shifter.get_pixel(addr.x)
    }

    fn update_tile_state(&mut self, cycle: &usize, ctrl: &CtrlFlags, addr: &mut Address, mem: &mut Memory) {
        // Fetch background tile data every 8 cycles
        match (cycle - 1) % 8 {
            0 => {
                self.shifter.load(&self.next_tile);
                self.fetch_nametable_byte(addr, mem);
            }
            2 => {
                self.fetch_attribute_byte(addr, mem);
            }
            4 => {
                self.fetch_tile_lsb(ctrl, addr, mem);
            }
            6 => {
                self.fetch_tile_msb(ctrl, addr, mem);
            }
            7 => {
                addr.increment_scroll_x();
            }
            _ => {}
        }
    }

    pub fn update_x(&mut self, cycle: usize, mask: &MaskFlags) {
        if !mask.contains(MaskFlags::ENABLE_BACKGROUND_RENDERING) {
            return;
        }
        if matches!(cycle, 1..=256 | 321..=336) {
            self.shifter.update();
        }
    }

    fn fetch_nametable_byte(&mut self, addr: &mut Address, mem: &mut Memory) {
        self.next_tile.id = mem.read(0x2000 | (addr.v & 0x0FFF));
    }

    fn fetch_attribute_byte(&mut self, addr: &mut Address, mem: &mut Memory) {
        let attribute = mem.read(
            0x23C0 |
            ((addr.v & 0x0C00) >> 0) |
            ((addr.v & 0x0380) >> 4) |
            ((addr.v & 0x001C) >> 2)
        );
        let shift = ((addr.v & 0x0002) | ((addr.v & 0x0040) >> 4)) as u8;
        self.next_tile.attr = (attribute >> shift) & 0x03;
    }

    fn fetch_tile_lsb(&mut self, ctrl: &CtrlFlags, addr: &mut Address, mem: &mut Memory) {
        let addr = ctrl.bg_pattern() + (self.next_tile.id as u16 * 16) + addr.fine_y();
        self.next_tile.lsb = mem.read(addr);
    }

    fn fetch_tile_msb(&mut self, ctrl: &CtrlFlags, addr: &mut Address, mem: &mut Memory){
        let addr = ctrl.bg_pattern() + (self.next_tile.id as u16 * 16) + addr.fine_y() + 8;
        self.next_tile.msb = mem.read(addr);
    }
}

impl BitRegister {
    #[inline]
    fn extract(&self, mask: u16) -> u8 {
        let bit0 = u8::from((self.lo & mask) != 0);
        let bit1 = u8::from((self.hi & mask) != 0);
        (bit1 << 1) | bit0
    }

    #[inline]
    fn shift_left(&mut self) {
        self.lo <<= 1;
        self.hi <<= 1;
    }

    #[inline]
    fn load_low_byte(&mut self, lo: u8, hi: u8) {
        self.lo = (self.lo & 0xFF00) | lo as u16;
        self.hi = (self.hi & 0xFF00) | hi as u16;
    }
}

impl Shifter {
    #[inline]
    pub fn update(&mut self) {
        self.pattern.shift_left();
        self.attr.shift_left();
    }

    #[inline]
    pub fn load(&mut self, tile: &PixelData) {
        self.pattern.load_low_byte(tile.lsb, tile.msb);
        let attrib_lo = if (tile.attr & 0b01) != 0 { 0xFF } else { 0x00 };
        let attrib_hi = if (tile.attr & 0b10) != 0 { 0xFF } else { 0x00 };
        self.attr.load_low_byte(attrib_lo, attrib_hi);
    }

    #[inline]
    pub fn get_pixel(&self, x: u8) -> (u8, u8) {
        let mask = 0x8000 >> x;
        let pixel = self.pattern.extract(mask);
        let palette = self.attr.extract(mask);
        (pixel, palette)
    }
}
