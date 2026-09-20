#[derive(Default, Debug, Clone)]
pub struct Address {
    // yyy NN YYYYY XXXXX
    // ||| || ||||| +++++-- coarse X (5 bits)
    // ||| || +++++-------- coarse Y (5 bits)
    // ||| ++-------------- nametable select (2 bits)
    // +++----------------- fine Y (3 bits)

    pub t: u16, // temporary VRAM address
    pub w: bool, // write latch
    pub x: u8, // fine X scroll (3-bit)
    pub v: u16 // current VRAM address (14-bit)
}

impl Address {
    const COARSE_X_MASK: u16 = 0b00000_00000_11111;
    const COARSE_Y_MASK: u16 = 0b00000_11111_00000;
    const NAMETABLE_MASK: u16 = 0b00011_00000_00000;
    const FINE_Y_MASK: u16 = 0b11100_00000_00000;

    const NAMETABLE_SHIFT: u16 = 10;
    const FINE_Y_SHIFT: u16 = 12;

    pub fn write_ppuaddr(&mut self, value: u8) {
        // Write to PPUADDR ($2006)
        //
        // First write: high byte (6 bits)
        // Second write: low byte
        // After second write: v = t
        if !self.w {
            // first write: high byte (6 bits)
            self.t = (self.t & 0x00FF) | ((value as u16 & 0x3F) << 8);
        } else {
            // second write: low byte
            self.t = (self.t & 0xFF00) | value as u16;
            self.v = self.t; // & 0x3FFF;
        }

        self.w = !self.w;
    }

    pub fn write_ppuscroll(&mut self, value: u8) {
        // Write to PPUSCROLL ($2005)
        //
        // First write:
        //   coarse X scroll + fine X
        // Second write:
        //   coarse Y scroll + fine Y
        if !self.w {
            self.x = value & 0x07;
            self.t = (self.t & !Self::COARSE_X_MASK) 
                    | ((value as u16) >> 3);
        } else {
            self.t = (self.t & !Self::FINE_Y_MASK) 
                | (((value as u16) & 0x07) << Self::FINE_Y_SHIFT);
            self.t = (self.t & !Self::COARSE_Y_MASK)
                | (((value as u16) & 0xF8) << 2);
        }

        self.w = !self.w;
    }

    pub fn write_ppuctrl(&mut self, value: u8) {
        // Write to PPUCTRL ($2000)
        //
        // Bits 0-1 select base nametable

        // Update nametable select bits in t
        self.t = (self.t & !Self::NAMETABLE_MASK)
            | (((value as u16) & 0x03) << Self::NAMETABLE_SHIFT);
    }

    pub fn reset(&mut self) {
        self.t = 0;
        self.v = 0;
        self.x = 0;
        self.w = false;
    }

    pub fn reset_latch(&mut self) { self.w = false; }

    pub fn fine_y(&self) -> u16 { (self.v >> 12) & 0x07 }

    pub fn increment_scroll_x(&mut self) {
        if (self.v & 0x001F) == 31 {
            self.v &= !0x001F;
            self.v ^= 0x0400;
        } else {
            self.v += 1;
        }
    }

    pub fn increment_scroll_y(&mut self) {
        if (self.v & 0x7000) != 0x7000 {
            self.v += 0x1000;
        } else {
            self.v &= !0x7000;
            let mut y = (self.v & 0x03E0) >> 5;

            if y == 29 {
                y = 0;
                self.v ^= 0x0800;
            } else if y == 31 {
                y = 0;
            } else {
                y += 1;
            }

            self.v = (self.v & !0x03E0) | (y << 5);
        }
    }

    pub fn transfer_address_x(&mut self) {
        self.v = (self.v & !0x041F) | (self.t & 0x041F);
    }

    pub fn transfer_address_y(&mut self) {
        self.v = (self.v & !0x7BE0) | (self.t & 0x7BE0);
    }
}
