// PPU (Picture Processing Unit) - Game Boy 圖形處理器

use crate::mmu::EnableState;

/// Tile 數據定址模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileAddressingMode {
    /// 0x8000-0x8FFF，無符號索引 (0-255)
    Mode8000,
    /// 0x8800-0x97FF，帶符號索引 (-128 到 127)，基址 0x9000
    Mode8800,
}

/// 背景/視窗地圖地址
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TileMapAddress {
    /// 0x9800-0x9BFF
    Map9800,
    /// 0x9C00-0x9FFF
    Map9C00,
}

/// 精靈大小
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpriteSize {
    /// 8x8 像素
    Size8x8,
    /// 8x16 像素
    Size8x16,
}

/// 精靈調色板選擇
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpritePalette {
    /// OBP0 調色板
    Obp0,
    /// OBP1 調色板
    Obp1,
}

/// 精靈優先級（與背景的關係）
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpritePriority {
    /// 精靈在背景上方
    AboveBg,
    /// 精靈在背景後方（BG 顏色 1-3 覆蓋精靈）
    BehindBg,
}

/// 精靈翻轉狀態
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpriteFlip {
    /// 不翻轉
    Normal,
    /// 翻轉
    Flipped,
}

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub y_pos: u8,      // Y 位置 (實際位置 = y_pos - 16)
    pub x_pos: u8,      // X 位置 (實際位置 = x_pos - 8)
    pub tile_index: u8, // 圖塊索引
    pub attributes: u8, // 屬性字節
}

impl Sprite {
    pub fn new(y_pos: u8, x_pos: u8, tile_index: u8, attributes: u8) -> Self {
        Sprite {
            y_pos,
            x_pos,
            tile_index,
            attributes,
        }
    }

    // 獲取精靈的實際 Y 位置
    pub fn actual_y(&self) -> i16 {
        self.y_pos as i16 - 16
    }

    // 獲取精靈的實際 X 位置
    pub fn actual_x(&self) -> i16 {
        self.x_pos as i16 - 8
    }

    // 檢查精靈是否在背景後面
    pub fn priority(&self) -> SpritePriority {
        if (self.attributes & 0x80) != 0 {
            SpritePriority::BehindBg
        } else {
            SpritePriority::AboveBg
        }
    }

    // 檢查精靈是否水平翻轉
    pub fn flip_x(&self) -> SpriteFlip {
        if (self.attributes & 0x20) != 0 {
            SpriteFlip::Flipped
        } else {
            SpriteFlip::Normal
        }
    }

    // 檢查精靈是否垂直翻轉
    pub fn flip_y(&self) -> SpriteFlip {
        if (self.attributes & 0x40) != 0 {
            SpriteFlip::Flipped
        } else {
            SpriteFlip::Normal
        }
    }

    // 獲取精靈使用的調色板
    pub fn palette(&self) -> SpritePalette {
        if (self.attributes & 0x10) != 0 {
            SpritePalette::Obp1
        } else {
            SpritePalette::Obp0
        }
    }
}

pub struct Ppu {
    // LCD 控制寄存器
    pub lcdc: u8, // 0xFF40 - LCD 控制
    pub stat: u8, // 0xFF41 - LCD 狀態
    pub scy: u8,  // 0xFF42 - 背景滾動 Y
    pub scx: u8,  // 0xFF43 - 背景滾動 X
    pub ly: u8,   // 0xFF44 - 當前掃描線
    pub lyc: u8,  // 0xFF45 - LY 比較
    pub dma: u8,  // 0xFF46 - DMA 傳輸
    pub bgp: u8,  // 0xFF47 - 背景調色板
    pub obp0: u8, // 0xFF48 - 精靈調色板 0
    pub obp1: u8, // 0xFF49 - 精靈調色板 1
    pub wy: u8,   // 0xFF4A - 視窗 Y 位置
    pub wx: u8,   // 0xFF4B - 視窗 X 位置

    // 內部狀態
    pub mode: LcdMode, // 當前 LCD 模式
    pub dots: u16,     // 點計數器

    // OAM 搜索結果 - 當前掃描線的可見精靈 (最多 10 個)
    pub oam_sprites: Vec<(usize, Sprite)>, // (OAM索引, 精靈)

