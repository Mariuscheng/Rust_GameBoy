// Bus IO (Joypad/Timer/PPU) 相關欄位與邏輯

use crate::GB::types::*;
use std::fmt::Write;
#[allow(dead_code)]
pub struct BusIO {
    pub ie: u8,
    pub ifl: u8,
    pub p1: u8,
    pub p1_sel: u8,
    pub joyp_dpad: u8,
    pub joyp_btns: u8,
    pub joyp_dpad_prev: u8,
    pub joyp_btns_prev: u8,
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    pub div_total: u32,
    pub lcdc: u8,
    pub stat_w: u8,
    pub scy: u8,
    pub scx: u8,
    pub ly: u8,
    pub lyc: u8,
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
    pub wy: u8,
    pub wx: u8,
    pub ppu_line_cycle: u32,
    pub ppu_mode: u8,
    pub lcd_on_delay: u32,
    pub framebuffer: [u8; 160 * 144],
    // Serial
    pub sb: u8,
    pub sc: u8,
    pub div_counter: u16,
    pub timer_accum: u32,
    pub div_sub: u8,
    pub tima_reload_delay: u32,
    pub win_line: u8,
    pub dbg_window_log_count: u32,
    pub dbg_pixel_log_count: u32,
    pub line_base_scx: u8,
    pub line_base_scy: u8,
    pub line_base_wx: u8,
    pub line_base_bgp: u8,
    pub win_tilemap_base: u16,
    pub win_signed: bool,
    //pub win_latched: bool,
    pub scan_events: Vec<RegEvent>,
    pub dbg_lcdc_first_write_done: bool,
    pub dbg_vram_first_write_done: bool,
    pub dma_active: bool,
    pub dma_pos: u16,
    pub dma_start_delay: u32,
    pub dma_cycle_accum: u32,
    pub dma_accum: u32,
    pub dma_src_base: u16,
    pub debug_enabled: bool,
}

impl BusIO {
    pub fn new() -> Self {
        Self {
            ie: 0x00,
            ifl: 0x00,
            p1: 0xFF,
            p1_sel: 0x30,
            joyp_dpad: 0x0F,
            joyp_btns: 0x0F,
            joyp_dpad_prev: 0x0F,
            joyp_btns_prev: 0x0F,
            div: 0x00,
            tima: 0x00,
            tma: 0x00,
            tac: 0x00,
            lcdc: 0x00,
            stat_w: 0x00,
            scy: 0x00,
            scx: 0x00,
            ly: 0x00,
            lyc: 0x00,
            bgp: 0xFC,
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0x00,
            wx: 0x00,
            ppu_line_cycle: 0,
            ppu_mode: 2,
            lcd_on_delay: 0,
            framebuffer: [0; 160 * 144],
            sb: 0,
            sc: 0,
            div_counter: 0,
            div_total: 0,
            timer_accum: 0,
            div_sub: 0,
            tima_reload_delay: 0,
            win_line: 0,
            dbg_window_log_count: 0,
            dbg_pixel_log_count: 0,
            line_base_scx: 0,
            line_base_scy: 0,
            line_base_wx: 0,
            line_base_bgp: 0xFC,
            win_tilemap_base: 0x9800,
            win_signed: true,
            //win_latched: false,
            scan_events: Vec::with_capacity(64),
            dbg_lcdc_first_write_done: false,
            dbg_vram_first_write_done: false,
            dma_active: false,
            dma_pos: 0,
            dma_start_delay: 0,
            dma_cycle_accum: 0,
            dma_accum: 0,
            dma_src_base: 0,
            debug_enabled: std::env::var("GB_DEBUG_PPU").is_ok(),
        }
    }
    // TODO: 移植 PPU、Timer、Joypad、DMA、framebuffer 等方法
    pub fn update_joypad(&mut self, dpad: u8, btns: u8) {
        // Check for falling edges (press events) to trigger interrupt
        let dpad_pressed = (self.joyp_dpad_prev & !dpad) & 0x0F;
        let btns_pressed = (self.joyp_btns_prev & !btns) & 0x0F;
        if dpad_pressed != 0 || btns_pressed != 0 {
            self.ifl |= 0x10; // Joypad interrupt
        }
        self.joyp_dpad_prev = self.joyp_dpad;
        self.joyp_btns_prev = self.joyp_btns;
        self.joyp_dpad = dpad;
        self.joyp_btns = btns;
    }
}

