use super::MBCController;

#[derive(Debug)]
pub struct MBC5 {
    ram_enabled: bool,
    rom_bank: usize,
    ram_bank: usize,
}

impl MBC5 {
    pub fn new() -> Self {
        MBC5 {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
        }
    }
}

impl MBCController for MBC5 {
    fn read(&self, addr: u16, rom: &[u8]) -> u8 {
        let rom_addr = self.translate_rom_address(addr) as usize;
        if rom_addr < rom.len() {
            rom[rom_addr]
        } else {
            0 // Out of bounds, return 0
        }
    }

    fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x2FFF => {
                // MBC5 允許直接選擇ROM bank 0
                self.rom_bank = (self.rom_bank & 0x100) | (value as usize);
            }
            0x3000..=0x3FFF => {
                // 9位元ROM bank號碼的最高位
                self.rom_bank = (self.rom_bank & 0xFF) | (((value & 0x01) as usize) << 8);
            }
            0x4000..=0x5FFF => {
                self.ram_bank = (value & 0x0F) as usize;
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
        ((self.ram_bank * 0x2000) + (addr as usize)) as u16
    }

    fn current_rom_bank(&self) -> u8 {
        (self.rom_bank & 0xFF) as u8
    }
}
