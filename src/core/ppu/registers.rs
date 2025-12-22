//! PPU 暫存器定義與 LCDRegisters struct

// PPU registers addresses
pub const LCDC: u16 = 0xFF40;
pub const STAT: u16 = 0xFF41;
pub const SCY: u16 = 0xFF42;
pub const SCX: u16 = 0xFF43;
pub const LY: u16 = 0xFF44;
pub const LYC: u16 = 0xFF45;
pub const DMA: u16 = 0xFF46;
pub const BGP: u16 = 0xFF47;
pub const OBP0: u16 = 0xFF48;
pub const OBP1: u16 = 0xFF49;
pub const WY: u16 = 0xFF4A;
pub const WX: u16 = 0xFF4B;

// LCDC bits
pub const LCDC_DISPLAY_ENABLE: u8 = 1 << 7;
pub const LCDC_WINDOW_TILE_MAP: u8 = 1 << 6;
pub const LCDC_WINDOW_ENABLE: u8 = 1 << 5;
pub const LCDC_BG_TILE_DATA: u8 = 1 << 4;
pub const LCDC_BG_TILE_MAP: u8 = 1 << 3;
pub const LCDC_OBJ_SIZE: u8 = 1 << 2;
pub const LCDC_OBJ_ENABLE: u8 = 1 << 1;
pub const LCDC_BG_ENABLE: u8 = 1 << 0;

// STAT bits
pub const STAT_LYC_INTERRUPT: u8 = 1 << 6;
pub const STAT_OAM_INTERRUPT: u8 = 1 << 5;
pub const STAT_VBLANK_INTERRUPT: u8 = 1 << 4;
pub const STAT_HBLANK_INTERRUPT: u8 = 1 << 3;
pub const STAT_LYC_EQUAL: u8 = 1 << 2;
pub const STAT_MODE_BITS: u8 = 0x03;

// Default values
pub const DEFAULT_LCDC: u8 = 0x91;
pub const DEFAULT_STAT: u8 = 0x00;
pub const DEFAULT_SCY: u8 = 0x00;
pub const DEFAULT_SCX: u8 = 0x00;
pub const DEFAULT_LY: u8 = 0x00;

#[derive(Default, Clone, Copy)]
pub struct LCDRegisters {
    pub lcdc: u8,
    pub stat: u8,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub wy: u8,
    pub wx: u8,
}

impl LCDRegisters {
    pub fn new() -> Self {
        Self::default()
    }
}
