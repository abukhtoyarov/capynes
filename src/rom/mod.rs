mod mapper;
pub mod info;
pub(crate) use mapper::Mapper;
pub use mapper::Mmc3IrqRevision;

use info::{NesHeader, RomInfo, Mirroring};
use mapper::nrom::NROM;
use mapper::mmc1::MMC1;
use mapper::unrom::UNROM;
use mapper::cnrom::CNROM;
use mapper::mmc3::MMC3;
use mapper::mmc5::MMC5;
use mapper::color_dreams_11::ColorDreams11;
use mapper::action53_28::Action53_28;
use mapper::vrc2a_22::Vrc2a22;
use mapper::vrc2b_23::Vrc2b23;
use mapper::bnrom_34::Bnrom34;
use mapper::axrom::AXROM;
use std::fmt;

pub struct Rom {
    pub trainer: Option<Vec<u8>>,
    pub info: RomInfo,
    pub mapper: std::cell::RefCell<Box<dyn Mapper>>,
}

pub type RomRef = std::rc::Rc<Rom>;

impl Rom {
    pub fn new(path: &str) -> Result<RomRef, String> {
        Ok(std::rc::Rc::new(Self::load_from_file(path)?))
    }

    pub fn mirroring(&self) -> Mirroring {
        if let Some(mirroring) = self.mapper.borrow().mirroring() {
            mirroring
        } else {
            self.info.mirroring
        }
    }

    fn create_mapper(
        mapper_id: u8,
        submapper_id: u8,
        prg_rom: Vec<u8>,
        chr_rom: Vec<u8>,
        chr_ram: Vec<u8>,
    ) -> Result<Box<dyn Mapper>, String> {
        match mapper_id {
            // iNES 1.0 mapper IDs currently supported by this emulator.
            0 => Ok(Box::new(NROM::new(prg_rom, chr_rom, chr_ram))),  // NROM
            1 => match submapper_id {
                5 => Ok(Box::new(MMC1::new_submapper5(prg_rom, chr_rom, chr_ram))), // SEROM/SHROM/SH1ROM
                6 => Ok(Box::new(MMC1::new_submapper6(prg_rom, chr_rom, chr_ram))), // 2ME (Famicom Network)
                7 => Ok(Box::new(MMC1::new_submapper7(prg_rom, chr_rom, chr_ram))), // KS-7058
                _ => Ok(Box::new(MMC1::new(prg_rom, chr_rom, chr_ram))),             // MMC1 / SxROM
            },
            2 => Ok(Box::new(UNROM::new(prg_rom, chr_rom, chr_ram))), // UxROM
            3 => Ok(Box::new(CNROM::new(prg_rom, chr_rom, chr_ram))), // CNROM
            4 => Ok(Box::new(MMC3::new(prg_rom, chr_rom, chr_ram))),  // MMC3 / TxROM
            5 => Ok(Box::new(MMC5::new(prg_rom, chr_rom, chr_ram))),  // MMC5 / ExROM
            11 => Ok(Box::new(ColorDreams11::new(prg_rom, chr_rom, chr_ram))), // Color Dreams
            28 => Ok(Box::new(Action53_28::new(prg_rom, chr_rom, chr_ram))), // Action 53 (minimal fallback)
            22 => Ok(Box::new(Vrc2a22::new(prg_rom, chr_rom, chr_ram))), // Konami VRC2a
            23 => Ok(Box::new(Vrc2b23::new(prg_rom, chr_rom, chr_ram))), // Konami VRC2b
            34 => Ok(Box::new(Bnrom34::new(prg_rom, chr_rom, chr_ram))), // BNROM/NINA-001 (BNROM path)
            7 => match submapper_id {
                1 => Ok(Box::new(AXROM::new_submapper1(prg_rom, chr_rom, chr_ram))), // AxROM, no bus conflicts
                2 => Ok(Box::new(AXROM::new_submapper2(prg_rom, chr_rom, chr_ram))), // AxROM, bus conflicts
                _ => Ok(Box::new(AXROM::new(prg_rom, chr_rom, chr_ram))),             // AxROM generic
            },
            118 => Ok(Box::new(MMC3::new_mapper118(prg_rom, chr_rom, chr_ram))), // TKSROM / TLSROM
            119 => Ok(Box::new(MMC3::new_mapper119(prg_rom, chr_rom, chr_ram))), // TQROM
            185 => Ok(Box::new(CNROM::new_mapper185(prg_rom, chr_rom, chr_ram))), // CNROM (CHR disable variant)
            _ => Err(format!("Mapper {} not supported", mapper_id)),
        }
    }

    fn load(data: &[u8]) -> Result<Self, String> {
        if data.len() < NesHeader::SIZE { return Err("File too small".to_string()); }

        let header = NesHeader::new(data)?;
        let info = header.parse();

        let mut offset = NesHeader::SIZE;
        let trainer = if info.has_trainer {
            if data.len() < offset + NesHeader::TRAINER_SIZE { return Err("Unexpected EOF (trainer)".to_string()); }
            let t = data[offset..offset + NesHeader::TRAINER_SIZE].to_vec();
            offset += NesHeader::TRAINER_SIZE;
            Some(t)
        } else { None };

        let prg_rom = data[offset..offset + info.prg_rom_size].to_vec();
        offset += info.prg_rom_size;

        let (chr_rom, chr_ram) = if info.chr_rom_size == 0 {
            (Vec::new(), vec![0; info.chr_ram_size])
        } else {
            (data[offset..offset + info.chr_rom_size].to_vec(), Vec::new())
        };

        let mapper = Self::create_mapper(
            info.mapper_id,
            info.submapper_id,
            prg_rom,
            chr_rom,
            chr_ram,
        )?;

        Ok(Self { trainer, info, mapper: std::cell::RefCell::new(mapper) })
    }

    fn load_from_file(path: &str) -> Result<Self, String> {
        let bytes = std::fs::read(path).map_err(|e| format!("Failed to load rom file: {}", e))?;
        Self::load(&bytes)
    }
}

impl fmt::Display for Rom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.info)
    }
}