    // 畫面緩衝區 - 160x144 像素，每個像素 2 位元 (0-3)
    pub framebuffer: Vec<u8>,

    // 內部狀態追蹤 - 用於 STAT 中斷升緣觸發檢測
    pub prev_stat_irq: Option<()>,

    // 視窗內部行計數器 - 追蹤已渲染的視窗行數
    pub window_line_counter: u8,
    // 視窗是否在當前幀被觸發過
    pub window_triggered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LcdMode {
    HBlank = 0,        // 水平空白期
    VBlank = 1,        // 垂直空白期
    OamSearch = 2,     // OAM 搜索
    PixelTransfer = 3, // 像素傳輸
}

struct SpritePixelInfo {
    color: u8,
    color_index: u8,
}

impl Ppu {
    pub fn new() -> Self {
        Ppu {
            lcdc: 0x91, // 預設值
            stat: 0x85, // 預設值
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: 0xE4,  // 預設背景調色板 (0xE4 是標準 DMG 啟動值)
            obp0: 0xFF, // 預設精靈調色板 0
            obp1: 0xFF, // 預設精靈調色板 1
            wy: 0,
            wx: 0,
            mode: LcdMode::OamSearch,
            dots: 0,
            oam_sprites: Vec::new(),
            framebuffer: vec![0; 160 * 144], // 160x144 像素
            prev_stat_irq: None,
            window_line_counter: 0,
            window_triggered: false,
        }
    }

