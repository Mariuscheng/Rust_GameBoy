// Bus 記憶體相關欄位與邏輯
use crate::GB::mbc::{MBCImpl, MBC};
use crate::GB::types::*;
use crate::GB::RAM::RAM;

pub struct BusMem {
    pub ram: RAM,
    pub rom: Vec<u8>,
    pub rom_banks: usize,
    pub mbc: MBCImpl,
    pub ext_ram: Vec<u8>,
}

impl BusMem {
    pub fn new() -> Self {
        Self {
            ram: RAM::new(),
            rom: Vec::new(),
            rom_banks: 0,
            mbc: MBCImpl::new(MbcType::None),
            ext_ram: Vec::new(),
        }
    }
    // TODO: 移植 load_rom, read_rom, write_mbc, mbc1_calc_bank0, mbc1_calc_bankX 等方法
}
impl BusMem {
    // 這裡可放原本 BUS.rs 內所有 RAM/ROM/MBC 相關方法
    pub fn load_rom(&mut self, data: Vec<u8>) {
        self.rom = data;
        self.rom_banks = (self.rom.len() + 0x3FFF) / 0x4000;
        let cart_type = if self.rom.len() > 0x0147 {
            self.rom[0x0147]
        } else {
            0x00
        };
        let mbc_type = match cart_type {
            0x01 | 0x02 | 0x03 => MbcType::Mbc1,
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => MbcType::Mbc3,
            0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1E => MbcType::Mbc5,
            _ => MbcType::None,
        };
        self.mbc = MBCImpl::new(mbc_type);
        let ram_size_code = if self.rom.len() > 0x0149 {
            self.rom[0x0149]
        } else {
            0
        };
        let ram_banks = match ram_size_code {
            0x02 => 1,
            0x03 => 4,
            0x04 => 16,
            0x05 => 8,
            _ => 0,
        };
        self.ext_ram = vec![0u8; ram_banks * 0x2000];
    }

    fn read_rom(&self, addr: u16) -> u8 {
        self.mbc.read_rom(addr, &self.rom, self.rom_banks)
    }
    fn write_mbc(&mut self, addr: u16, val: u8) {
        self.mbc.write(addr, val);
    }

