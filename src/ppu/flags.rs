use bitflags::bitflags;

bitflags! {
    // 7  bit  0
    // ---- ----
    // VPHB SINN
    // |||| ||||
    // |||| ||++- Base nametable address
    // |||| ||    (0 = $2000; 1 = $2400; 2 = $2800; 3 = $2C00)
    // |||| |+--- VRAM address increment per CPU read/write of PPUDATA
    // |||| |     (0: add 1, going across; 1: add 32, going down)
    // |||| +---- Sprite pattern table address for 8x8 sprites
    // ||||       (0: $0000; 1: $1000; ignored in 8x16 mode)
    // |||+------ Background pattern table address (0: $0000; 1: $1000)
    // ||+------- Sprite size (0: 8x8 pixels; 1: 8x16 pixels – see PPU OAM#Byte 1)
    // |+-------- PPU master/slave select
    // |          (0: read backdrop from EXT pins; 1: output color on EXT pins)
    // +--------- Vblank NMI enable (0: off, 1: on)
    #[derive(Default)]
    pub struct CtrlFlags: u8 {
        const NAMETABLE1 = (1 << 0);
        const NAMETABLE2 = (1 << 1);
        const INCREMENT_VRAM_ADDR = (1 << 2);
        const SPRITE_PATTERN = (1 << 3);
        const BG_PATTERN = (1 << 4);
        const SPRITE_SIZE = (1 << 5);
        const MASTER_SLAVE_SELECT = (1 << 6);
        const VBLANK_NMI_ENABLE = (1<<7);
    }

    // 7  bit  0
    // ---- ----
    // BGRs bMmG
    // |||| ||||
    // |||| |||+- Greyscale (0: normal color, 1: greyscale)
    // |||| ||+-- 1: Show background in leftmost 8 pixels of screen, 0: Hide
    // |||| |+--- 1: Show sprites in leftmost 8 pixels of screen, 0: Hide
    // |||| +---- 1: Enable background rendering
    // |||+------ 1: Enable sprite rendering
    // ||+------- Emphasize red (green on PAL/Dendy)
    // |+-------- Emphasize green (red on PAL/Dendy)
    // +--------- Emphasize blue
    #[derive(Default)]
    pub struct MaskFlags: u8 {
        const GREYSCALE = (1 << 0);
        const BACKGROUND_LEFTMOST = (1 << 1);
        const SPRITE_LEFTMOST = (1 << 2);
        const ENABLE_BACKGROUND_RENDERING = (1 << 3);
        const ENABLE_SPRITE_RENDERING = (1 << 4);
        const EMPHASIS_RED = (1 << 5);
        const EMPHASIS_GREEN = (1 << 6);
        const EMPHASIS_BLUE = (1 << 7);
    }

    // 7  bit  0
    // ---- ----
    // VSOx xxxx
    // |||| ||||
    // |||+-++++- (PPU open bus or 2C05 PPU identifier)
    // ||+------- Sprite overflow flag
    // |+-------- Sprite 0 hit flag
    // +--------- Vblank flag, cleared on read. Unreliable; see below.
    #[derive(Default)]
    pub struct StatusFlags: u8 {
        const RESERVED1 = (1 << 0);
        const RESERVED2 = (1 << 1);
        const RESERVED3 = (1 << 2);
        const RESERVED4 = (1 << 3);
        const RESERVED5 = (1 << 4);
        const SPRITE_OVERFLOW = (1 << 5);
        const SPRITE_ZERO_HIT = (1 << 6);
        const VBLANK = (1 << 7);
    }
}

impl CtrlFlags {
    pub fn bg_pattern(&self) -> u16 { if self.contains(CtrlFlags::BG_PATTERN) { 0x1000 } else { 0 } }
    pub fn sprite_pattern(&self) -> u16 { if self.contains(CtrlFlags::SPRITE_PATTERN) { 0x1000 } else { 0 } }
    pub fn sprite_size(&self) -> u8 { if self.contains(CtrlFlags::SPRITE_SIZE) { 16 } else { 8 } }
    pub fn increment(&self) -> u16 { if self.contains(CtrlFlags::INCREMENT_VRAM_ADDR) { 32 } else { 1 } }
    // Frame buffer
}

impl MaskFlags {
}

impl StatusFlags {
}
