impl MMU {
    /// 寫入 16 位元 (u16) 到指定位址（小端序）
    pub fn write_word(&mut self, addr: u16, value: u16) {
        let lo = (value & 0xFF) as u8;
        let hi = (value >> 8) as u8;
        self.write_byte(addr, lo);
        self.write_byte(addr.wrapping_add(1), hi);
    }

    /// 從指定位址讀取 16 位元 (u16)（小端序）
    pub fn read_word(&self, addr: u16) -> u16 {
        let lo = self.read_byte(addr).unwrap_or(0) as u16;
        let hi = self.read_byte(addr.wrapping_add(1)).unwrap_or(0) as u16;
        (hi << 8) | lo
    }
}
use crate::core::utils::logger::log_to_file;
// Game Boy MMU struct（合併重複欄位，統一映射）
pub struct MMU {
    pub eram: [u8; 0x2000],                 // 0xA000-0xBFFF 外部 RAM (8KB)
    pub memory: [u8; 0x10000], // 64KB GameBoy 記憶體（主映射，部分區段可用欄位/物件代理）
    pub vram: crate::core::ppu::vram::VRAM, // 8KB VRAM
    pub oam: [u8; 0xA0],       // 160 bytes OAM (Sprite Attribute Table)
    pub mbc: Option<Box<dyn crate::core::mmu::mbc::MBCController>>, // MBC 控制器
    pub rom: Vec<u8>,          // 原始 ROM buffer
    pub io: [u8; 0x80],        // 0xFF00-0xFF7F I/O Registers
    pub hram: [u8; 0x7F],      // 0xFF80-0xFFFE High RAM
    pub ie: u8,                // 0xFFFF Interrupt Enable Register
    pub lcd_registers: crate::core::mmu::lcd_registers::LCDRegisters, // LCD/PPU Registers
    pub timer_cycles: u32,     // 計時器累積 cycles
}

impl Default for MMU {
    fn default() -> Self {
        MMU {
            eram: [0; 0x2000],
            memory: [0; 0x10000],
            vram: crate::core::ppu::vram::VRAM::new_with_buffer(None),
            oam: [0; 0xA0],
            mbc: None,
            rom: vec![],
            io: [0; 0x80],
            hram: [0; 0x7F],
            ie: 0,
            lcd_registers: crate::core::mmu::lcd_registers::LCDRegisters::new(),
            timer_cycles: 0,
        }
    }
}

impl MMU {
    /// 每執行一次 CPU/PPU tick 時呼叫，更新 DIV/TIMA/TMA/TAC
    pub fn timer_tick(&mut self, cycles: u32) {
        // DIV (0xFF04) 每 256 cycles 增加 1
        self.timer_cycles += cycles;
        if self.timer_cycles >= 256 {
            self.io[0x04] = self.io[0x04].wrapping_add(1);
            self.timer_cycles -= 256;
        }
        // Timer enable (TAC bit 2)
        let tac = self.io[0x07];
        if tac & 0x04 != 0 {
            // Timer frequency
            let freq = match tac & 0x03 {
                0 => 1024, // 4096Hz
                1 => 16,   // 262144Hz
                2 => 64,   // 65536Hz
                3 => 256,  // 16384Hz
                _ => 1024,
            };
            // TIMA (0xFF05) 每 freq cycles 增加 1
            if self.timer_cycles % freq == 0 {
                let old = self.io[0x05];
                self.io[0x05] = self.io[0x05].wrapping_add(1);
                if old == 0xFF {
                    // 溢位，TIMA = TMA，觸發中斷
                    self.io[0x05] = self.io[0x06];
                    self.io[0x0F] |= 0x04; // IF 計時器中斷
                }
            }
        }
    }
    /// 由 ROM buffer 建立 MMU 實例
    pub fn from_buffer(buffer: &[u8]) -> Self {
        // VRAM 初始化：自動複製 ROM buffer 的 0x8000~0x9FFF 區段（tile data/tile map）
        let vram = if buffer.len() >= 0xA000 {
            crate::core::ppu::vram::VRAM::new_with_buffer(Some(&buffer[0x8000..0xA000]))
        } else {
            crate::core::ppu::vram::VRAM::new_with_buffer(None)
        };
        let mbc_type = if buffer.len() > 0x147 {
            buffer[0x147]
        } else {
            0x00
        };
        let mbc = crate::core::mmu::mbc::create_mbc(mbc_type);
        let rom = buffer.to_vec();
        let mut mmu = MMU {
            eram: [0; 0x2000],
            memory: [0; 0x10000],
            vram,
            oam: [0; 0xA0],
            mbc,
            rom,
            io: [0; 0x80],
            hram: [0; 0x7F],
            ie: 0,
            lcd_registers: crate::core::mmu::lcd_registers::LCDRegisters::new(),
            timer_cycles: 0,
        };
        let len = buffer.len().min(0x8000);
        mmu.memory[0..len].copy_from_slice(&buffer[0..len]);
        for b in &mut mmu.memory[0xC000..=0xDFFF] {
            *b = 0xFF;
        }
        for i in 0xE000..=0xFDFF {
            mmu.memory[i] = mmu.memory[i - 0x2000];
        }
        mmu
    }

