use super::flags::{ StatusFlags, CtrlFlags };
use super::{ Memory, Oam };

#[derive(Default, Clone, Copy)]
struct SpriteData {
    id: u8,
    attr: u8,
    x: u8,
    y: u8,
}

#[derive(Default)]
struct Shifter {
    pattern_lo: u8,
    pattern_hi: u8,
}

#[derive(Default)]
struct Sprite {
    data: SpriteData,
    shifter: Shifter,
}

#[derive(Default)]
pub struct Sprites {
    sprites: [Sprite; 8],
    counter: usize,
    overflow_trigger_cycle: Option<usize>,
    pub zero_hit_possible: bool,
    pub zero_rendered: bool,
}

impl Sprites {
    pub fn reset(&mut self) { *self = Self::default(); }

    pub fn update(&mut self, cycle: &usize, scanline: &u16,
        ctrl: &CtrlFlags, status: &mut StatusFlags, oam: &Oam, mem: &mut Memory)
    {
        if *cycle == 65 {
            let next_scanline = if *scanline == 261 { 0 } else { scanline.wrapping_add(1) };
            self.overflow_trigger_cycle =
                self.compute_overflow_trigger_cycle(next_scanline, ctrl, oam);
        }

        if self.overflow_trigger_cycle == Some(*cycle) {
            status.set(StatusFlags::SPRITE_OVERFLOW, true);
        }

        // Sprite evaluation happens for the *next* scanline at cycle 257
        if *cycle == 257 {
            let next_scanline = if *scanline == 261 { 0 } else { scanline.wrapping_add(1) };
            self.evaluate(&next_scanline, ctrl, status, oam);
            self.load_sprite_shifters(&next_scanline, ctrl, mem);
        }
    }

    pub fn update_x(&mut self, cycle: usize) {
        self.tick_render(cycle);
    }

    pub fn get_pixel(&mut self) -> (u8, u8, bool) {
        self.zero_rendered = false;

        for i in 0..self.counter {
            if self.sprites[i].data.x == 0 {
                let pixel = self.sprites[i].shifter.get_pixel();
                if pixel != 0 {
                    let palette = (self.sprites[i].data.attr & 0x03) + 4;
                    let priority = (self.sprites[i].data.attr & 0x20) != 0;

                    if i == 0 {
                        self.zero_rendered = true;
                    }

                    return (pixel, palette, priority);
                }
            }
        }

        (0, 0, false)
    }

    fn evaluate(&mut self, scanline: &u16, ctrl: &CtrlFlags, status: &mut StatusFlags, oam: &Oam) {
        self.counter = 0;
        self.zero_hit_possible = false;
        let scanline_y = *scanline as u16;

        for i in 0..64 {
            let entry = i * 4;
            let sprite_top = oam.data[entry] as u16 + 1;
            let sprite_height = ctrl.sprite_size() as u16;

            if scanline_y >= sprite_top && scanline_y < sprite_top + sprite_height {
                if self.counter < 8 {
                    if i == 0 {
                        self.zero_hit_possible = true;
                    }

                    self.sprites[self.counter].data = SpriteData::new(oam, entry);

                    self.counter += 1;
                } else {
                    status.set(StatusFlags::SPRITE_OVERFLOW, true);
                    break;
                }
            }
        }
    }

    fn compute_overflow_trigger_cycle(
        &self,
        scanline: u16,
        ctrl: &CtrlFlags,
        oam: &Oam,
    ) -> Option<usize> {
        let scanline_y = scanline as u16;
        let sprite_height = ctrl.sprite_size() as u16;
        let mut cycle = 65usize;
        let mut in_range_count = 0usize;

        let mut sprite_index = 0usize;
        while sprite_index < 64 {
            let y = oam.data[sprite_index * 4];
            if Self::byte_in_range(scanline_y, y, sprite_height) {
                in_range_count += 1;
                if in_range_count == 8 {
                    cycle += 8;
                    sprite_index += 1;
                    break;
                }

                cycle += 8;
            } else {
                cycle += 2;
            }

            sprite_index += 1;
        }

        let mut byte_index = 0usize;
        while sprite_index < 64 {
            let value = oam.data[sprite_index * 4 + byte_index];
            if Self::byte_in_range(scanline_y, value, sprite_height) {
                return Some(cycle + 1);
            }

            cycle += 2;
            sprite_index += 1;
            byte_index = (byte_index + 1) & 0x03;
        }

        None
    }

    fn byte_in_range(scanline_y: u16, value: u8, sprite_height: u16) -> bool {
        let sprite_top = value as u16 + 1;
        scanline_y >= sprite_top && scanline_y < sprite_top + sprite_height
    }