impl BusIO {
    // 這裡可放原本 BUS.rs 內所有 IO 相關方法
    pub fn step(
        &mut self,
        cycles: u64,
        mem: &mut crate::GB::bus_mem::BusMem,
        _apu: &mut crate::GB::bus_apu::BusAPU,
    ) {
        let mut remaining = cycles;
        while remaining > 0 {
            let c = std::cmp::min(remaining, u32::MAX as u64) as u32;

            // 更新 DIV (每 256 CPU cycles 會加 1) 與 TIMA (簡化：若啟用則每 1024 cycles 加 1)
            // Update DIV using a total-cycle counter so we can detect input bit edges.
            self.div_total = self.div_total.wrapping_add(c as u32);
            while self.div_total >= 256 {
                self.div_total -= 256;
                self.div = self.div.wrapping_add(1);
            }
            if (self.tac & 0x04) != 0 {
                // Calculate timer period based on TAC bits 0-1
                let period = match self.tac & 0x03 {
                    0 => 1024, // 4096 Hz
                    1 => 16,   // 262144 Hz
                    2 => 64,   // 65536 Hz
                    3 => 256,  // 16384 Hz
                    _ => unreachable!(),
                };
                // accumulate CPU cycles for TIMA
                self.timer_accum = self.timer_accum.wrapping_add(c as u32);
                while self.timer_accum >= period {
                    self.timer_accum -= period;
                    // If a previous overflow is pending reload, ignore further increments
                    if self.tima_reload_delay > 0 {
                        // do nothing while waiting for reload
                        continue;
                    }
                    if self.tima == 0xFF {
                        // Overflow: TIMA becomes 0 immediately, but TMA is loaded
                        // only after 4 CPU cycles. Request of interrupt also occurs
                        // when the reload happens.
                        self.tima = 0;
                        self.tima_reload_delay = 4;
                    } else {
                        self.tima = self.tima.wrapping_add(1);
                    }
                }
                // Handle TIMA reload delay countdown within this chunk
                if self.tima_reload_delay > 0 {
                    if (c as u32) >= self.tima_reload_delay {
                        // reload occurs during this step
                        self.tima_reload_delay = 0;
                        self.tima = self.tma;
                        self.ifl |= 0x04; // TIMA overflow interrupt
                    } else {
                        self.tima_reload_delay -= c as u32;
                    }
                }
            }

            // DMA 啟動延遲處理 & 簡化 OAM 複製 (不做來源 gating)
            if self.dma_active {
                if self.dma_start_delay > 0 {
                    if c >= self.dma_start_delay {
                        self.dma_start_delay = 0;
                    } else {
                        self.dma_start_delay -= c;
                    }
                } else {
                    // 每 4 cycles 複製一筆 (簡化)
                    self.dma_accum += c as u32;
                    while self.dma_accum >= 4 && self.dma_pos < 160 {
                        self.dma_accum -= 4;
                        // 實際應從來源高位 FF46<<8 + dma_pos 讀取
                        let src = self.dma_src_base.wrapping_add(self.dma_pos);
                        let val = mem.ram.read(src);
                        // OAM 範圍：FE00 + pos
                        mem.ram.write(0xFE00u16 + self.dma_pos, val);
                        self.dma_pos += 1;
                    }
                    if self.dma_pos >= 160 {
                        self.dma_active = false;
                    }
                }
            }

            // PPU 掃描線推進與渲染
            if (self.lcdc & 0x80) != 0 {
                let mut remain = c;
                while remain > 0 {
                    let (mode, boundary) = if self.ly >= 144 {
                        (1u8, 456u32)
                    } else if self.ppu_line_cycle < 80 {
                        (2u8, 80u32)
                    } else {
                        // Mode3 has a fixed duration of 172 t-cycles (CPU cycles)
                        let mode3_len = 172u32;
                        let mode3_end = 80u32 + mode3_len; // 252
                        if self.ppu_line_cycle < mode3_end {
                            (3u8, mode3_end)
                        } else {
                            (0u8, 456u32)
                        }
                    };
                    // Mode 切換時 - 更新 ppu_mode 並在需要時觸發 STAT 中斷
                    if mode != self.ppu_mode {
                        self.ppu_mode = mode;
                        // STAT interrupt (IF bit1) is requested when a mode-change occurs
                        // and the corresponding STAT enable bit is set in STAT (stat_w):
                        // bit5 (0x20) -> Mode2 (OAM), bit4 (0x10) -> Mode1 (VBlank), bit3 (0x08) -> Mode0 (HBlank)
                        match mode {
                            2 => {
                                if (self.stat_w & 0x20) != 0 {
                                    self.ifl |= 0x02;
                                }
                            }
                            1 => {
                                if (self.stat_w & 0x10) != 0 {
                                    self.ifl |= 0x02;
                                }
                            }
                            0 => {
                                if (self.stat_w & 0x08) != 0 {
                                    self.ifl |= 0x02;
                                }
                            }
                            _ => {}
                        }
                    }
                    let step = (boundary - self.ppu_line_cycle).min(remain);
                    self.ppu_line_cycle += step;
                    remain -= step;
                    // 每條可見掃描線都強制呼叫 render_scanline
                    if self.ppu_line_cycle >= 456 {
                        if self.ly < 144 {
                            Self::render_scanline(self, mem);
                        }
                        if self.ly == 143 {
                            // 進入 VBlank 的第一條，觸發 VBlank IF bit0
                            self.ifl |= 0x01;
                        }
                        
                        // Check if Window was rendered on the CURRENT line (before LY increments)
                        // This must be evaluated before LY changes!
                        let window_was_rendered = (self.lcdc & 0x20) != 0 
                            && (self.lcdc & 0x01) != 0
                            && self.ly < 144 
                            && self.wy as i16 <= self.ly as i16
                            && (self.wx as i16 - 7) < 160;
                        
                        self.ppu_line_cycle = 0;
                        self.ly = self.ly.wrapping_add(1);
                        if self.ly > 153 {
                            // New frame
                            self.ly = 0;
                            self.win_line = 0;
                            //self.win_latched = false;
                        } else if window_was_rendered {
                            // Only increment win_line if Window was actually rendered on the previous line
                            self.win_line = self.win_line.wrapping_add(1);
                        }
                        // Check LYC coincidence
                        if self.ly == self.lyc && (self.stat_w & 0x40) != 0 {
                            self.ifl |= 0x02; // LCD STAT interrupt
                        }
                        // Debug: log when win_line increments
                        if (self.lcdc & 0x20) != 0
                            && self.ly < 144
                            && self.wy as i16 <= self.ly as i16
                        {
                            if self.dbg_window_log_count < 200 {
                                println!(
                                    "[PPU DEBUG] LY={} win_line={} (after inc) WY={} WX={} LCDC={:02X}",
                                    self.ly, self.win_line, self.wy, self.wx, self.lcdc
                                );
                                self.dbg_window_log_count += 1;
                            }
                        }
                    }
                }
            } else {
                self.ppu_mode = 0;
                self.ppu_line_cycle = 0;
                self.win_line = 0;
                self.ly = 0;
            }

            remaining -= c as u64;
        }
    }

