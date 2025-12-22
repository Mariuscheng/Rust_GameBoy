use crate::core::mmu::mmu::MMU;

/// PPU (像素處理單元) 負責 Game Boy 的圖形渲染
pub struct PPU {
    pub mmu: *mut MMU, // 直接持有裸指標，僅供內部存取，不負責釋放
    pub lcd_enabled: bool,
    pub framebuffer: Vec<(u8, u8, u8)>,
    // [LOG] 每次 frame buffer 更新時插入：
    // log_to_file!("[PPU] framebuffer updated, size={}", self.framebuffer.len());
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub scx: u8,
    pub scy: u8,
    pub wy: u8,
    pub wx: u8,
    pub lcdc: u8,
    pub last_frame_time: std::time::Instant,
    pub fps_counter: u32,
    pub mode: u8,
    pub ly: u8,
    pub lyc: u8,
    pub stat: u8,
    pub dots: u32,
    pub oam: [u8; 0xA0],
    pub vram: *mut crate::core::ppu::vram::VRAM, // 直接持有裸指標
}
impl PPU {
    /// 取得 VRAM 內部像素資料（unsafe，僅供 dump/debug 用）
    pub fn get_vram_data(&self) -> &[u8] {
        unsafe { &(*self.vram).data }
    }
    /// 取得目前畫面像素索引（供 SDL display 使用）
    pub fn get_framebuffer_indices(&self) -> Vec<u8> {
        // 若需單色索引，回傳 R 分量
        self.framebuffer.iter().map(|rgb| rgb.0).collect()
    }
    /// 執行一個 PPU step（等同 tick）
    pub fn step(&mut self) {
        self.tick();
    }

    /// 連續執行 n 次 PPU tick
    pub fn run(&mut self, cycles: usize) {
        for _ in 0..cycles {
            self.tick();
        }
    }
    pub fn new(mmu: &mut MMU, vram: &mut crate::core::ppu::vram::VRAM) -> Self {
        Self {
            mmu: mmu as *mut MMU,
            lcd_enabled: false,
            framebuffer: vec![(224, 248, 208); 160 * 144],
            bgp: 0xE4,
            obp0: 0xFF,
            obp1: 0xFF,
            scx: 0,
            scy: 0,
            wy: 0,
            wx: 0,
            lcdc: 0xB1, // 啟用 window (bit 5)
            last_frame_time: std::time::Instant::now(),
            fps_counter: 0,
            mode: 0,
            ly: 0,
            lyc: 0,
            stat: 0,
            dots: 0,
            oam: [0; 0xA0],
            vram: vram as *mut crate::core::ppu::vram::VRAM,
        }
    }

    pub fn set_lcd_enabled(&mut self, enabled: bool) {
        self.lcd_enabled = enabled;
    }

    pub fn is_lcd_enabled(&self) -> bool {
        self.lcd_enabled
    }

