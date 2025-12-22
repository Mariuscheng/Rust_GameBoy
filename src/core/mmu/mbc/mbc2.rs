use super::MBCController;

#[derive(Debug)]
pub struct MBC2 {
    ram_enabled: bool,
    rom_bank: usize,
}

impl MBC2 {
    pub fn new() -> Self {
        MBC2 {
            ram_enabled: false,
            rom_bank: 1,
        }
    }
}

impl MBCController for MBC2 {
    fn read(&self, addr: u16, rom: &[u8]) -> u8 {
        if addr < 0x4000 {
            if addr as usize >= rom.len() {
                0xFF
            } else {
                rom[addr as usize]
            }
        } else if addr < 0x8000 {
            let bank = self.rom_bank;
            let offset = (bank as usize) * 0x4000 + (addr as usize - 0x4000);
            if offset >= rom.len() {
                0xFF
            } else {
                rom[offset]
            }
        } else {
            0xFF
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                // MBC2 的特殊RAM啟用邏輯
                if (addr & 0x0100) == 0 {
                    self.ram_enabled = value & 0x0F == 0x0A;
                }
            }
            0x2000..=0x3FFF => {
                // MBC2 只使用低4位
                let bank = value & 0x0F;
                self.rom_bank = if bank == 0 { 1 } else { bank as usize };
            }
            _ => {}
        }
    }

    fn translate_rom_address(&self, addr: u16) -> u32 {
        match addr {
            0x0000..=0x3FFF => addr as u32,
            0x4000..=0x7FFF => {
                let bank = self.rom_bank;
                ((bank * 0x4000) + (addr as usize - 0x4000)) as u32
            }
            _ => addr as u32,
        }
    }

    fn translate_ram_address(&self, addr: u16) -> u16 {
        if !self.ram_enabled {
            return addr;
        }
        // MBC2 有512字節的內建RAM
        (addr & 0x1FF) as u16
    }

    fn current_rom_bank(&self) -> u8 {
        self.rom_bank as u8
    }
}
