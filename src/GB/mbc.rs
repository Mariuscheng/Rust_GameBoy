// MBC (Memory Bank Controller) implementations
use crate::GB::types::MbcType;

pub trait MBC {
    fn read_rom(&self, addr: u16, rom: &[u8], rom_banks: usize) -> u8;
    fn write(&mut self, addr: u16, val: u8);
    fn ram_enabled(&self) -> bool;
    fn ram_bank(&self) -> u8;
    fn rom_bank(&self) -> u16;
}

#[derive(Debug)]
pub struct MBCNone;

impl MBCNone {
    pub fn new() -> Self {
        Self
    }
}

impl MBC for MBCNone {
    fn read_rom(&self, addr: u16, rom: &[u8], _rom_banks: usize) -> u8 {
        let i = addr as usize;
        if i < rom.len() {
            rom[i]
        } else {
            0xFF
        }
    }

    fn write(&mut self, _addr: u16, _val: u8) {
        // No-op for MBC None
    }

    fn ram_enabled(&self) -> bool {
        false
    }

    fn ram_bank(&self) -> u8 {
        0
    }

    fn rom_bank(&self) -> u16 {
        0
    }
}

#[derive(Debug)]
pub struct MBC1 {
    ram_enable: bool,
    rom_bank: u16,
    ram_bank: u8,
    bank_low5: u8,
    bank_high2: u8,
    mode: u8,
}

impl MBC1 {
    pub fn new() -> Self {
        Self {
            ram_enable: false,
            rom_bank: 1,
            ram_bank: 0,
            bank_low5: 1,
            bank_high2: 0,
            mode: 0,
        }
    }

    fn calc_bank0(&self) -> u16 {
        if self.mode & 1 == 0 {
            0
        } else {
            ((self.bank_high2 as u16) & 0x03) << 5
        }
    }

    fn calc_bankx(&self) -> u16 {
        let low5 = (self.bank_low5 as u16) & 0x1F;
        let mut bank = if self.mode & 1 == 0 {
            low5 | (((self.bank_high2 as u16) & 0x03) << 5)
        } else {
            low5
        };
        if (bank & 0x1F) == 0 {
            bank |= 1;
        }
        bank
    }
}

impl MBC for MBC1 {
    fn read_rom(&self, addr: u16, rom: &[u8], rom_banks: usize) -> u8 {
        if addr < 0x4000 {
            let bank0 = (self.calc_bank0() as usize) % rom_banks;
            let base = bank0 * 0x4000;
            let i = base + addr as usize;
            if i < rom.len() {
                rom[i]
            } else {
                0xFF
            }
        } else {
            let bankx = (self.calc_bankx() as usize) % rom_banks;
            let base = bankx * 0x4000;
            let i = base + (addr as usize - 0x4000);
            if i < rom.len() {
                rom[i]
            } else {
                0xFF
            }
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enable = (val & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                let mut low5 = val & 0x1F;
                if low5 == 0 {
                    low5 = 1;
                }
                self.bank_low5 = low5;
            }
            0x4000..=0x5FFF => {
                self.bank_high2 = val & 0x03;
                if self.mode & 1 == 1 {
                    self.ram_bank = self.bank_high2 & 0x03;
                }
            }
            0x6000..=0x7FFF => {
                self.mode = val & 0x01;
            }
            _ => {}
        }
    }

    fn ram_enabled(&self) -> bool {
        self.ram_enable
    }

    fn ram_bank(&self) -> u8 {
        if self.mode & 1 == 0 {
            0
        } else {
            self.ram_bank
        }
    }

    fn rom_bank(&self) -> u16 {
        self.calc_bankx()
    }
}

#[derive(Debug)]
pub struct MBC3 {
    ram_enable: bool,
    rom_bank: u16,
    ram_bank: u8,
    rtc_sel: Option<u8>,
    rtc_regs: [u8; 5],
}

impl MBC3 {
    pub fn new() -> Self {
        Self {
            ram_enable: false,
            rom_bank: 1,
            ram_bank: 0,
            rtc_sel: None,
            rtc_regs: [0; 5],
        }
    }
}

impl MBC for MBC3 {
    fn read_rom(&self, addr: u16, rom: &[u8], rom_banks: usize) -> u8 {
        if addr < 0x4000 {
            let i = addr as usize;
            if i < rom.len() {
                rom[i]
            } else {
                0xFF
            }
        } else {
            let mut bank = (self.rom_bank as usize) & 0x7F;
            if bank == 0 {
                bank = 1;
            }
            let base = (bank % rom_banks) * 0x4000;
            let i = base + (addr as usize - 0x4000);
            if i < rom.len() {
                rom[i]
            } else {
                0xFF
            }
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => {
                if cfg!(test) {
                    eprintln!("[MBC3] write addr=0x{:04X} val=0x{:02X}", addr, val);
                }
                self.ram_enable = (val & 0x0F) == 0x0A;
            }
            0x2000..=0x3FFF => {
                let mut b = (val & 0x7F) as u16;
                if b == 0 {
                    b = 1;
                }
                self.rom_bank = b;
            }
            0x4000..=0x5FFF => {
                let v = val & 0x0F;
                if v <= 0x03 {
                    self.ram_bank = v;
                    self.rtc_sel = None;
                } else if (0x08..=0x0C).contains(&v) {
                    self.rtc_sel = Some(v);
                }
            }
            0x6000..=0x7FFF => {}
            _ => {}
        }
    }

