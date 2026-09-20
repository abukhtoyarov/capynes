use bytemuck::{Pod, Zeroable};
use bitflags::bitflags;
use std::fmt;

#[repr(C)]
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub struct NesHeader {
    pub magic: [u8; 4],      // "NES\x1A"
    pub prg_rom_pages: u8,   // 16 KB pages
    pub chr_rom_pages: u8,   // 8 KB pages
    pub flags6: u8,
    pub flags7: u8,
    pub prg_ram_pages: u8,   // 8 KB pages
    pub flags9: u8,
    pub flags10: u8,
    pub reserved: [u8; 5],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TVFormat {
    NTSC,
    PAL,
    Dual,
}

impl fmt::Display for TVFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TVFormat::NTSC => write!(f, "NTSC"),
            TVFormat::PAL => write!(f, "PAL"),
            TVFormat::Dual => write!(f, "NTSC/PAL"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mirroring {
    Horizontal,
    Vertical,
    SingleScreenLower,
    SingleScreenUpper,
    FourScreen,
}

impl fmt::Display for Mirroring {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Mirroring::Horizontal => write!(f, "Horizontal"),
            Mirroring::Vertical => write!(f, "Vertical"),
            Mirroring::SingleScreenLower => write!(f, "Single screen lower"),
            Mirroring::SingleScreenUpper => write!(f, "Single screen lower"),
            Mirroring::FourScreen => write!(f, "Four-Screen"),
        }
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    struct Flags6: u8 {
        const MIRRORING    = 0x01;
        const BATTERY      = 0x02;
        const TRAINER      = 0x04;
        const FOUR_SCREEN  = 0x08;
        const MAPPER_LOWER = 0xF0;
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    struct Flags7: u8 {
        const VS_UNISYSTEM = 0x01;
        const PLAYCHOICE10 = 0x02;
        const NES2_LOWER   = 0x0C;
        const MAPPER_UPPER = 0xF0;
    }
}

bitflags! {
    #[derive(Copy, Clone)]
    struct Flags9: u8 {
        const PAL = 0x01;
    }
}

#[derive(Debug, Clone)]
pub struct RomInfo {
    pub mapper_id: u8,
    pub submapper_id: u8,
    pub mirroring: Mirroring,
    pub tv_format: TVFormat,
    pub has_battery: bool,
    pub has_trainer: bool,
    pub vs_unisystem: bool,
    pub playchoice10: bool,
    pub nes2_format: bool,
    pub prg_rom_size: usize,
    pub chr_rom_size: usize,
    pub prg_ram_size: usize,
    pub chr_ram_size: usize,
}

impl fmt::Display for RomInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Loaded ROM information:")?;
        writeln!(f, "=== NES ROM Information ===")?;
        writeln!(f, "Mapper:          {} ({})", self.mapper_id, self.mapper_name())?;
        if self.submapper_id != 0 {
            writeln!(f, "Submapper:       {}", self.submapper_id)?;
        }
        writeln!(f, "Mirroring:       {}", self.mirroring)?;
        writeln!(f, "TV Format:       {}", self.tv_format)?;
        writeln!(f, "PRG ROM:         {} KB ({} pages)", self.prg_rom_size / 1024, self.prg_rom_size / (16 * 1024))?;
        if self.chr_rom_size > 0 {
            writeln!(f, "CHR ROM:         {} KB ({} pages)", self.chr_rom_size / 1024, self.chr_rom_size / (8 * 1024))?;
        } else {
            writeln!(f, "CHR RAM:         {} KB", self.chr_ram_size / 1024)?;
        }
        if self.prg_ram_size > 0 {
            writeln!(f, "PRG RAM:         {} KB", self.prg_ram_size / 1024)?;
        }
        if self.has_battery { writeln!(f, "Battery:         Yes")?; }
        if self.has_trainer { writeln!(f, "Trainer:         Yes (512 bytes)")?; }
        if self.vs_unisystem { writeln!(f, "VS Unisystem:    Yes")?; }
        if self.playchoice10 { writeln!(f, "PlayChoice-10:   Yes")?; }
        writeln!(f, "Format:          {}", if self.nes2_format { "NES 2.0" } else { "iNES" })?;
        Ok(())
    }
}