    pub fn read(
        &self,
        addr: u16,
        io: &crate::GB::bus_io::BusIO,
        _apu: &crate::GB::bus_apu::BusAPU,
    ) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                if self.rom_banks > 0 {
                    self.read_rom(addr)
                } else {
                    self.ram.read(addr)
                }
            }
            // IO mapped registers
            0xFF00 => {
                let sel = io.p1_sel & 0x30;
                let mut v = 0xC0 | sel;
                if (sel & 0x10) == 0 {
                    v = (v & 0xF0) | (io.joyp_dpad & 0x0F);
                } else if (sel & 0x20) == 0 {
                    v = (v & 0xF0) | (io.joyp_btns & 0x0F);
                } else {
                    v |= 0x0F;
                }
                v
            }
            0xFF04 => io.div,
            0xFF01 => io.sb,
            0xFF02 => io.sc | 0x7C, // 只留 bit0/bit7，其餘讀為 1
            0xFF05 => io.tima,
            0xFF06 => io.tma,
            0xFF07 => io.tac & 0x07,
            0xFF0F => io.get_if_raw(),
            0xFF40 => io.lcdc,
            0xFF41 => {
                let mut stat = io.stat_w & 0x78;
                if io.ly == io.lyc {
                    stat |= 0x04;
                }
                stat |= io.ppu_mode & 0x03;
                stat | 0x80
            }
            0xFF42 => io.scy,
            0xFF43 => io.scx,
            0xFF44 => io.ly,
            0xFF45 => io.lyc,
            0xFF47 => io.bgp,
            0xFF48 => io.obp0,
            0xFF49 => io.obp1,
            0xFF4A => io.wy,
            0xFF4B => io.wx,
            0xFFFF => io.ie,
            // APU regs mirror (minimal): 0xFF10..=0xFF3F
            0xFF10..=0xFF25 | 0xFF27..=0xFF2F | 0xFF30..=0xFF3F => _apu.read_apu_reg(addr),
            0xFF26 => {
                // NR52: bit7 power, bits 0-3 channel on, bits 4-6 read 1 (DMG: bits 4-6=1, bits 0-3=0)
                let stored = self.ram.read(0xFF26);
                0xF0 | (stored & 0x80)
            }
            0xA000..=0xBFFF => {
                if self.ext_ram.is_empty() || !self.mbc.ram_enabled() {
                    0xFF
                } else {
                    let bank = self.mbc.ram_bank() as usize;
                    let base = bank * 0x2000;
                    let off = (addr as usize - 0xA000) & 0x1FFF;
                    self.ext_ram.get(base + off).copied().unwrap_or(0xFF)
                }
            }
            _ => self.ram.read(addr),
        }
    }

    pub fn write(
        &mut self,
        addr: u16,
        val: u8,
        io: &mut crate::GB::bus_io::BusIO,
        _apu: &mut crate::GB::bus_apu::BusAPU,
    ) {
        match addr {
            0x0000..=0x7FFF => {
                if self.rom_banks > 0 {
                    self.write_mbc(addr, val);
                } else {
                    self.ram.write(addr, val);
                }
            }
            0x8000..=0x9FFF => {
                self.ram.write(addr, val);
            }
            // IO mapped registers
            0xFF00 => {
                io.p1_sel = val & 0x30;
            }
            0xFF04 => {
                // Writing DIV resets the internal divider. This can cause a
                // falling edge on the timer input bit which should increment
                // TIMA if the timer is enabled. Detect that edge based on the
                // previous `div_total` value and the TAC frequency selection.
                let prev_div_total = io.div_total;
                // Determine selected bit position for current TAC frequency
                let bitpos = match io.tac & 0x03 {
                    0 => 10u32, // period 1024 -> bit 10
                    1 => 4u32,  // period 16 -> bit 4
                    2 => 6u32,  // period 64 -> bit 6
                    3 => 8u32,  // period 256 -> bit 8
                    _ => 10u32,
                };
                let prev_bit = ((prev_div_total >> bitpos) & 0x01) != 0;
                // reset divider counters
                io.div = 0;
                io.div_total = 0;
                io.div_counter = 0;
                io.div_sub = 0;
                // If timer enabled and a falling edge occurred (1->0), increment TIMA
                if (io.tac & 0x04) != 0 && prev_bit {
                    // follow TIMA increment/overflow semantics
                    if io.tima_reload_delay == 0 {
                        if io.tima == 0xFF {
                            io.tima = 0;
                            io.tima_reload_delay = 4;
                        } else {
                            io.tima = io.tima.wrapping_add(1);
                        }
                    }
                }
            }
            0xFF05 => {
                // If a TIMA write occurs during a pending reload delay, the write
                // overrides the scheduled reload and cancels the delay.
                io.tima = val;
                if io.tima_reload_delay > 0 {
                    io.tima_reload_delay = 0;
                }
            }
            0xFF06 => {
                // Writing TMA during a pending reload updates the value that
                // will be loaded when the delay expires.
                io.tma = val;
            }
            0xFF07 => {
                // Update TAC. Changes to timer input do not cancel a pending
                // TIMA reload; just update the control register.
                let prev = io.tac;
                io.tac = val & 0x07;
                // Optionally, if timer is disabled/enable transition occurs,
                // reset internal accumulators to avoid miscounting.
                if (prev & 0x04) == 0 && (io.tac & 0x04) != 0 {
                    // timer enabled now: clear accumulators for alignment
                    io.timer_accum = 0;
                } else if (prev & 0x04) != 0 && (io.tac & 0x04) == 0 {
                    // timer disabled: clear accumulator
                    io.timer_accum = 0;
                }
            }
            0xFF40 => {
                let prev = io.lcdc;
                io.lcdc = val;
                // Important: Window internal line counter (win_line) should NOT be reset
                // when LCDC bits change mid-frame. According to DMG behavior, win_line
                // persists through LCDC changes and only resets at frame start (LY > 153).
                // The tilemap/signed mode are read fresh from LCDC during each scanline render.
                if io.dbg_window_log_count < 200 {
                    println!(
                        "[BUS WRITE] WRITE LCDC=0x{:02X} prev=0x{:02X} LY={} WY={} WX={}",
                        val, prev, io.ly, io.wy, io.wx
                    );
                    io.dbg_window_log_count += 1;
                }
            }
            0xFF41 => {
                io.stat_w = val & 0x78;
            }
            0xFF42 => {
                io.scy = val;
            }
            0xFF43 => {
                io.scx = val;
            }
            0xFF44 => {
                io.ly = 0;
                io.ppu_line_cycle = 0;
                // Reset window line counter when LY is reset
                io.win_line = 0;
                // Reset window latch so the tilemap/signed-mode are re-latched on
                // the next window pixel after LY reset. This avoids persisting
                // an out-of-date latched window configuration across frames.
            }
            0xFF45 => {
                io.lyc = val;
            }
            0xFF46 => {
                io.dma_active = true;
                io.dma_pos = 0;
                io.dma_start_delay = 0;
                io.dma_accum = 0;
                io.dma_src_base = (val as u16) << 8;
                self.ram.write(0xFF46, val);
            }
            0xFF47 => {
                io.bgp = val;
            }
            0xFF48 => {
                io.obp0 = val;
            }
            0xFF49 => {
                io.obp1 = val;
            }
            0xFF4A => {
                let prev = io.wy;
                io.wy = val;
                // Note: Writing WY does NOT reset win_line according to DMG behavior.
                // win_line is the Window's internal row counter and persists through
                // WY changes, only resetting at frame start (LY > 153).
                // WY controls when the Window starts being visible (Y position comparison).

                if io.dbg_window_log_count < 200 {
                    println!(
                        "[BUS WRITE] WRITE WY=0x{:02X} prev=0x{:02X} LY={}",
                        val, prev, io.ly
                    );
                    io.dbg_window_log_count += 1;
                }
            }
            0xFF4B => {
                let prev = io.wx;
                io.wx = val;
                // Note: Writing WX does NOT reset win_line according to DMG behavior.
                // win_line is the Window's internal row counter and persists through
                // WX changes, only resetting at frame start (LY > 153).
                // WX controls the horizontal position where Window pixels start appearing.

                if io.dbg_window_log_count < 200 {
                    println!(
                        "[BUS WRITE] WRITE WX=0x{:02X} prev=0x{:02X} LY={}",
                        val, prev, io.ly
                    );
                    io.dbg_window_log_count += 1;
                }
            }
            // Serial
            0xFF01 => {
                io.sb = val;
            }
            0xFF02 => {
                io.sc = val & 0x81; // bit7=start, bit0=clock select
                                    // Always print SB for debugging
                let ch = io.sb as char;
                let pc = crate::GB::debug::load_pc();
                let regs = crate::GB::debug::load_state();
                println!(
                    "[SERIAL DEBUG] PC=0x{:04X} '{}' (0x{:02X}) SC=0x{:02X} A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} F={:02X} H={:02X} L={:02X} SP={:04X}",
                    pc,
                    ch,
                    io.sb,
                    io.sc,
                    regs.a,
                    regs.b,
                    regs.c,
                    regs.d,
                    regs.e,
                    regs.f,
                    regs.h,
                    regs.l,
                    regs.sp
                );
                if (io.sc & 0x80) != 0 {
                    // 當寫入 FF02 且 bit7=1，將 FF01 (SB) 的內容印出
                    let ch = io.sb as char;
                    let pc = crate::GB::debug::load_pc();
                    // Also load register snapshot if available
                    let regs = crate::GB::debug::load_state();
                    // Print a readable representation with PC context, hex code, and registers
                    println!(
                        "[SERIAL OUT] PC=0x{:04X} '{}' (0x{:02X}) A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} F={:02X} H={:02X} L={:02X} SP={:04X} OP=0x{:02X}",
                        pc,
                        ch,
                        io.sb,
                        regs.a,
                        regs.b,
                        regs.c,
                        regs.d,
                        regs.e,
                        regs.f,
                        regs.h,
                        regs.l,
                        regs.sp,
                        regs.last_opcode
                    );
                    // Dump last few executed opcodes for diagnostic context
                    let last_ops = crate::GB::debug::get_last_ops();
                    if !last_ops.is_empty() {
                        print!("    [LAST OPS]: ");
                        for (pc, op) in last_ops.into_iter().rev().take(10) {
                            print!("0x{:04X}=0x{:02X} ", pc, op);
                        }
                        println!();
                    }
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    io.ifl |= 0x08; // serial interrupt
                    io.sc &= !0x80; // clear start (transfer complete)
                }
            }
            // IF (interrupt flags)
            0xFF0F => {
                self.ram.write(addr, val);
                io.ifl = val;
            }
            // APU regs
            0xFF10..=0xFF25 | 0xFF27..=0xFF2F | 0xFF30..=0xFF3F => {
                _apu.write_apu_reg(addr, val);
                self.ram.write(addr, val);
            }
            0xFF26 => {
                // NR52 power control: store to RAM mirror; DMG: bits 4-6=1, bits 0-3=0, bit7=power
                self.ram.write(addr, 0xF0 | (val & 0x80));
                if (val & 0x80) == 0 {
                    // power off clears other APU regs (simplified)
                    for a in 0xFF10u16..=0xFF3Fu16 {
                        if a != 0xFF26 {
                            self.ram.write(a, 0);
                        }
                    }
                }
            }
            0xFFFF => {
                io.ie = val;
                #[cfg(feature = "dbg_int")]
                {
                    println!(
                        "[DBG INT] WRITE IE=0x{:02X} (IF=0x{:02X}) PC=0x{:04X}",
                        val,
                        self.ram.read(0xFF0F),
                        0u16
                    );
                }
            }
            0xA000..=0xBFFF => {
                if self.ext_ram.is_empty() || !self.mbc.ram_enabled() {
                    // ignore
                } else {
                    let bank = self.mbc.ram_bank() as usize;
                    let base = bank * 0x2000;
                    let off = (addr as usize - 0xA000) & 0x1FFF;
                    if base + off < self.ext_ram.len() {
                        self.ext_ram[base + off] = val;
                    }
                }
            }
            _ => self.ram.write(addr, val),
        }
    }
}