    // 讀取 LCD 寄存器
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.lcdc,
            0xFF41 => self.stat | (self.mode as u8),
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF46 => self.dma,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0xFF,
        }
    }

    // 寫入 LCD 寄存器
    pub fn write_register(&mut self, addr: u16, value: u8, interrupt_flags: &mut u8) {
        match addr {
            0xFF40 => {
                let old_lcdc = self.lcdc;
                self.lcdc = value;
                // 處理 LCD 啟用/關閉狀態變化
                match ((old_lcdc & 0x80) != 0, (value & 0x80) != 0) {
                    (true, false) => {
                        // LCD 從開啟變為關閉：LY 和 dots 歸零，模式設為 HBlank
                        self.ly = 0;
                        self.dots = 0;
                        self.mode = LcdMode::HBlank;
                        self.stat &= 0xFC;
                    }
                    (false, true) => {
                        // LCD 從關閉變為開啟：重置為 OAM 搜索模式
                        self.mode = LcdMode::OamSearch;
                        self.dots = 0;
                    }
                    _ => {} // 狀態沒變，不做任何事
                }
            }
            0xFF41 => {
                // 位 0-2 只讀，位 7 始終為 1
                self.stat = (self.stat & 0x07) | (value & 0x78) | 0x80;
                self.update_stat(interrupt_flags);
            }
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => {} // LY 是只讀的
            0xFF45 => {
                self.lyc = value;
                self.update_stat(interrupt_flags);
            }
            0xFF46 => self.dma = value,
            0xFF47 => self.bgp = value,
            0xFF48 => self.obp0 = value,
            0xFF49 => self.obp1 = value,
            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,
            _ => {}
        }
    }

    fn update_stat(&mut self, interrupt_flags: &mut u8) {
        // 更新 LYC == LY 標誌 (Bit 2)
        if self.ly == self.lyc {
            self.stat |= 0x04;
        } else {
            self.stat &= !0x04;
        }

        // 檢查 STAT 中斷條件 - 使用 Option 代替 bool
        let mut irq: Option<()> = None;

        // LYC == LY 中斷 (Bit 6)
        if (self.stat & 0x40) != 0 && (self.stat & 0x04) != 0 {
            irq = Some(());
        }

        // Mode 中斷
        match self.mode {
            LcdMode::HBlank => {
                if (self.stat & 0x08) != 0 {
                    irq = Some(());
                }
            }
            LcdMode::VBlank => {
                if (self.stat & 0x10) != 0 {
                    irq = Some(());
                }
            }
            LcdMode::OamSearch => {
                if (self.stat & 0x20) != 0 {
                    irq = Some(());
                }
            }
            _ => {}
        }

        // 升緣觸發中斷 - 只在從無中斷變為有中斷時觸發
        if irq.is_some() && self.prev_stat_irq.is_none() {
            *interrupt_flags |= 0x02; // LCD STAT 中斷 (Bit 1)
        }
        self.prev_stat_irq = irq;
    }

    fn change_mode(&mut self, mode: LcdMode, interrupt_flags: &mut u8) {
        self.mode = mode;
        self.stat = (self.stat & 0xFC) | (mode as u8);

        if mode == LcdMode::OamSearch {
            self.oam_sprites.clear();
        }

        self.update_stat(interrupt_flags);
    }

    // 從 VRAM 讀取圖塊資料
    pub fn read_tile_data(
        &self,
        mmu: &crate::mmu::Mmu,
        tile_index: u8,
        tile_line: u8,
        addressing_mode: TileAddressingMode,
    ) -> (u8, u8) {
        let tile_addr = match addressing_mode {
            TileAddressingMode::Mode8000 => {
                // 無符號索引，從 0x8000 開始
                0x8000 + (tile_index as u16 * 16)
            }
            TileAddressingMode::Mode8800 => {
                // 帶符號的圖塊索引 (-128 到 127), 基址為 0x9000
                let signed_index = tile_index as i8 as i32;
                (0x9000i32 + (signed_index * 16)) as u16
            }
        };

        let line_addr = tile_addr + (tile_line as u16 * 2);
        let low_byte = mmu.read_byte(line_addr);
        let high_byte = mmu.read_byte(line_addr + 1);

        (low_byte, high_byte)
    }

    // 從 OAM 讀取精靈資料
    pub fn read_sprite_data(&self, mmu: &crate::mmu::Mmu, sprite_index: usize) -> (u8, u8, u8, u8) {
        let base_addr = 0xFE00 + (sprite_index * 4) as u16;
        let y_pos = mmu.read_byte(base_addr);
        let x_pos = mmu.read_byte(base_addr + 1);
        let tile_index = mmu.read_byte(base_addr + 2);
        let attributes = mmu.read_byte(base_addr + 3);

        (y_pos, x_pos, tile_index, attributes)
    }

    // 從背景地圖讀取圖塊索引
    pub fn read_bg_tile_index(
        &self,
        mmu: &crate::mmu::Mmu,
        x: u8,
        y: u8,
        map_addr: TileMapAddress,
    ) -> u8 {
        let map_base = match map_addr {
            TileMapAddress::Map9800 => 0x9800,
            TileMapAddress::Map9C00 => 0x9C00,
        };
        let tile_x = x / 8;
        let tile_y = y / 8;
        let tile_addr = map_base + (tile_y as u16 * 32) + tile_x as u16;

        mmu.read_byte(tile_addr)
    }

    // 從視窗地圖讀取圖塊索引
    pub fn read_window_tile_index(
        &self,
        mmu: &crate::mmu::Mmu,
        x: u8,
        y: u8,
        map_addr: TileMapAddress,
    ) -> u8 {
        let map_base = match map_addr {
            TileMapAddress::Map9800 => 0x9800,
            TileMapAddress::Map9C00 => 0x9C00,
        };
        let tile_x = x / 8;
        let tile_y = y / 8;
        let tile_addr = map_base + (tile_y as u16 * 32) + tile_x as u16;

        mmu.read_byte(tile_addr)
    }

    // 檢查精靈在當前掃描線上的可見性
    fn check_sprite_visibility(&mut self, mmu: &crate::mmu::Mmu, sprite_index: usize) {
        if self.oam_sprites.len() >= 10 {
            return; // 每條掃描線最多 10 個精靈
        }

        let (y_pos, x_pos, tile_index, attributes) = self.read_sprite_data(mmu, sprite_index);

        // 檢查精靈是否完全在螢幕外
        // x_pos = 0 表示精靈完全在螢幕左側 (actual_x = -8)
        // x_pos >= 168 表示精靈完全在螢幕右側 (actual_x >= 160)
        // 注意：x_pos = 0 的精靈完全不可見，但 x_pos = 1-7 的精靈部分可見
        if x_pos == 0 {
            return; // 精靈在螢幕 X 範圍外（完全隱藏）
        }

        let sprite = Sprite::new(y_pos, x_pos, tile_index, attributes);

        let sprite_height = match self.get_sprite_size() {
            SpriteSize::Size8x8 => 8,
            SpriteSize::Size8x16 => 16,
        };
        let sprite_y = sprite.actual_y();

        // 檢查精靈是否與當前掃描線相交
        if self.ly as i16 >= sprite_y && (self.ly as i16) < sprite_y + sprite_height as i16 {
            self.oam_sprites.push((sprite_index, sprite));
        }
    }

    // 在 OAM 搜索結束後對精靈進行排序
    fn sort_sprites(&mut self) {
        // DMG 優先級規則：
        // 1. X 座標較小的精靈優先（在前面）
        // 2. X 座標相同時，OAM 索引較小的優先
        self.oam_sprites.sort_by(|a, b| {
            let x_cmp = a.1.x_pos.cmp(&b.1.x_pos);
            if x_cmp == std::cmp::Ordering::Equal {
                a.0.cmp(&b.0) // OAM 索引
            } else {
                x_cmp
            }
        });
    }

    // 將調色板值轉換為顏色
    pub fn get_palette_color(&self, palette: u8, color_index: u8) -> u8 {
        // 從調色板中提取指定索引的顏色 (0-3)
        let shift = color_index * 2;
        (palette >> shift) & 0x03
    }

    // 獲取背景調色板顏色
    pub fn get_bg_color(&self, color_index: u8) -> u8 {
        self.get_palette_color(self.bgp, color_index)
    }

    // 獲取精靈調色板顏色
    pub fn get_sprite_color(&self, color_index: u8, palette: SpritePalette) -> u8 {
        let palette_value = match palette {
            SpritePalette::Obp0 => self.obp0,
            SpritePalette::Obp1 => self.obp1,
        };
        self.get_palette_color(palette_value, color_index)
    }

    // 檢查 LCD 是否啟用
    pub fn lcd_state(&self) -> EnableState {
        if (self.lcdc & 0x80) != 0 {
            EnableState::Enabled
        } else {
            EnableState::Disabled
        }
    }

    // 檢查背景是否啟用
    pub fn bg_state(&self) -> EnableState {
        if (self.lcdc & 0x01) != 0 {
            EnableState::Enabled
        } else {
            EnableState::Disabled
        }
    }

    // 檢查精靈是否啟用
    pub fn sprite_state(&self) -> EnableState {
        if (self.lcdc & 0x02) != 0 {
            EnableState::Enabled
        } else {
            EnableState::Disabled
        }
    }

    // 檢查視窗是否啟用
    pub fn window_state(&self) -> EnableState {
        if (self.lcdc & 0x20) != 0 {
            EnableState::Enabled
        } else {
            EnableState::Disabled
        }
    }

    // 獲取背景圖塊定址模式
    pub fn get_tile_addressing_mode(&self) -> TileAddressingMode {
        match self.lcdc & 0x10 {
            0 => TileAddressingMode::Mode8800,
            _ => TileAddressingMode::Mode8000,
        }
    }

    // 獲取背景地圖地址
    pub fn get_bg_map_address(&self) -> TileMapAddress {
        match self.lcdc & 0x08 {
            0 => TileMapAddress::Map9800,
            _ => TileMapAddress::Map9C00,
        }
    }

    // 獲取視窗地圖地址
    pub fn get_window_map_address(&self) -> TileMapAddress {
        match self.lcdc & 0x40 {
            0 => TileMapAddress::Map9800,
            _ => TileMapAddress::Map9C00,
        }
    }

    // 獲取精靈大小
    pub fn get_sprite_size(&self) -> SpriteSize {
        match self.lcdc & 0x04 {
            0 => SpriteSize::Size8x8,
            _ => SpriteSize::Size8x16,
        }
    }

    // PPU 主時鐘滴答 - 每個 T-狀態調用一次
    pub fn tick(&mut self, mmu: &crate::mmu::Mmu, interrupt_flags: &mut u8) {
        if self.lcd_state() == EnableState::Disabled {
            return;
        }

        self.dots += 1;

        match self.mode {
            LcdMode::OamSearch => {
                // 每 2 個點檢查一個精靈
                // dots 從 1 開始，所以 dots=1,2 檢查 sprite 0，dots=3,4 檢查 sprite 1，以此類推
                if self.dots.is_multiple_of(2) {
                    let sprite_index = (self.dots / 2 - 1) as usize;
                    if sprite_index < 40 {
                        self.check_sprite_visibility(mmu, sprite_index);
                    }
                }

                if self.dots >= 80 {
                    // OAM 搜索結束後排序精靈
                    self.sort_sprites();
                    self.change_mode(LcdMode::PixelTransfer, interrupt_flags);
                }
            }
            LcdMode::PixelTransfer => {
                if self.dots >= 252 {
                    self.render_scanline(mmu);
                    self.change_mode(LcdMode::HBlank, interrupt_flags);
                }
            }
            LcdMode::HBlank => {
                if self.dots >= 456 {
                    self.dots = 0;
                    self.ly += 1;

                    if self.ly >= 144 {
                        self.change_mode(LcdMode::VBlank, interrupt_flags);
                        *interrupt_flags |= 0x01; // VBlank 中斷
                    } else {
                        self.change_mode(LcdMode::OamSearch, interrupt_flags);
                    }
                }
            }
            LcdMode::VBlank => {
                if self.dots >= 456 {
                    self.dots = 0;
                    self.ly += 1;

                    if self.ly >= 154 {
                        self.ly = 0;
                        // 新幀開始時重置視窗行計數器
                        self.window_line_counter = 0;
                        self.window_triggered = false;
                        self.change_mode(LcdMode::OamSearch, interrupt_flags);
                    } else {
                        self.update_stat(interrupt_flags);
                    }
                }
            }
        }
    }

    // 渲染當前掃描線
    fn render_scanline(&mut self, mmu: &crate::mmu::Mmu) {
        if self.ly >= 144 {
            return; // VBlank 期間不渲染
        }

        // 追蹤這條掃描線是否使用了視窗
        let mut window_used_this_line = false;

        for x in 0..160u8 {
            let (color, used_window) = self.get_pixel_color(mmu, x, self.ly);
            self.framebuffer[(self.ly as usize * 160) + x as usize] = color;
            if used_window {
                window_used_this_line = true;
            }
        }

        // 如果這條掃描線使用了視窗，增加視窗行計數器
        if window_used_this_line {
            self.window_line_counter += 1;
        }
    }

    // 獲取指定像素的顏色，返回 (顏色, 是否使用視窗)
    fn get_pixel_color(&self, mmu: &crate::mmu::Mmu, x: u8, y: u8) -> (u8, bool) {
        let (bg_color, bg_color_idx, used_window) = self.get_bg_pixel_color(mmu, x, y);

        // 如果精靈未啟用，直接返回背景顏色
        if self.sprite_state() == EnableState::Disabled {
            return (bg_color, used_window);
        }

        // 檢查精靈像素 - 按排序後的順序（X 座標較小的優先）
        for (_, sprite) in self.oam_sprites.iter() {
            if let Some(sprite_info) = self.get_sprite_pixel_info(mmu, sprite, x, y) {
                // 顏色索引 0 是透明的
                if sprite_info.color_index == 0 {
                    continue;
                }

                // 檢查優先級：如果設定了 "behind BG" 且背景顏色索引不是 0，則背景優先
                if sprite.priority() == SpritePriority::BehindBg && bg_color_idx != 0 {
                    return (bg_color, used_window);
                }

                return (sprite_info.color, used_window);
            }
        }

        (bg_color, used_window)
    }

    // 獲取背景像素顏色和索引，返回 (顏色, 顏色索引, 是否使用視窗)
    fn get_bg_pixel_color(&self, mmu: &crate::mmu::Mmu, x: u8, y: u8) -> (u8, u8, bool) {
        if self.bg_state() == EnableState::Disabled {
            return (0, 0, false); // 背景關閉時返回顏色 0
        }

        // 計算背景像素
        let bg_x = ((x as u16 + self.scx as u16) % 256) as u8;
        let bg_y = ((y as u16 + self.scy as u16) % 256) as u8;

        // 確定是否在視窗區域
        // 視窗條件：視窗啟用 + LY >= WY + X >= WX-7
        let in_window =
            self.window_state() == EnableState::Enabled && (y >= self.wy) && (x + 7 >= self.wx);

        let (tile_x, tile_y, map_addr) = if in_window {
            // 視窗座標 - 使用內部視窗行計數器
            let window_x = x + 7 - self.wx;
            let window_y = self.window_line_counter; // 使用視窗行計數器
            (window_x, window_y, self.get_window_map_address())
        } else {
            // 背景座標
            (bg_x, bg_y, self.get_bg_map_address())
        };

        // 獲取圖塊索引
        let tile_index = if in_window {
            self.read_window_tile_index(mmu, tile_x, tile_y, map_addr)
        } else {
            self.read_bg_tile_index(mmu, tile_x, tile_y, map_addr)
        };

        // 獲取圖塊資料
        let tile_line = tile_y % 8;
        let addressing_mode = self.get_tile_addressing_mode();
        let (low_byte, high_byte) =
            self.read_tile_data(mmu, tile_index, tile_line, addressing_mode);

        // 提取像素顏色
        let pixel_x = tile_x % 8;
        let bit_index = 7 - pixel_x;
        let low_bit = (low_byte >> bit_index) & 0x01;
        let high_bit = (high_byte >> bit_index) & 0x01;
        let color_index = (high_bit << 1) | low_bit;

        // 應用調色板
        (self.get_bg_color(color_index), color_index, in_window)
    }

    // 獲取精靈在指定位置的像素資訊
    fn get_sprite_pixel_info(
        &self,
        mmu: &crate::mmu::Mmu,
        sprite: &Sprite,
        x: u8,
        y: u8,
    ) -> Option<SpritePixelInfo> {
        let sprite_x = sprite.actual_x();
        let sprite_y = sprite.actual_y();
        let sprite_height = match self.get_sprite_size() {
            SpriteSize::Size8x8 => 8,
            SpriteSize::Size8x16 => 16,
        };

        // 檢查像素是否在精靈範圍內
        if (x as i16) < sprite_x || (x as i16) >= sprite_x + 8 {
            return None;
        }
        if (y as i16) < sprite_y || (y as i16) >= sprite_y + sprite_height as i16 {
            return None;
        }

        // 計算精靈內部的相對座標
        let rel_x = (x as i16 - sprite_x) as u8;
        let mut rel_y = (y as i16 - sprite_y) as u8;

        // 處理垂直翻轉
        if sprite.flip_y() == SpriteFlip::Flipped {
            rel_y = sprite_height - 1 - rel_y;
        }

        // 確定圖塊索引 (8x16 模式下 bit 0 被忽略)
        let tile_index = match self.get_sprite_size() {
            SpriteSize::Size8x16 => {
                if rel_y >= 8 {
                    sprite.tile_index | 0x01
                } else {
                    sprite.tile_index & 0xFE
                }
            }
            SpriteSize::Size8x8 => sprite.tile_index,
        };

        // 獲取圖塊資料 (精靈總是使用 0x8000-0x8FFF 圖塊集)
        let tile_line = rel_y % 8;
        let (low_byte, high_byte) =
            self.read_tile_data(mmu, tile_index, tile_line, TileAddressingMode::Mode8000);

        // 提取像素顏色索引 - 處理水平翻轉
        let bit_index = if sprite.flip_x() == SpriteFlip::Flipped {
            rel_x // 翻轉時從右到左讀取（bit 0 是最右邊）
        } else {
            7 - rel_x // 正常時從左到右讀取（bit 7 是最左邊）
        };
        let low_bit = (low_byte >> bit_index) & 0x01;
        let high_bit = (high_byte >> bit_index) & 0x01;
        let color_index = (high_bit << 1) | low_bit;

        // 應用精靈調色板
        let color = self.get_sprite_color(color_index, sprite.palette());

        Some(SpritePixelInfo { color, color_index })
    }

    // 獲取畫面緩衝區的引用
    pub fn get_framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }
}