    /// 取得 ROM 區段 (0x0000~0x7FFF)
    pub fn get_rom_buffer(&self) -> Vec<u8> {
        self.memory[0x0000..0x8000].to_vec()
    }
    pub fn new() -> Self {
        MMU {
            eram: [0; 0x2000],
            memory: [0; 0x10000],
            vram: crate::core::ppu::vram::VRAM::new_with_buffer(None),
            oam: [0; 0xA0],
            mbc: None,
            rom: vec![],
            io: [0; 0x80],
            hram: [0; 0x7F],
            ie: 0,
            lcd_registers: crate::core::mmu::lcd_registers::LCDRegisters::new(),
            timer_cycles: 0,
        }
    }
    /// 寫入 GameBoy 記憶體
    pub fn write_byte(&mut self, addr: u16, value: u8) -> crate::core::error::Result<()> {
        // 強制 log 所有寫入
        log_to_file(&format!(
            "[MMU_WRITE] addr={:04X} value={:02X}",
            addr, value
        ));

        // MBC 控制區段
        if (0x0000..=0x7FFF).contains(&addr) {
            if let Some(ref mut mbc) = self.mbc {
                log_to_file(&format!(
                    "[MMU][BANK_SWITCH] write addr={:04X} value={:02X}",
                    addr, value
                ));
                mbc.write(addr, value);
                return Ok(());
            }
        }
        if (0xA000..=0xBFFF).contains(&addr) {
            if let Some(ref mut mbc) = self.mbc {
                mbc.write(addr, value);
                return Ok(());
            } else {
                // 無 MBC 時直接寫入內部 ERAM
                self.eram[(addr - 0xA000) as usize] = value;
                return Ok(());
            }
        }
        // DMA (OAM DMA) 傳輸觸發
        if addr == 0xFF46 {
            let src_base = (value as u16) << 8;
            for i in 0..0xA0 {
                self.oam[i] = self.read_byte(src_base + i as u16).unwrap_or(0xFF);
            }
            self.lcd_registers.dma = value;
            self.io[0x46] = value;
            return Ok(());
        }
        match addr {
            0x8000..=0x9FFF => self.vram.write_byte((addr - 0x8000) as usize, value),
            0xFE00..=0xFE9F => self.oam[(addr - 0xFE00) as usize] = value,
            0xFF00 => {
                // Joypad
                self.io[0x00] = value & 0x30; // 只允許高2位選擇列
                // TODO: 這裡應該觸發 joypad 狀態查詢，需連動輸入模組
            }
            0xFF01 => {
                // Serial transfer data
                self.io[0x01] = value;
                // TODO: 觸發 serial 傳輸
            }
            0xFF02 => {
                // Serial transfer control
                self.io[0x02] = value;
                // TODO: 觸發 serial 傳輸
            }
            0xFF04 => {
                // DIV (Divider) register
                self.io[0x04] = 0; // 寫入任何值都會重設為 0
            }
            0xFF05 => {
                // TIMA (Timer Counter)
                self.io[0x05] = value;
            }
            0xFF06 => {
                // TMA (Timer Modulo)
                self.io[0x06] = value;
            }
            0xFF07 => {
                // TAC (Timer Control)
                self.io[0x07] = value & 0x07; // 只允許低3位
            }
            0xFF0F => {
                // IF (Interrupt Flag)
                self.io[0x0F] = value & 0x1F; // 只允許低5位
            }
            0xFF10..=0xFF3F => {
                // Sound registers (APU)
                let idx = (addr - 0xFF00) as usize;
                self.io[idx] = value;
                // TODO: 觸發 APU 狀態變化
            }
            0xFF40 => {
                self.lcd_registers.lcdc = value;
                self.io[0x40] = value;
            }
            0xFF41 => {
                self.lcd_registers.stat = value;
                self.io[0x41] = value;
            }
            0xFF42 => {
                self.lcd_registers.scy = value;
                self.io[0x42] = value;
            }
            0xFF43 => {
                self.lcd_registers.scx = value;
                self.io[0x43] = value;
            }
            0xFF44 => {
                self.lcd_registers.ly = 0;
                self.io[0x44] = 0;
            } // 寫入任何值都會重設為 0
            0xFF45 => {
                self.lcd_registers.lyc = value;
                self.io[0x45] = value;
            }
            0xFF46 => { /* 已於前面 DMA 處理 */ }
            0xFF47 => {
                self.lcd_registers.bgp = value;
                self.io[0x47] = value;
            }
            0xFF48 => {
                self.lcd_registers.obp0 = value;
                self.io[0x48] = value;
            }
            0xFF49 => {
                self.lcd_registers.obp1 = value;
                self.io[0x49] = value;
            }
            0xFF4A => {
                self.lcd_registers.wy = value;
                self.io[0x4A] = value;
            }
            0xFF4B => {
                self.lcd_registers.wx = value;
                self.io[0x4B] = value;
            }
            0xFF4D => {
                self.io[0x4D] = value & 0x01;
            } // KEY1 (CGB)
            0xFF4F => {
                self.io[0x4F] = value & 0x01;
            } // VBK (CGB)
            0xFF51..=0xFF55 => {
                self.io[(addr - 0xFF00) as usize] = value;
            } // HDMA (CGB)
            0xFF68..=0xFF6B => {
                self.io[(addr - 0xFF00) as usize] = value;
            } // CGB palette
            0xFF70 => {
                self.io[0x70] = value & 0x07;
            } // SVBK (CGB)
            0xFF00..=0xFF7F => {
                // 其他 I/O 暫存器 fallback
                let idx = (addr - 0xFF00) as usize;
                self.io[idx] = value;
            }
            0xFF80..=0xFFFE => self.hram[(addr - 0xFF80) as usize] = value,
            0xFFFF => self.ie = value,
            _ => self.memory[addr as usize] = value,
        }
        Ok(())
    }
    /// 讀取 GameBoy 記憶體
    pub fn read_byte(&self, addr: u16) -> crate::core::error::Result<u8> {
        // MBC 控制區段
        if (0x0000..=0x7FFF).contains(&addr) {
            if let Some(ref mbc) = self.mbc {
                // 交由 MBC 控制器處理
                return Ok(mbc.read(addr, &self.rom));
            } else {
                // 無 MBC 時直接讀 ROM
                return Ok(self.rom.get(addr as usize).copied().unwrap_or(0xFF));
            }
        } else if (0xA000..=0xBFFF).contains(&addr) {
            if let Some(ref mbc) = self.mbc {
                // 交由 MBC 控制器處理
                return Ok(mbc.read(addr, &self.rom));
            } else {
                // 無 MBC 時直接讀內部 ERAM
                return Ok(self.eram[(addr - 0xA000) as usize]);
            }
        }
        match addr {
            0x8000..=0x9FFF => Ok(self.vram.read_byte((addr - 0x8000) as usize)),
            0xFE00..=0xFE9F => Ok(self.oam[(addr - 0xFE00) as usize]),
            0xFF00 => {
                // Joypad
                // 實作 Joypad 讀取副作用
                Ok(self.io[0x00])
                // 可呼叫 cpu::io::read_joypad()
            }
            0xFF01 => {
                // Serial transfer data
                Ok(self.io[0x01])
                // 可呼叫 cpu::io::read_serial()
            }
            0xFF02 => {
                // Serial transfer control
                Ok(self.io[0x02])
                // 可呼叫 cpu::io::read_serial()
            }
            0xFF04..=0xFF07 => {
                // Timer registers (DIV, TIMA, TMA, TAC)
                let idx = (addr - 0xFF00) as usize;
                Ok(self.io[idx])
                // 可呼叫 timer 讀取副作用
            }
            0xFF0F => {
                // IF (Interrupt Flag)
                Ok(self.io[0x0F])
            }
            0xFF40..=0xFF4B => {
                // LCD/PPU registers (LCDC, STAT, SCY, SCX, LY, LYC, DMA, BGP, OBP0, OBP1, WY, WX)
                let idx = (addr - 0xFF00) as usize;
                // 同步自 lcd_registers 結構
                let val = match addr {
                    0xFF40 => self.lcd_registers.lcdc,
                    0xFF41 => self.lcd_registers.stat,
                    0xFF42 => self.lcd_registers.scy,
                    0xFF43 => self.lcd_registers.scx,
                    0xFF44 => self.lcd_registers.ly,
                    0xFF45 => self.lcd_registers.lyc,
                    0xFF46 => self.lcd_registers.dma,
                    0xFF47 => self.lcd_registers.bgp,
                    0xFF48 => self.lcd_registers.obp0,
                    0xFF49 => self.lcd_registers.obp1,
                    0xFF4A => self.lcd_registers.wy,
                    0xFF4B => self.lcd_registers.wx,
                    _ => self.io[idx],
                };
                Ok(val)
            }
            0xFF10..=0xFF3F => {
                // Sound registers
                let idx = (addr - 0xFF00) as usize;
                Ok(self.io[idx])
                // 可呼叫 APU/Sound 讀取副作用
            }
            0xFF00..=0xFF7F => {
                // 其他 I/O 暫存器 fallback
                let idx = (addr - 0xFF00) as usize;
                Ok(self.io[idx])
            }
            0xFF80..=0xFFFE => Ok(self.hram[(addr - 0xFF80) as usize]),
            0xFFFF => Ok(self.ie),
            _ => Ok(self.memory[addr as usize]),
        }
    }
}