    fn load_sprite_shifters(&mut self, scanline: &u16, ctrl: &CtrlFlags, mem: &mut Memory) {
        let scanline_y = *scanline as u16;
        let sprite_pattern_table = ctrl.sprite_pattern();
        let base_ppu_cycle = mem.ppu_cycle();

        for i in 0..8 {
            let fetch_start_cycle = base_ppu_cycle + 7 + (i as u64 * 8);
            let (sprite_pattern_addr_lo, sprite_pattern_addr_hi) = if i < self.counter {
                let sprite = &self.sprites[i];
                let sprite_height = ctrl.sprite_size();

                if sprite_height == 8 {
                    let sprite_row = scanline_y - (sprite.data.y as u16 + 1);
                    let flip_vertical = (sprite.data.attr & 0x80) != 0;
                    let row = if flip_vertical { 7 - sprite_row } else { sprite_row };
                    let lo = sprite_pattern_table + (sprite.data.id as u16 * 16) + row;
                    (lo, lo + 8)
                } else {
                    let pattern_table = if (sprite.data.id & 0x01) != 0 { 0x1000 } else { 0x0000 };
                    let tile_id = sprite.data.id & 0xFE;
                    let sprite_row = scanline_y - (sprite.data.y as u16 + 1);
                    let flip_vertical = (sprite.data.attr & 0x80) != 0;
                    let row = if flip_vertical { 15 - sprite_row } else { sprite_row };
                    let lo = if row < 8 {
                        pattern_table + (tile_id as u16 * 16) + row
                    } else {
                        pattern_table + ((tile_id + 1) as u16 * 16) + (row - 8)
                    };
                    (lo, lo + 8)
                }
            } else {
                (sprite_pattern_table, sprite_pattern_table + 8)
            };

            mem.set_ppu_cycle(fetch_start_cycle);
            let lo = mem.read(sprite_pattern_addr_lo);
            mem.set_ppu_cycle(fetch_start_cycle + 2);
            let hi = mem.read(sprite_pattern_addr_hi);

            if i < self.counter {
                let sprite = &mut self.sprites[i];
                sprite.shifter.load(&sprite.data, lo, hi);
            }
        }

        mem.set_ppu_cycle(base_ppu_cycle);
    }

    fn tick_render(&mut self, cycle: usize) {
        if !(1..=256).contains(&cycle) {
            return;
        }

        for i in 0..self.counter {
            if self.sprites[i].data.x == 0 {
                self.sprites[i].shifter.update();
            } else {
                self.sprites[i].data.x -= 1;
            }
        }
    }
}

impl Shifter {
    #[inline]
    pub fn update(&mut self) {
        self.pattern_lo <<= 1;
        self.pattern_hi <<= 1;
    }

    #[inline]
    pub fn load(&mut self, sprite: &SpriteData, lo: u8, hi: u8) {
        let flip = (sprite.attr & 0x40) != 0;
        self.pattern_lo = if flip { lo.reverse_bits() } else { lo };
        self.pattern_hi = if flip { hi.reverse_bits() } else { hi };
    }

    #[inline]
    pub fn get_pixel(&mut self) -> u8 {
        let p0 = ((self.pattern_lo & 0x80) > 0) as u8;
        let p1 = ((self.pattern_hi & 0x80) > 0) as u8;
        (p1 << 1) | p0
    }
}

impl SpriteData {
    pub fn new(oam: &Oam, entry: usize) -> Self {
        Self {
            id: oam.data[entry + 1],
            attr: oam.data[entry + 2],
            x: oam.data[entry + 3],
            y: oam.data[entry],
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sprite_with_x_delay_becomes_visible_on_next_pixel() {
        let mut sprites = Sprites::default();
        sprites.counter = 1;
        sprites.sprites[0].data.x = 1;
        sprites.sprites[0].shifter.pattern_lo = 0b1000_0000;
        sprites.sprites[0].shifter.pattern_hi = 0;

        assert_eq!(sprites.get_pixel(), (0, 0, false));

        // Cycle 1 consumes X delay only.
        sprites.tick_render(1);
        assert_eq!(sprites.get_pixel(), (1, 4, false));
    }

    #[test]
    fn sprite_pixel_shifts_after_visible_pixel_is_sampled() {
        let mut sprites = Sprites::default();
        sprites.counter = 1;
        sprites.sprites[0].data.x = 0;
        sprites.sprites[0].shifter.pattern_lo = 0b1000_0000;
        sprites.sprites[0].shifter.pattern_hi = 0;

        assert_eq!(sprites.get_pixel(), (1, 4, false));

        // Shift for next pixel.
        sprites.tick_render(2);
        assert_eq!(sprites.get_pixel(), (0, 0, false));
    }

    #[test]
    fn sprite_y_255_is_hidden_on_scanline_zero() {
        let mut sprites = Sprites::default();
        let mut status = StatusFlags::default();
        let ctrl = CtrlFlags::default();
        let mut oam = Oam::default();

        oam.data[0] = 255;
        oam.data[1] = 0;
        oam.data[2] = 0;
        oam.data[3] = 0;

        sprites.evaluate(&0, &ctrl, &mut status, &oam);
        assert!(!sprites.zero_hit_possible);
        assert_eq!(sprites.counter, 0);
    }
}
