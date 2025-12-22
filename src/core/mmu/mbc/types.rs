#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MBCType {
    None,        // 無 MBC (ROM Only)
    MBC1,        // MBC1 - 最常見的控制器
    MBC2,        // MBC2 - 內建 RAM
    MBC3,        // MBC3 - 支援 RTC
    MBC5,        // MBC5 - 最先進的控制器
    Unknown(u8), // 未知類型
}

#[allow(dead_code)]
impl MBCType {
    pub fn from_cartridge_type(cartridge_type: u8) -> Self {
        match cartridge_type {
            0x00 => MBCType::None,
            0x01..=0x03 => MBCType::MBC1,
            0x05..=0x06 => MBCType::MBC2,
            0x0F..=0x13 => MBCType::MBC3,
            0x19..=0x1E => MBCType::MBC5,
            _ => MBCType::Unknown(cartridge_type),
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            MBCType::None => "ROM Only",
            MBCType::MBC1 => "MBC1",
            MBCType::MBC2 => "MBC2 + Battery",
            MBCType::MBC3 => "MBC3 + Timer + Battery",
            MBCType::MBC5 => "MBC5",
            MBCType::Unknown(_code) => "Unknown MBC",
        }
    }
}

#[allow(dead_code)]
pub fn get_rom_size_bytes(rom_size_code: u8) -> usize {
    match rom_size_code {
        0x00 => 32 * 1024,   // 32KB
        0x01 => 64 * 1024,   // 64KB
        0x02 => 128 * 1024,  // 128KB
        0x03 => 256 * 1024,  // 256KB
        0x04 => 512 * 1024,  // 512KB
        0x05 => 1024 * 1024, // 1MB
        0x06 => 2048 * 1024, // 2MB
        0x07 => 4096 * 1024, // 4MB
        0x08 => 8192 * 1024, // 8MB
        _ => 32 * 1024,      // 默認 32KB
    }
}

#[allow(dead_code)]
pub fn get_ram_size_bytes(ram_size_code: u8) -> usize {
    match ram_size_code {
        0x00 => 0,          // 無 RAM
        0x01 => 2 * 1024,   // 2KB
        0x02 => 8 * 1024,   // 8KB
        0x03 => 32 * 1024,  // 32KB (4 banks of 8KB)
        0x04 => 128 * 1024, // 128KB (16 banks of 8KB)
        0x05 => 64 * 1024,  // 64KB (8 banks of 8KB)
        _ => 0,             // 默認無 RAM
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct MBCState {
    pub mbc_type: MBCType,
    pub rom_bank: u16,          // 當前 ROM bank
    pub ram_bank: u8,           // 當前 RAM bank
    pub ram_enabled: bool,      // RAM 是否啟用
    pub mbc1_mode: bool,        // MBC1 模式 (false=ROM模式, true=RAM模式)
    pub rtc_enabled: bool,      // MBC3 RTC 是否啟用
    pub rtc_latched: bool,      // MBC3 RTC 鎖存狀態
    pub rtc_registers: [u8; 5], // MBC3 RTC 暫存器 (S, M, H, DL, DH)
    pub battery_backed: bool,   // 是否有電池備份
}

#[allow(dead_code)]
impl MBCState {
    pub fn new(mbc_type: MBCType) -> Self {
        Self {
            mbc_type,
            rom_bank: if mbc_type == MBCType::None { 0 } else { 1 },
            ram_bank: 0,
            ram_enabled: false,
            mbc1_mode: false,
            rtc_enabled: false,
            rtc_latched: false,
            rtc_registers: [0; 5],
            battery_backed: false,
        }
    }
}

#[allow(dead_code)]
/// Memory Bank Controller trait
pub trait MemoryBankController {
    /// 讀取指定地址的值
    fn read(&self, addr: u16) -> u8;

    /// 寫入值到指定地址
    fn write(&mut self, addr: u16, value: u8);

    /// 獲取當前 ROM 庫號
    fn get_rom_bank(&self) -> usize;

    /// 獲取當前 RAM 庫號
    fn get_ram_bank(&self) -> usize;

    /// RAM 是否已啟用
    fn is_ram_enabled(&self) -> bool;
}