impl RomInfo {
    fn mapper_name(&self) -> &'static str {
        match self.mapper_id {
            0 => "NROM",
            1 => "MMC1 / SxROM",
            2 => "UxROM",
            3 => "CNROM",
            4 => "MMC3 / TxROM",
            5 => "MMC5 / ExROM",
            7 => "AxROM",
            9 => "MMC2",
            10 => "MMC4",
            11 => "Color Dreams",
            22 => "Konami VRC2a",
            23 => "Konami VRC2b / VRC4e",
            28 => "Action 53",
            34 => "BNROM / NINA-001",
            66 => "GxROM",
            118 => "TKSROM / TLSROM (MMC3)",
            119 => "TQROM (MMC3)",
            185 => "CNROM (CHR disable protection)",
            _ => "Unknown",
        }
    }
}

impl NesHeader {
    pub const SIZE: usize = 16;
    pub const TRAINER_SIZE: usize = 512;
    const PRG_ROM_PAGE_SIZE: usize = 16 * 1024;
    const CHR_ROM_PAGE_SIZE: usize = 8 * 1024;
    const PRG_RAM_PAGE_SIZE: usize = 8 * 1024;

    pub fn new(data: &[u8]) -> Result<Self, String> {
        let header: NesHeader = *bytemuck::from_bytes(&data[..Self::SIZE]);
        if &header.magic != b"NES\x1A" { return Err("Not a NES ROM".to_string()); }
        Ok(header)
    }

    pub fn parse(&self) -> RomInfo {
        let flags6 = Flags6::from_bits_truncate(self.flags6);
        let flags7 = Flags7::from_bits_truncate(self.flags7);
        let flags9 = Flags9::from_bits_truncate(self.flags9);
        let nes2_format = (flags7 & Flags7::NES2_LOWER).bits() == 0x08;

        let mapper_lower = ((flags6 & Flags6::MAPPER_LOWER).bits() >> 4) as u16;
        let mapper_upper = (flags7 & Flags7::MAPPER_UPPER).bits() as u16;
        let mapper_ext = if nes2_format {
            (self.prg_ram_pages as u16) & 0x0F
        } else {
            0
        };
        let mapper_id = ((mapper_ext << 8) | mapper_upper | mapper_lower) as u8;
        let submapper_id = if nes2_format {
            (self.prg_ram_pages >> 4) & 0x0F
        } else {
            0
        };

        let mirroring = if flags6.contains(Flags6::FOUR_SCREEN) {
            Mirroring::FourScreen
        } else if flags6.contains(Flags6::MIRRORING) {
            Mirroring::Vertical
        } else {
            Mirroring::Horizontal
        };

        let prg_rom_size = self.prg_rom_pages as usize * Self::PRG_ROM_PAGE_SIZE;
        let chr_rom_size = self.chr_rom_pages as usize * Self::CHR_ROM_PAGE_SIZE;

        let prg_ram_size = if self.prg_ram_pages == 0 {
            Self::PRG_RAM_PAGE_SIZE
        } else {
            self.prg_ram_pages as usize * Self::PRG_RAM_PAGE_SIZE
        };

        let chr_ram_size = if chr_rom_size == 0 { Self::CHR_ROM_PAGE_SIZE } else { 0 };

        RomInfo {
            mapper_id,
            submapper_id,
            mirroring,
            tv_format: if flags9.contains(Flags9::PAL) { TVFormat::PAL } else { TVFormat::NTSC },
            has_battery: flags6.contains(Flags6::BATTERY),
            has_trainer: flags6.contains(Flags6::TRAINER),
            vs_unisystem: flags7.contains(Flags7::VS_UNISYSTEM),
            playchoice10: flags7.contains(Flags7::PLAYCHOICE10),
            nes2_format,
            prg_rom_size,
            chr_rom_size,
            prg_ram_size,
            chr_ram_size,
        }
    }
}
