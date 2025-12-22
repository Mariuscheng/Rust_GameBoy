pub mod mbc1;
pub mod mbc2;
pub mod mbc3;
pub mod mbc5;
pub mod types;

pub use self::mbc1::MBC1;
pub use self::mbc2::MBC2;
pub use self::mbc3::MBC3;
pub use self::mbc5::MBC5;
#[allow(unused_imports)]
pub use self::types::MemoryBankController;

/// MBC 控制器特徵
#[allow(dead_code)]
pub trait MBCController: std::fmt::Debug {
    #[allow(dead_code)]
    fn read(&self, addr: u16, rom: &[u8]) -> u8;
    fn write(&mut self, addr: u16, value: u8);
    fn translate_rom_address(&self, addr: u16) -> u32;
    fn translate_ram_address(&self, addr: u16) -> u16;
    fn current_rom_bank(&self) -> u8;
}

// 用於創建適當的 MBC 實例
pub fn create_mbc(cartridge_type: u8) -> Option<Box<dyn MBCController>> {
    match cartridge_type {
        0x00 => None,                               // ROM ONLY
        0x01..=0x03 => Some(Box::new(MBC1::new())), // MBC1
        0x05..=0x06 => Some(Box::new(MBC2::new())), // MBC2
        0x0F..=0x13 => Some(Box::new(MBC3::new())), // MBC3
        0x19..=0x1E => Some(Box::new(MBC5::new())), // MBC5
        _ => None,
    }
}