    fn ram_enabled(&self) -> bool {
        self.ram_enable
    }

    fn ram_bank(&self) -> u8 {
        self.ram_bank
    }

    fn rom_bank(&self) -> u16 {
        self.rom_bank
    }
}

#[derive(Debug)]
pub struct MBC5 {
    ram_enable: bool,
    rom_bank: u16,
    ram_bank: u8,
}

impl MBC5 {
    pub fn new() -> Self {
        Self {
            ram_enable: false,
            rom_bank: 1,
            ram_bank: 0,
        }
    }
}

impl MBC for MBC5 {
    fn read_rom(&self, addr: u16, rom: &[u8], rom_banks: usize) -> u8 {
        if addr < 0x4000 {
            let base = 0usize;
            let i = base + addr as usize;
            if i < rom.len() {
                rom[i]
            } else {
                0xFF
            }
        } else {
            let bank = (self.rom_bank as usize) % rom_banks;
            let base = bank * 0x4000;
            let i = base + (addr as usize - 0x4000);
            if i < rom.len() {
                rom[i]
            } else {
                0xFF
            }
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0x1FFF => {
                self.ram_enable = (val & 0x0F) == 0x0A;
            }
            0x2000..=0x2FFF => {
                if cfg!(test) {
                    eprintln!("[MBC5] write low addr=0x{:04X} val=0x{:02X}", addr, val);
                }
                self.rom_bank = (self.rom_bank & 0x100) | (val as u16);
            }
            0x3000..=0x3FFF => {
                if cfg!(test) {
                    eprintln!("[MBC5] write high addr=0x{:04X} val=0x{:02X}", addr, val);
                }
                self.rom_bank = (self.rom_bank & 0x0FF) | (((val as u16) & 0x01) << 8);
            }
            0x4000..=0x5FFF => {
                self.ram_bank = val & 0x0F;
            }
            0x6000..=0x7FFF => {}
            _ => {}
        }
    }

    fn ram_enabled(&self) -> bool {
        self.ram_enable
    }

    fn ram_bank(&self) -> u8 {
        self.ram_bank
    }

    fn rom_bank(&self) -> u16 {
        self.rom_bank
    }
}

#[derive(Debug)]
pub enum MBCImpl {
    None(MBCNone),
    MBC1(MBC1),
    MBC3(MBC3),
    MBC5(MBC5),
}

impl MBCImpl {
    pub fn new(mbc_type: MbcType) -> Self {
        match mbc_type {
            MbcType::None => MBCImpl::None(MBCNone::new()),
            MbcType::Mbc1 => MBCImpl::MBC1(MBC1::new()),
            MbcType::Mbc3 => MBCImpl::MBC3(MBC3::new()),
            MbcType::Mbc5 => MBCImpl::MBC5(MBC5::new()),
        }
    }
}

impl MBC for MBCImpl {
    fn read_rom(&self, addr: u16, rom: &[u8], rom_banks: usize) -> u8 {
        match self {
            MBCImpl::None(mbc) => mbc.read_rom(addr, rom, rom_banks),
            MBCImpl::MBC1(mbc) => mbc.read_rom(addr, rom, rom_banks),
            MBCImpl::MBC3(mbc) => mbc.read_rom(addr, rom, rom_banks),
            MBCImpl::MBC5(mbc) => mbc.read_rom(addr, rom, rom_banks),
        }
    }

    fn write(&mut self, addr: u16, val: u8) {
        match self {
            MBCImpl::None(mbc) => mbc.write(addr, val),
            MBCImpl::MBC1(mbc) => mbc.write(addr, val),
            MBCImpl::MBC3(mbc) => mbc.write(addr, val),
            MBCImpl::MBC5(mbc) => mbc.write(addr, val),
        }
    }

    fn ram_enabled(&self) -> bool {
        match self {
            MBCImpl::None(mbc) => mbc.ram_enabled(),
            MBCImpl::MBC1(mbc) => mbc.ram_enabled(),
            MBCImpl::MBC3(mbc) => mbc.ram_enabled(),
            MBCImpl::MBC5(mbc) => mbc.ram_enabled(),
        }
    }

    fn ram_bank(&self) -> u8 {
        match self {
            MBCImpl::None(mbc) => mbc.ram_bank(),
            MBCImpl::MBC1(mbc) => mbc.ram_bank(),
            MBCImpl::MBC3(mbc) => mbc.ram_bank(),
            MBCImpl::MBC5(mbc) => mbc.ram_bank(),
        }
    }

    fn rom_bank(&self) -> u16 {
        match self {
            MBCImpl::None(mbc) => mbc.rom_bank(),
            MBCImpl::MBC1(mbc) => mbc.rom_bank(),
            MBCImpl::MBC3(mbc) => mbc.rom_bank(),
            MBCImpl::MBC5(mbc) => mbc.rom_bank(),
        }
    }
}
