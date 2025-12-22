#![allow(dead_code)]
//! 背景渲染與掃描線產生

use crate::core::mmu::mmu::MMU;

#[derive(Debug)]
pub struct BackgroundRenderer {
    tile_data: Vec<u8>,
    tile_map: Vec<u8>,
}

impl BackgroundRenderer {
    pub fn new() -> Self {
        Self {
            tile_data: vec![0; 0x1800],
            tile_map: vec![0; 0x800],
        }
    }

    /// 產生一條掃描線的背景像素色碼 (0~3)
    pub fn render_line(&self, line: u8, mmu: &MMU) -> Vec<u8> {
        let mut result = vec![0u8; 160];
        // TODO: 根據 MMU 讀取 VRAM/registers，產生正確像素色碼
        result
    }
}

pub fn render_background(ppu: &mut super::ppu::PPU) {
    // TODO: 根據 VRAM, LCDC, SCY, SCX 等渲染背景
}