    /// 最簡單的背景掃描線渲染（僅灰階）+ Sprites
    fn render_scanline(&mut self, mem: &crate::GB::bus_mem::BusMem) {
        let y = self.ly as usize;
        if y >= 144 {
            return;
        }
        let mut shades = [0u8; 160];
        // origin: -1 = bg, -2 = window, >=0 = OAM index
        let mut origin: [i16; 160] = [-1; 160];
        // store raw BG/Window color indices (0..3) for correct OBJ-to-BG priority checks
        let mut bg_color_idx: [u8; 160] = [0u8; 160];
        // BG enable
        if (self.lcdc & 0x01) != 0 {
            let scy = self.scy as u16;
            let scx = self.scx as u16;
            let v = ((self.ly as u16).wrapping_add(scy)) & 0xFF;
            let tilemap = if (self.lcdc & 0x08) != 0 {
                0x9C00
            } else {
                0x9800
            };
            let signed = (self.lcdc & 0x10) == 0;
            let row_in_tile = (v & 7) as u16;
            let tile_row = ((v >> 3) & 31) as u16;
            for x in 0..160u16 {
                let h = (x.wrapping_add(scx)) & 0xFF;
                let tile_col = ((h >> 3) & 31) as u16;
                let map_index = tile_row * 32 + tile_col;
                let tile_id = mem.ram.read(tilemap + map_index);
                let tile_addr = Self::calc_tile_addr(tile_id, signed, row_in_tile);
                let lo = mem.ram.read(tile_addr);
                let hi = mem.ram.read(tile_addr + 1);
                let bit = 7 - ((h & 7) as u8);
                let color = Self::get_pixel_color(lo, hi, bit);
                let shade = (self.bgp >> (color * 2)) & 0x03;
                shades[x as usize] = shade;
                bg_color_idx[x as usize] = color;
                origin[x as usize] = -1;
            }
        } else {
            for x in 0..160usize {
                shades[x] = 0;
            }
        }

        // Window rendering
        if (self.lcdc & 0x20) != 0 && (self.lcdc & 0x01) != 0 {
            let wy = self.wy as i16;
            let wx = self.wx as i16 - 7; // WX has 7 pixel offset
            if wy <= self.ly as i16 && wx < 160 {
                // Window tilemap and tile data selection read from current LCDC.
                // LCDC bit 6: 0 -> 0x9800, 1 -> 0x9C00
                // LCDC bit 4: 0 -> signed ($8800-97FF), 1 -> unsigned ($8000-8FFF)
                let tilemap = if (self.lcdc & 0x40) != 0 { 0x9C00 } else { 0x9800 };
                let signed = (self.lcdc & 0x10) == 0;
                self.win_tilemap_base = tilemap;
                self.win_signed = signed;

                if self.debug_enabled && self.dbg_window_log_count < 1000 {
                    println!(
                        "[PPU DEBUG] WINDOW RENDER: LY={} win_line={} WX={} WY={} LCDC={:02X} TILEMAP=0x{:04X} SIGNED={}",
                        self.ly, self.win_line, self.wx, self.wy, self.lcdc, tilemap, signed
                    );
                    self.dbg_window_log_count += 1;
                }

                let row_in_tile = (self.win_line & 7) as u16;
                let tile_row = ((self.win_line >> 3) & 31) as u16;

                for x in 0..160u16 {
                    let sx = x as i16;
                    if sx < wx {
                        continue;
                    }
                    let window_x = (sx - wx) as u16;
                    let tile_col = ((window_x >> 3) & 31) as u16;
                    let map_index = tile_row * 32 + tile_col;
                    let tile_id = mem.ram.read(tilemap + map_index);
                    let tile_addr = Self::calc_tile_addr(tile_id, signed, row_in_tile);
                    let lo = mem.ram.read(tile_addr);
                    let hi = mem.ram.read(tile_addr + 1);
                    let bit = 7 - ((window_x & 7) as u8);
                    let color = Self::get_pixel_color(lo, hi, bit);
                    let shade = (self.bgp >> (color * 2)) & 0x03;
                    shades[x as usize] = shade;
                    bg_color_idx[x as usize] = color;
                    origin[x as usize] = -2;
                }
            }
        }

        // Sprites rendering (OBJ)
        if (self.lcdc & 0x02) != 0 {
            let sprite_height = if (self.lcdc & 0x04) != 0 { 16 } else { 8 };
            let mut sprites_on_line = Vec::new();

            // First pass: collect all sprites on this scanline (up to 10)
            for oam_idx in (0..160).step_by(4) {
                if sprites_on_line.len() >= 10 {
                    break; // Max 10 sprites per line
                }

                let _sprite_y = mem.ram.read((0xFE00u16).wrapping_add(oam_idx as u16)) as i16 - 16;

                // Check if sprite is on this scanline
                if _sprite_y <= self.ly as i16
                    && (self.ly as i16) < (_sprite_y + sprite_height as i16)
                {
                    sprites_on_line.push(oam_idx);
                }
            }

            // Debug: dump selected sprites for problematic LY ranges
            if self.debug_enabled
                && self.dbg_window_log_count < 200
                && (44..=80).contains(&(self.ly as i32))
            {
                // Dump sprites on this line with full OAM attributes to aid debugging
                print!("[SPR DEBUG] LY={} sprites_on_line:", self.ly);
                for &si in sprites_on_line.iter() {
                    let sy = mem.ram.read((0xFE00u16).wrapping_add(si as u16)) as i16 - 16;
                    let sx = mem.ram.read((0xFE00u16).wrapping_add(si as u16).wrapping_add(1)) as i16 - 8;
                    let tile_idx = mem.ram.read((0xFE00u16).wrapping_add(si as u16).wrapping_add(2));
                    let flags = mem.ram.read((0xFE00u16).wrapping_add(si as u16).wrapping_add(3));
                    // Print OAM index, position, tile and flags (hex) for easier comparison
                    print!(
                        " [oam={} x={} y={} tile=0x{:02X} flags=0x{:02X}]",
                        si / 4,
                        sx,
                        sy,
                        tile_idx,
                        flags
                    );
                }
                println!();
                self.dbg_window_log_count += 1;
            }

            // Second pass: render selected sprites using DMG OBJ-to-OBJ priority rules:
            // - Smaller X coordinate has higher priority (wins conflicts)
            // - If X is equal, smaller OAM index has higher priority
            // We sort with higher priority first, then render front-to-back.
            // Once a pixel is drawn, lower priority sprites cannot overwrite it.
            let mut sprites_sorted = sprites_on_line;
            sprites_sorted.sort_by(|&a, &b| {
                let ax = mem.ram.read((0xFE00u16).wrapping_add(a as u16).wrapping_add(1)) as i16 - 8;
                let bx = mem.ram.read((0xFE00u16).wrapping_add(b as u16).wrapping_add(1)) as i16 - 8;
                // Smaller X first (higher priority), then smaller OAM index first
                ax.cmp(&bx).then_with(|| a.cmp(&b))
            });

            if std::env::var("GB_DEBUG_PPU").is_ok() && !sprites_sorted.is_empty() {
                println!(
                    "[SPRITE DEBUG] LY={} sprites: {:?}",
                    self.ly,
                    sprites_sorted
                        .iter()
                        .map(|&idx| {
                            let x = mem.ram.read((0xFE00u16).wrapping_add(idx as u16).wrapping_add(1)) as i16 - 8;
                            (idx, x)
                        })
                        .collect::<Vec<_>>()
                );
            }
            // Render sprites front-to-back (high to low priority)
            for &oam_idx in sprites_sorted.iter() {
                let (sprite_y, sprite_x, tile_idx, flags) = Self::read_sprite_oam(mem, oam_idx);
                let mut pixels_rendered = 0;

                // 計算 sprite tile row (處理 Y flip)
                let mut sprite_tile_row = (self.ly as i16 - sprite_y) as u16;
                if (flags & 0x40) != 0 {
                    sprite_tile_row = (sprite_height - 1) as u16 - sprite_tile_row;
                }

                // 處理 8x16 sprites
                let mut actual_tile_idx = tile_idx;
                if sprite_height == 16 {
                    actual_tile_idx = tile_idx & 0xFE;
                    if sprite_tile_row >= 8 {
                        actual_tile_idx |= 0x01;
                        sprite_tile_row -= 8;
                    }
                }

                // 讀取 tile 資料
                let tile_addr = 0x8000u16 + (actual_tile_idx as u16) * 16 + sprite_tile_row * 2;
                let lo = mem.ram.read(tile_addr);
                let hi = mem.ram.read(tile_addr + 1);

                // 渲染 sprite 像素
                for px in 0..8 {
                    let bit = if (flags & 0x20) != 0 { px } else { 7 - px };
                    let color = Self::get_pixel_color(lo, hi, bit);

                    if color == 0 {
                        continue;
                    }

                    let screen_x = sprite_x + px as i16;
                    if screen_x < 0 || screen_x >= 160 {
                        continue;
                    }

                    let idx = screen_x as usize;
                    
                    // 檢查優先級（sprite-to-sprite 和 BG priority）
                    if !Self::should_draw_sprite(&origin, &bg_color_idx, idx, flags) {
                        // Debug: 記錄被 bg_priority 隱藏的 sprite
                        if self.debug_enabled
                            && self.dbg_window_log_count < 200
                            && (44..=80).contains(&(self.ly as i32))
                        {                            let bg_color = bg_color_idx[idx];                                let bg_source = match origin[idx] {
                                    -2 => "WINDOW",
                                    -1 => "BG",
                                    _ => "SPR",
                                };

                                // Gather extra info: tilemap & tile id & raw color at this pixel
                                let mut extra = String::new();
                                // We'll capture the raw tile row bytes for BG/Window here
                                let mut bg_lo: u8 = 0;
                                let mut bg_hi: u8 = 0;
                                if origin[idx] == -1 {
                                    // Background
                                    let scy = self.scy as u16;
                                    let scx = self.scx as u16;
                                    let v = ((self.ly as u16).wrapping_add(scy)) & 0xFF;
                                    let h = ((idx as u16).wrapping_add(scx)) & 0xFF;
                                    let tilemap = if (self.lcdc & 0x08) != 0 {
                                        0x9C00
                                    } else {
                                        0x9800
                                    };
                                    let signed = (self.lcdc & 0x10) == 0;
                                    let row_in_tile = (v & 7) as u16;
                                    let tile_row = ((v >> 3) & 31) as u16;
                                    let tile_col = ((h >> 3) & 31) as u16;
                                    let map_index = tile_row * 32 + tile_col;
                                    let tile_id = mem.ram.read(tilemap + map_index);
                                    let tile_addr = Self::calc_tile_addr(tile_id, signed, row_in_tile);
                                    bg_lo = mem.ram.read(tile_addr);
                                    bg_hi = mem.ram.read(tile_addr + 1);
                                    let bit2 = 7 - ((h & 7) as u8);
                                    let color_here = Self::get_pixel_color(bg_lo, bg_hi, bit2);
                                    let _ = write!(
                                        extra,
                                        " tilemap=0x{:04X} tile=0x{:02X} color={} signed={}",
                                        tilemap, tile_id, color_here, signed
                                    );
                                } else if origin[idx] == -2 {
                                    // Window
                                    let wx = self.wx as i16 - 7;
                                    let sx = idx as i16;
                                    let window_x = (sx - wx) as u16;
                                    let tilemap = if (self.lcdc & 0x40) != 0 {
                                        0x9C00
                                    } else {
                                        0x9800
                                    };
                                    let signed = (self.lcdc & 0x10) == 0;
                                    let row_in_tile = (self.win_line & 7) as u16;
                                    let tile_row = ((self.win_line >> 3) & 31) as u16;
                                    let tile_col = ((window_x >> 3) & 31) as u16;
                                    let map_index = tile_row * 32 + tile_col;
                                    let tile_id = mem.ram.read(tilemap + map_index);
                                    let tile_addr = Self::calc_tile_addr(tile_id, signed, row_in_tile);
                                    bg_lo = mem.ram.read(tile_addr);
                                    bg_hi = mem.ram.read(tile_addr + 1);
                                    let bit2 = 7 - ((window_x & 7) as u8);
                                    let color_here = Self::get_pixel_color(bg_lo, bg_hi, bit2);
                                    let _ = write!(extra, " tilemap=0x{:04X} tile=0x{:02X} color={} signed={}", tilemap, tile_id, color_here, signed);
                                }

                                // Append sprite-specific info: tile, raw sprite color, flags and flips
                                let spr_prio = (flags & 0x80) != 0;
                                let spr_xflip = (flags & 0x20) != 0;
                                let spr_yflip = (flags & 0x40) != 0;
                                let spr_pal = if (flags & 0x10) != 0 { 1 } else { 0 };
                                let _ = write!(
                                    extra,
                                    " spr_tile=0x{:02X} spr_color={} flags=0x{:02X} prio={} xflip={} yflip={} pal={} sprite_x={}",
                                    tile_idx,
                                    color,
                                    flags,
                                    spr_prio as u8,
                                    spr_xflip as u8,
                                    spr_yflip as u8,
                                    spr_pal,
                                    sprite_x
                                );

                                // Append raw tile row bytes for background/window (lo2/hi2)
                                // and for the sprite (lo/hi) to aid debugging.
                                let _ = write!(
                                    extra,
                                    " bg_tile_bytes=0x{:02X} 0x{:02X} spr_tile_bytes=0x{:02X} 0x{:02X}",
                                    bg_lo,
                                    bg_hi,
                                    lo,
                                    hi
                                );

                                println!(
                                    "[SPR DEBUG] LY={} skip px={} oam={} (bg_priority bg_source={} bg_color={}){}",
                                    self.ly,
                                    idx,
                                    oam_idx / 4,
                                    bg_source,
                                    bg_color,
                                extra
                            );
                            self.dbg_window_log_count += 1;
                        }
                        continue;
                    }

                    // 選擇調色板並繪製像素
                    let palette = if (flags & 0x10) != 0 {
                        self.obp1
                    } else {
                        self.obp0
                    };
                    let shade = (palette >> (color * 2)) & 0x03;
                    shades[idx] = shade;
                    origin[idx] = (oam_idx / 4) as i16;
                    pixels_rendered += 1;
                }

                // Debug: report tile data for all sprites
                if self.debug_enabled && self.dbg_window_log_count < 200 && self.ly == 66 {
                    let tile_base = 0x8000 + (actual_tile_idx as u16) * 16;
                    let mut tile_data = [0u8; 16];
                    for i in 0..16 {
                        tile_data[i] = mem.ram.read(tile_base + i as u16);
                    }
                    println!(
                        "[SPR TILE FULL] LY={} oam={} x={} y={} tile=0x{:02X} actual_tile=0x{:02X} tile_addr=0x{:04X} flags=0x{:02X} data={:02X?}",
                        self.ly,
                        oam_idx / 4,
                        sprite_x,
                        sprite_y,
                        tile_idx,
                        actual_tile_idx,
                        tile_addr,
                        flags,
                        tile_data
                    );
                    self.dbg_window_log_count += 1;
                }

                // Debug: report pixels rendered by this sprite
                if self.debug_enabled
                    && self.dbg_window_log_count < 200
                    && (44..=80).contains(&(self.ly as i32))
                {
                    println!(
                        "[SPR RENDER] LY={} oam={} x={} tile=0x{:02X} pixels_rendered={}",
                        self.ly,
                        oam_idx / 4,
                        sprite_x,
                        tile_idx,
                        pixels_rendered
                    );
                    self.dbg_window_log_count += 1;
                }
            }
        }

        let base = y * 160;
        for x in 0..160usize {
            self.framebuffer[base + x] = shades[x];
        }
        // Pixel-level debug output for focused LY ranges
        if self.debug_enabled && ((0..=16).contains(&y) || (40..=80).contains(&y) || y == 66) {
            if self.debug_enabled && self.dbg_pixel_log_count < 200 {
                // Print a compact per-pixel summary: X:SRC:shade
                // SRC = B (bg), W (window), S# (sprite index)
                let mut line = String::new();
                for x in 0..160usize {
                    let src = match origin[x] {
                        -2 => "W".to_string(),
                        -1 => "B".to_string(),
                        n => format!("S{}", n),
                    };
                    let _ = write!(line, "{}:{} ", src, shades[x]);
                }
                println!("[PIX] LY={} {}", y, line);
                self.dbg_pixel_log_count += 1;
            }
        }
    }
    pub fn is_dma_active(&self) -> bool {
        self.dma_active && self.dma_pos < 160 && self.dma_start_delay == 0
    }
    pub fn get_ie_raw(&self) -> u8 {
        self.ie
    }
    pub fn get_if_raw(&self) -> u8 {
        self.ifl | 0xE0
    }
    pub fn set_if_raw(&mut self, v: u8) {
        self.ifl = v & 0x1F;
    }
    pub fn set_joypad_rows(&mut self, dpad: u8, btns: u8) {
        self.update_joypad(dpad & 0x0F, btns & 0x0F);
    }
    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }

    // ===== PPU 輔助函數 =====

    /// 計算 tile 資料的記憶體位址
    #[inline]
    fn calc_tile_addr(tile_id: u8, signed: bool, row_in_tile: u16) -> u16 {
        let tile_base: u16 = if signed {
            let idx = tile_id as i8 as i32;
            let offset = idx * 16;
            (0x9000i32.wrapping_add(offset)) as u16
        } else {
            0x8000u16.wrapping_add((tile_id as u16) * 16)
        };
        tile_base.wrapping_add(row_in_tile * 2)
    }

    /// 從 tile 資料中提取像素顏色索引 (0-3)
    #[inline]
    fn get_pixel_color(lo: u8, hi: u8, bit: u8) -> u8 {
        let lo_b = (lo >> bit) & 1;
        let hi_b = (hi >> bit) & 1;
        (hi_b << 1) | lo_b
    }

    /// 讀取 sprite OAM 屬性
    #[inline]
    fn read_sprite_oam(mem: &crate::GB::bus_mem::BusMem, oam_idx: u16) -> (i16, i16, u8, u8) {
        let base = 0xFE00u16 + oam_idx;
        let sprite_y = mem.ram.read(base) as i16 - 16;
        let sprite_x = mem.ram.read(base + 1) as i16 - 8;
        let tile_idx = mem.ram.read(base + 2);
        let flags = mem.ram.read(base + 3);
        (sprite_y, sprite_x, tile_idx, flags)
    }

    /// 檢查 sprite 像素是否應該繪製（考慮優先級）
    #[inline]
    fn should_draw_sprite(
        origin: &[i16; 160],
        bg_color_idx: &[u8; 160],
        idx: usize,
        flags: u8,
    ) -> bool {
        // Sprite-to-sprite 衝突：已有更高優先級 sprite
        if origin[idx] >= 0 {
            return false;
        }
        // BG 優先級：sprite 在 BG color 1-3 後面
        let bg_priority = (flags & 0x80) != 0;
        let bg_color = bg_color_idx[idx];
        !(bg_priority && bg_color != 0)
    }
}