    /// 執行一個 PPU 週期 (tick)
    pub fn tick(&mut self) {
        // === PPU 主迴圈（彩色化，BG/Window/Sprite palette 分離）===
        // 動態讀取 MMU palette register（0xFF47/0xFF48/0xFF49）
        if let Some(mmu) = unsafe { self.mmu.as_mut() } {
            self.bgp = mmu.read_byte(0xFF47).unwrap_or(0xE4);
            self.obp0 = mmu.read_byte(0xFF48).unwrap_or(0xFF);
            self.obp1 = mmu.read_byte(0xFF49).unwrap_or(0xFF);
        }
        let w = 160;
        let h = 144;
        let vram_ref = unsafe { &*self.vram };
        self.framebuffer.clear();
        self.framebuffer.resize(w * h, (224, 248, 208)); // 白底
        if self.lcdc & 0x80 == 0 {
            // LCDC bit 7: display disable
            return;
        }
        let bg_enable = self.lcdc & 0x01 != 0;
        let sprite_enable = self.lcdc & 0x02 != 0;
        let window_enable = self.lcdc & 0x20 != 0;
        // --- Palette設置：Game Boy 綠色系（彩色化）---
        let color_map: [(u8, u8, u8); 4] = [
            (224, 248, 208), // 白
            (136, 192, 112), // 淺綠
            (52, 104, 86),   // 深綠
            (8, 24, 32),     // 黑
        ];
        // BG palette (BGP)
        let bgp = if self.bgp == 0 { 0xE4 } else { self.bgp };
        let bg_palette = [
            (bgp >> 0) & 0x03,
            (bgp >> 2) & 0x03,
            (bgp >> 4) & 0x03,
            (bgp >> 6) & 0x03,
        ];
        // Sprite palettes (OBP0/OBP1)
        let obp0 = self.obp0;
        let obp1 = self.obp1;
        let sprite_palette0 = [
            (obp0 >> 0) & 0x03,
            (obp0 >> 2) & 0x03,
            (obp0 >> 4) & 0x03,
            (obp0 >> 6) & 0x03,
        ];
        let sprite_palette1 = [
            (obp1 >> 0) & 0x03,
            (obp1 >> 2) & 0x03,
            (obp1 >> 4) & 0x03,
            (obp1 >> 6) & 0x03,
        ];
        let bg_tile_map_addr = if self.lcdc & 0x08 != 0 {
            0x1C00
        } else {
            0x1800
        };
        let bg_tile_data_addr = if self.lcdc & 0x10 != 0 {
            0x0000
        } else {
            0x0800
        };
        for y in 0..h {
            let scy = self.scy as usize;
            let scx = self.scx as usize;
            let ly = (y + scy) & 0xFF;
            for x in 0..w {
                let mut rgb = color_map[0];
                // --- Window decode（優先）---
                if window_enable
                    && y >= self.wy as usize
                    && x >= (self.wx as usize).saturating_sub(7)
                {
                    let win_tile_map_addr = if self.lcdc & 0x40 != 0 {
                        0x1C00
                    } else {
                        0x1800
                    };
                    let win_y = y - self.wy as usize;
                    let win_x = x - (self.wx as usize).saturating_sub(7);
                    let tile_map_x = win_x / 8;
                    let tile_map_y = win_y / 8;
                    let tile_map_index = tile_map_y * 32 + tile_map_x;
                    let tile_map_addr = win_tile_map_addr + tile_map_index;
                    let tile_num = vram_ref.data.get(tile_map_addr).copied().unwrap_or(0);
                    let tile_addr = if self.lcdc & 0x10 != 0 {
                        0x0000 + (tile_num as usize) * 16
                    } else {
                        let idx = tile_num as i8 as i16;
                        (0x0800 as isize + (idx * 16) as isize) as usize
                    };
                    let tile_y = win_y % 8;
                    let byte1 = vram_ref
                        .data
                        .get(tile_addr + tile_y * 2)
                        .copied()
                        .unwrap_or(0);
                    let byte2 = vram_ref
                        .data
                        .get(tile_addr + tile_y * 2 + 1)
                        .copied()
                        .unwrap_or(0);
                    let bit = 7 - (win_x % 8);
                    let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
                    let pal_idx = bg_palette.get(color_num as usize).copied().unwrap_or(0);
                    rgb = color_map
                        .get(pal_idx as usize)
                        .copied()
                        .unwrap_or(color_map[0]);
                    if y == 0 && x < 32 {
                        use std::fs::OpenOptions;
                        use std::io::Write;
                        let mut log = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("logs/emulator.log")
                            .unwrap();
                        writeln!(log, "WIN[{}] tile_map_addr={:04X} tile_num={:02X} tile_addr={:04X} color_num={} pal_idx={} rgb={:?}",
                            x, tile_map_addr, tile_num, tile_addr, color_num, pal_idx, rgb).ok();
                    }
                } else {
                    // BG tile decode
                    let lx = (x + scx) & 0xFF;
                    let tile_map_x = (lx / 8) % 32;
                    let tile_map_y = (ly / 8) % 32;
                    let tile_map_index = tile_map_y * 32 + tile_map_x;
                    let tile_map_addr = bg_tile_map_addr + tile_map_index;
                    let tile_num = vram_ref.data.get(tile_map_addr).copied().unwrap_or(0);
                    let tile_addr = if self.lcdc & 0x10 != 0 {
                        bg_tile_data_addr + (tile_num as usize) * 16
                    } else {
                        let idx = tile_num as i8 as i16;
                        (bg_tile_data_addr as isize + (idx * 16) as isize) as usize
                    };
                    let tile_y = ly % 8;
                    let byte1 = vram_ref
                        .data
                        .get(tile_addr + tile_y * 2)
                        .copied()
                        .unwrap_or(0);
                    let byte2 = vram_ref
                        .data
                        .get(tile_addr + tile_y * 2 + 1)
                        .copied()
                        .unwrap_or(0);
                    let bit = 7 - (lx % 8);
                    let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
                    let pal_idx = bg_palette.get(color_num as usize).copied().unwrap_or(0);
                    rgb = color_map
                        .get(pal_idx as usize)
                        .copied()
                        .unwrap_or(color_map[0]);
                    if y == 0 && x < 32 {
                        use std::fs::OpenOptions;
                        use std::io::Write;
                        let mut log = OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open("logs/emulator.log")
                            .unwrap();
                        writeln!(log, "BG[{}] tile_map_addr={:04X} tile_num={:02X} tile_addr={:04X} color_num={} pal_idx={} rgb={:?}",
                            x, tile_map_addr, tile_num, tile_addr, color_num, pal_idx, rgb).ok();
                    }
                }
                // --- Sprite decode（疊加）---
                if sprite_enable {
                    for i in 0..40 {
                        let oam_idx = i * 4;
                        if oam_idx + 3 >= self.oam.len() {
                            break;
                        }
                        let sprite_y = self.oam[oam_idx] as isize - 16;
                        let sprite_x = self.oam[oam_idx + 1] as isize - 8;
                        let tile_idx = self.oam[oam_idx + 2] as usize;
                        let attr = self.oam[oam_idx + 3];
                        let use_palette = if (attr & 0x10) == 0 {
                            &sprite_palette0
                        } else {
                            &sprite_palette1
                        };
                        let sprite_color_map: [(u8, u8, u8); 4] = [
                            (224, 248, 208), // 白
                            (136, 192, 112), // 淺綠
                            (52, 104, 86),   // 深綠
                            (8, 24, 32),     // 黑
                        ];
                        let sprite_line = (y as isize - sprite_y) as usize;
                        if sprite_line >= 8 {
                            continue;
                        }
                        let tile_addr = tile_idx * 16 + sprite_line * 2;
                        let byte1 = vram_ref.data.get(tile_addr).copied().unwrap_or(0);
                        let byte2 = vram_ref.data.get(tile_addr + 1).copied().unwrap_or(0);
                        for sx in 0..8 {
                            let bit = 7 - sx;
                            let color_num = ((byte2 >> bit) & 1) << 1 | ((byte1 >> bit) & 1);
                            if color_num == 0 {
                                continue;
                            }
                            let px = sprite_x + sx;
                            if px < 0 || px >= w as isize {
                                continue;
                            }
                            if px as usize == x {
                                let bg_priority = (attr & 0x80) != 0;
                                if !bg_priority || rgb == color_map[0] {
                                    let pal_idx =
                                        use_palette.get(color_num as usize).copied().unwrap_or(0);
                                    let rgb_sprite = sprite_color_map
                                        .get(pal_idx as usize)
                                        .copied()
                                        .unwrap_or(sprite_color_map[0]);
                                    rgb = rgb_sprite;
                                    if y == 0 && x < 32 {
                                        use std::fs::OpenOptions;
                                        use std::io::Write;
                                        let mut log = OpenOptions::new()
                                            .create(true)
                                            .append(true)
                                            .open("logs/emulator.log")
                                            .unwrap();
                                        writeln!(log, "SPRITE[{}] oam_idx={} tile_idx={} tile_addr={:04X} color_num={} pal_idx={} rgb={:?}",
                                            x, oam_idx, tile_idx, tile_addr, color_num, pal_idx, rgb).ok();
                                    }
                                }
                            }
                        }
                    }
                }
                // Debug: 前 32 像素 rgb
                if y == 0 && x < 32 {
                    use std::fs::OpenOptions;
                    use std::io::Write;
                    let mut log = OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/emulator.log")
                        .unwrap();
                    writeln!(
                        log,
                        "[PPU] y={} x={} rgb=({},{},{})",
                        y, x, rgb.0, rgb.1, rgb.2
                    )
                    .ok();
                }
                self.framebuffer[y * w + x] = rgb;
            }
        }
    }
    /// 取得目前畫面緩衝區
    pub fn get_framebuffer(&self) -> &[(u8, u8, u8)] {
        // 若不足則回傳全白畫面
        if self.framebuffer.len() < 160 * 144 {
            static WHITE_FB: [(u8, u8, u8); 160 * 144] = [(224, 248, 208); 160 * 144];
            &WHITE_FB
        } else {
            &self.framebuffer
        }
    }
}
