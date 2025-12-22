use super::MBCController;

#[derive(Debug)]
pub struct MBC3 {
    ram_enabled: bool,
    rom_bank: usize,
    ram_bank: usize,
    rtc_enabled: bool,
}

impl MBC3 {
    pub fn new() -> Self {
        MBC3 {
            ram_enabled: false,
            rom_bank: 1,
            ram_bank: 0,
            rtc_enabled: false,
        }
    }
}

impl MBCController for MBC3 {
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
                self.ram_enabled = value & 0x0F == 0x0A;
            }
            0x2000..=0x3FFF => {
                let bank = value & 0x7F;
                self.rom_bank = if bank == 0 { 1 } else { bank as usize };
            }
            0x4000..=0x5FFF => {
                // RAM banks 00h-03h, or RTC registers 08h-0Ch
                if value <= 0x03 {
                    self.ram_bank = value as usize;
                    self.rtc_enabled = false;
                } else if value >= 0x08 && value <= 0x0C {
                    self.rtc_enabled = true;
                }
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
        if !self.ram_enabled || self.rtc_enabled {
            return addr;
        }
        ((self.ram_bank * 0x2000) + (addr as usize)) as u16
    }

    fn current_rom_bank(&self) -> u8 {
        self.rom_bank as u8
    }
}
