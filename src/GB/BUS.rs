use crate::GB::RAM::RAM;
use crate::interface::audio::SimpleAPUSynth;
use std::cmp::Ordering;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
enum RegKind {
    Scx,
    Wx,
    Bgp,
}

#[derive(Debug, Clone, Copy)]
struct RegEvent {
    x: u16, // pixel column 0..159 at which the new value takes effect
    kind: RegKind,
    val: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MbcType {
    None,
    Mbc1,
    Mbc3,
    Mbc5,
}

// Minimal Bus/MMU abstraction that proxies to RAM and maps key I/O registers.
pub struct Bus {
    ram: RAM,
    // Cartridge ROM and MBC
    rom: Vec<u8>,
    rom_banks: usize, // number of 16KB banks
    mbc: MbcType,
    // Common MBC state
    ram_enable: bool,
    rom_bank: u16,    // current switchable ROM bank (for 0x4000..0x7FFF)
    ram_bank: u8,     // current external RAM bank
    ext_ram: Vec<u8>, // external RAM backing store (multiple 8KB banks)
    // MBC1-specific
    mbc1_bank_low5: u8,
    mbc1_bank_high2: u8,
    mbc1_mode: u8, // 0=ROM banking, 1=RAM banking
    // MBC3-specific (basic RTC stub)
    mbc3_rtc_sel: Option<u8>, // 0x08..0x0C when selecting RTC regs
    mbc3_rtc_regs: [u8; 5],   // S, M, H, DL, DH
    // Interrupt registers
    ie: u8,  // 0xFFFF
    ifl: u8, // 0xFF0F
    // Joypad
    p1: u8,        // 0xFF00 (legacy snapshot)
    p1_sel: u8,    // selection bits (only bits4..5 matter): 1=unselected, 0=selected
    joyp_dpad: u8, // low nibble active-low: Right/Left/Up/Down
    joyp_btns: u8, // low nibble active-low: A/B/Select/Start
    // Timer
    div: u8,  // 0xFF04 (high 8 bits of internal 16-bit divider)
    tima: u8, // 0xFF05
    tma: u8,  // 0xFF06
    tac: u8,  // 0xFF07
    // DMA
    dma: u8, // 0xFF46 (last written value)
    // OAM DMA timing state (1 byte per 4 cycles, total 160 bytes)
    dma_active: bool,
    dma_src_base: u16,
    dma_pos: u16,         // 0..160 bytes copied
    dma_cycle_accum: u32, // cycles accumulated toward next byte copy
    dma_start_delay: u32, // initial latency after FF46 write before blocking/transfer starts (in t-cycles)
    // PPU basic registers
    lcdc: u8,   // 0xFF40
    stat_w: u8, // writable bits of STAT (we keep bits 3..6); read composes mode & coincidence
    scy: u8,    // 0xFF42
    scx: u8,    // 0xFF43
    ly: u8,     // 0xFF44 (read-only; write resets to 0)
    lyc: u8,    // 0xFF45
    bgp: u8,    // 0xFF47
    obp0: u8,   // 0xFF48
    obp1: u8,   // 0xFF49
    wy: u8,     // 0xFF4A
    wx: u8,     // 0xFF4B
    // PPU timing
    ppu_line_cycle: u32, // 0..=455 within a scanline
    ppu_mode: u8,        // 0=HBlank,1=VBlank,2=OAM,3=Transfer
    lcd_on_delay: u32,   // t-cycles to wait after LCDC ON before starting PPU at LY=0/Mode2
    // Simple framebuffer (DMG shades 0..3)
    framebuffer: [u8; 160 * 144],
    // internal counters
    div_counter: u32,       // low 8 bits of 16-bit system counter (in M-cycles)
    div_sub: u8,            // t-cycles within current M-cycle (0..3)
    tima_reload_delay: u32, // counts down t-cycles after overflow; when hits 0, reload TIMA=TMA and request IF
    // Window internal line counter (increments only when window is drawn)
    win_line: u8,
    // Dynamic per-line state for mid-line changes
    line_base_scx: u8,
    line_base_scy: u8,
    line_base_wx: u8,
    line_base_bgp: u8,
    scan_events: Vec<RegEvent>,
    // debug once-only markers
    dbg_lcdc_first_write_done: bool,
    dbg_vram_first_write_done: bool,
    #[allow(dead_code)]
    apu_dbg_printed: bool,
    // APU (very minimal): mirror of 0xFF10..=0xFF3F and a handle to synth
    apu_regs: [u8; 0x30],
    apu_synth: Option<Arc<Mutex<SimpleAPUSynth>>>,
    // Debug: enable minimal timer tracing when GB_DEBUG_TIMER=1
    dbg_timer: bool,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            ram: RAM::new(),
            rom: Vec::new(),
            rom_banks: 0,
            mbc: MbcType::None,
            ram_enable: false,
            rom_bank: 1,
            ram_bank: 0,
            ext_ram: Vec::new(),
            mbc1_bank_low5: 1,
            mbc1_bank_high2: 0,
            mbc1_mode: 0,
            mbc3_rtc_sel: None,
            mbc3_rtc_regs: [0; 5],
            ie: 0x00,
            ifl: 0x00,
            p1: 0xFF,     // default
            p1_sel: 0x30, // both unselected
            joyp_dpad: 0x0F,
            joyp_btns: 0x0F,
            div: 0x00,
            tima: 0x00,
            tma: 0x00,
            tac: 0x00,
            dma: 0x00,
            dma_active: false,
            dma_src_base: 0,
            dma_pos: 0,
            dma_cycle_accum: 0,
            dma_start_delay: 0,
            lcdc: 0x00,
            stat_w: 0x00,
            scy: 0x00,
            scx: 0x00,
            ly: 0x00,
            lyc: 0x00,
            bgp: 0xFC, // typical DMG default
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0x00,
            wx: 0x00,
            ppu_line_cycle: 0,
            ppu_mode: 2, // power-on: assume start of a line in OAM search
            lcd_on_delay: 0,
            framebuffer: [0; 160 * 144],
            div_counter: 0,
            div_sub: 0,
            tima_reload_delay: 0,
            win_line: 0,
            line_base_scx: 0,
            line_base_scy: 0,
            line_base_wx: 0,
            line_base_bgp: 0xFC,
            scan_events: Vec::with_capacity(64),
            dbg_lcdc_first_write_done: false,
            dbg_vram_first_write_done: false,
            apu_dbg_printed: false,
            apu_regs: [0; 0x30],
            apu_synth: None,
            dbg_timer: std::env::var("GB_DEBUG_TIMER").ok().map(|v| v != "0").unwrap_or(false),
        }
    }

    // --- Raw IE/IF accessors (bypass CPU bus gating) ---
    #[inline]
    pub fn get_ie_raw(&self) -> u8 {
        self.ie
    }
    #[inline]
    pub fn get_if_raw(&self) -> u8 {
        self.ifl | 0xE0
    }
    #[inline]
    pub fn set_if_raw(&mut self, v: u8) {
        self.ifl = v & 0x1F
    }

    // Internal read used by the DMA engine to fetch source bytes. This bypasses
    // CPU bus gating and VRAM/OAM access restrictions, and performs minimal
    // address mapping (ROM via MBC; otherwise direct RAM).
    #[inline]
    fn read_for_dma(&self, addr: u16) -> u8 {
        match addr {
            0x0000..=0x7FFF => {
                if self.rom_banks > 0 {
                    self.read_rom(addr)
                } else {
                    self.ram.read(addr)
                }
            }
            0xA000..=0xBFFF => {
                // External RAM read (respect enable and banking where possible)
                if self.ext_ram.is_empty() || !self.ram_enable {
                    0xFF
                } else {
                    let bank = match self.mbc {
                        MbcType::Mbc1 => {
                            if self.mbc1_mode & 1 == 0 {
                                0
                            } else {
                                (self.ram_bank & 0x03) as usize
                            }
                        }
                        MbcType::Mbc5 => (self.ram_bank & 0x0F) as usize,
                        _ => 0,
                    };
                    let base = bank * 0x2000;
                    let off = (addr as usize - 0xA000) & 0x1FFF;
                    self.ext_ram.get(base + off).copied().unwrap_or(0xFF)
                }
            }
            _ => self.ram.read(addr),
        }
    }

    // DMA helpers
    #[inline]
    pub fn is_dma_active(&self) -> bool {
        self.dma_active && self.dma_pos < 160 && self.dma_start_delay == 0
    }
    #[inline]
    pub fn dma_cycles_left(&self) -> u32 {
        if self.is_dma_active() {
            let total_left = (160u32 - self.dma_pos as u32) * 4;
            total_left.saturating_sub(self.dma_cycle_accum)
        } else {
            0
        }
    }

    /// 立刻評估 STAT/LYC 相關的中斷條件，必要時設定 IF.STAT (bit1)。
    /// - bit6：LYC=LY 中斷允許且目前相等 -> 觸發
    /// - bit5/bit4/bit3：依目前 ppu_mode 分別對應 OAM(2)/VBlank(1)/HBlank(0) 觸發
    #[inline]
    fn eval_stat_irq_immediate(&mut self) {
        // LYC=LY
        if (self.stat_w & 0x40) != 0 && self.ly == self.lyc {
            self.ifl |= 0x02;
        }
        // Mode IRQs
        match self.ppu_mode {
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

    #[inline]
    fn line_cycle_to_x(&self, cyc: u32) -> u16 {
        // Mode3 roughly 80..248 in our simple model; map 80->0 pixel
        if cyc <= 80 {
            0
        } else {
            let px = cyc - 80;
            if px >= 160 { 159 } else { px as u16 }
        }
    }

    #[inline]
    fn start_mode3_line_capture(&mut self) {
        if self.ly < 144 {
            self.line_base_scx = self.scx;
            self.line_base_scy = self.scy;
            self.line_base_wx = self.wx;
            self.line_base_bgp = self.bgp;
            self.scan_events.clear();
        }
    }

    #[inline]
    #[allow(dead_code)]
    fn record_scan_event(&mut self, kind: RegKind, val: u8) {
        if (self.lcdc & 0x80) == 0 {
            return;
        }
        if self.ppu_mode != 3 || self.ly >= 144 {
            return;
        }
        let x = self.line_cycle_to_x(self.ppu_line_cycle);
        if self.scan_events.len() < 128 {
            self.scan_events.push(RegEvent { x, kind, val });
        }
    }

    // Attach a simple synth so that APU register writes can generate audio
    pub fn attach_synth(&mut self, synth: Arc<Mutex<SimpleAPUSynth>>) {
        self.apu_synth = Some(synth);
        // Apply current register state to synth immediately
        self.apu_update_synth(true);
    }

    fn apu_update_synth(&mut self, just_triggered: bool) {
        if let Some(ref synth) = self.apu_synth {
            if let Ok(mut s) = synth.lock() {
                // NR52 Master
                let nr52 = self.apu_regs[(0x26 - 0x10) as usize];
                s.master_enable = (nr52 & 0x80) != 0;
                if !s.master_enable {
                    s.ch1_enable = false;
                    s.ch2_enable = false;
                }
                // NR51 Routing (0xFF25): bit0..3 R, bit4..7 L; if both sides off, treat as muted
                let nr51 = self.apu_regs[(0x25 - 0x10) as usize];
                let ch1_routed = ((nr51 & 0x01) != 0) || ((nr51 & 0x10) != 0);
                let ch2_routed = ((nr51 & 0x02) != 0) || ((nr51 & 0x20) != 0);
                // CH1 duty (NR11)
                let nr11 = self.apu_regs[(0x11 - 0x10) as usize]; // offset 0x01
                s.ch1_duty = (nr11 >> 6) & 0x03;
                // CH1 volume (NR12 upper nibble)
                let nr12 = self.apu_regs[(0x12 - 0x10) as usize]; // offset 0x02
                let init_vol = (nr12 >> 4) & 0x0F;
                // Master volume (NR50): average L(4..6) & R(0..2)
                let nr50 = self.apu_regs[(0x24 - 0x10) as usize]; // offset 0x14
                let l = ((nr50 >> 4) & 0x07) as f32;
                let r = (nr50 & 0x07) as f32;
                let master = ((l + r) / 2.0) / 7.0; // 0..1
                let base = (init_vol as f32) / 15.0;
                s.ch1_volume = base * master;
                // CH1 frequency from NR13/NR14 (lower 3)
                let n_lo = self.apu_regs[(0x13 - 0x10) as usize] as u16; // offset 0x03
                let nr14 = self.apu_regs[(0x14 - 0x10) as usize]; // offset 0x04
                let n_hi = (nr14 as u16) & 0x07;
                let n = (n_hi << 8) | n_lo;
                if n < 2048 {
                    s.ch1_freq_hz = 131_072.0 / (2048 - n) as f32;
                } else {
                    s.ch1_freq_hz = 0.0;
                }
                // Trigger (NR14 bit7)
                if s.master_enable && ((nr14 & 0x80) != 0 || just_triggered) {
                    s.ch1_enable = ch1_routed && s.ch1_volume > 0.0 && s.ch1_freq_hz > 0.0;
                    s.trigger();
                    // if !self.apu_dbg_printed {
                    //     println!(
                    //         "[APU] CH1 trigger: freq={:.1}Hz vol={:.2} master={}",
                    //         s.ch1_freq_hz,
                    //         s.ch1_volume,
                    //         ((nr52 & 0x80) != 0)
                    //     );
                    //     self.apu_dbg_printed = true;
                    // }
                }
                // If routing changed while playing, update enable accordingly (still simplified)
                if s.master_enable && !((nr14 & 0x80) != 0 || just_triggered) {
                    s.ch1_enable = ch1_routed && s.ch1_volume > 0.0 && s.ch1_freq_hz > 0.0;
                }

                // CH2: NR21..NR24
                let nr21 = self.apu_regs[(0x16 - 0x10) as usize]; // duty in bits6..7
                s.ch2_duty = (nr21 >> 6) & 0x03;
                let nr22 = self.apu_regs[(0x17 - 0x10) as usize];
                let init2 = (nr22 >> 4) & 0x0F;
                let base2 = (init2 as f32) / 15.0;
                s.ch2_volume = base2 * master;
                let n2_lo = self.apu_regs[(0x18 - 0x10) as usize] as u16;
                let nr24 = self.apu_regs[(0x19 - 0x10) as usize];
                let n2_hi = (nr24 as u16) & 0x07;
                let n2 = (n2_hi << 8) | n2_lo;
                if n2 < 2048 {
                    s.ch2_freq_hz = 131_072.0 / (2048 - n2) as f32;
                } else {
                    s.ch2_freq_hz = 0.0;
                }
                if s.master_enable && (nr24 & 0x80) != 0 {
                    s.ch2_enable = ch2_routed && s.ch2_volume > 0.0 && s.ch2_freq_hz > 0.0;
                    s.trigger();
                    // if !self.apu_dbg_printed {
                    //     println!(
                    //         "[APU] CH2 trigger: freq={:.1}Hz vol={:.2} master={}",
                    //         s.ch2_freq_hz,
                    //         s.ch2_volume,
                    //         ((nr52 & 0x80) != 0)
                    //     );
                    //     self.apu_dbg_printed = true;
                    // }
                }
                if s.master_enable && (nr24 & 0x80) == 0 {
                    s.ch2_enable = ch2_routed && s.ch2_volume > 0.0 && s.ch2_freq_hz > 0.0;
                }
            }
        }
    }

    /// Load a full ROM image and initialize basic MBC state
    pub fn load_rom(&mut self, data: Vec<u8>) {
        self.rom = data;
        self.rom_banks = (self.rom.len() + 0x3FFF) / 0x4000; // ceil
        // Detect cartridge type from header 0x0147 (if present)
        let cart_type = if self.rom.len() > 0x0147 { self.rom[0x0147] } else { 0x00 };
        self.mbc = match cart_type {
            0x01 | 0x02 | 0x03 => MbcType::Mbc1,
            0x0F | 0x10 | 0x11 | 0x12 | 0x13 => MbcType::Mbc3, // MBC3 (+Timer,Battery,RAM)
            0x19 | 0x1A | 0x1B | 0x1C | 0x1D | 0x1E => MbcType::Mbc5,
            _ => MbcType::None,
        };
        // External RAM size (header 0x0149)
        let ram_size_code = if self.rom.len() > 0x0149 { self.rom[0x0149] } else { 0 };
        let ram_banks = match ram_size_code {
            0x02 => 1,  // 8KB
            0x03 => 4,  // 32KB
            0x04 => 16, // 128KB
            0x05 => 8,  // 64KB
            _ => 0,
        };
        self.ext_ram = vec![0u8; ram_banks * 0x2000];
        // Reset MBC state
        self.ram_enable = false;
        self.rom_bank = 1;
        self.ram_bank = 0;
        self.mbc1_bank_low5 = 1;
        self.mbc1_bank_high2 = 0;
        self.mbc1_mode = 0;
        self.mbc3_rtc_sel = None;
        self.mbc3_rtc_regs = [0; 5];
        // Note: We no longer mirror ROM into RAM for 0x0000..0x7FFF; reads go via self.rom
        // println!(
        //     "[Bus] ROM loaded: {} bytes, banks={}, MBC={:?}, extRAM={} bytes",
        //     self.rom.len(),
        //     self.rom_banks,
        //     self.mbc,
        //     self.ext_ram.len()
        // );
    }

    #[inline]
    fn mbc1_calc_bank0(&self) -> u16 {
        if self.mbc1_mode & 1 == 0 { 0 } else { ((self.mbc1_bank_high2 as u16) & 0x03) << 5 }
    }
    #[inline]
    fn mbc1_calc_bankX(&self) -> u16 {
        let low5 = (self.mbc1_bank_low5 as u16) & 0x1F;
        let mut bank = if self.mbc1_mode & 1 == 0 {
            // ROM banking mode: combine high2:low5
            (low5) | (((self.mbc1_bank_high2 as u16) & 0x03) << 5)
        } else {
            // RAM banking mode: only low5 applies to ROM bank
            low5
        };
        if (bank & 0x1F) == 0 {
            bank |= 1;
        }
        bank
    }
    #[inline]
    fn read_rom(&self, addr: u16) -> u8 {
        if self.rom_banks == 0 {
            return 0xFF;
        }
        match self.mbc {
            MbcType::None => {
                let i = addr as usize;
                if i < self.rom.len() { self.rom[i] } else { 0xFF }
            }
            MbcType::Mbc1 => {
                if addr < 0x4000 {
                    let bank0 = (self.mbc1_calc_bank0() as usize) % self.rom_banks;
                    let base = bank0 * 0x4000;
                    let i = base + addr as usize;
                    if i < self.rom.len() { self.rom[i] } else { 0xFF }
                } else {
                    let bankx = (self.mbc1_calc_bankX() as usize) % self.rom_banks;
                    let base = bankx * 0x4000;
                    let i = base + (addr as usize - 0x4000);
                    if i < self.rom.len() { self.rom[i] } else { 0xFF }
                }
            }
            MbcType::Mbc3 => {
                if addr < 0x4000 {
                    let i = addr as usize;
                    if i < self.rom.len() { self.rom[i] } else { 0xFF }
                } else {
                    let mut bank = (self.rom_bank as usize) & 0x7F; // 7-bit
                    if bank == 0 {
                        bank = 1;
                    }
                    let base = (bank % self.rom_banks) * 0x4000;
                    let i = base + (addr as usize - 0x4000);
                    if i < self.rom.len() { self.rom[i] } else { 0xFF }
                }
            }
            MbcType::Mbc5 => {
                if addr < 0x4000 {
                    let base = 0usize; // fixed bank 0
                    let i = base + addr as usize;
                    if i < self.rom.len() { self.rom[i] } else { 0xFF }
                } else {
                    let bank = (self.rom_bank as usize) % self.rom_banks;
                    let base = bank * 0x4000;
                    let i = base + (addr as usize - 0x4000);
                    if i < self.rom.len() { self.rom[i] } else { 0xFF }
                }
            }
        }
    }

    #[inline]
    fn write_mbc(&mut self, addr: u16, val: u8) {
        match self.mbc {
            MbcType::None => { /* ignore writes to 0x0000..0x7FFF */ }
            MbcType::Mbc1 => {
                match addr {
                    0x0000..=0x1FFF => {
                        self.ram_enable = (val & 0x0F) == 0x0A;
                    }
                    0x2000..=0x3FFF => {
                        let mut low5 = val & 0x1F;
                        if low5 == 0 {
                            low5 = 1;
                        }
                        self.mbc1_bank_low5 = low5;
                    }
                    0x4000..=0x5FFF => {
                        self.mbc1_bank_high2 = val & 0x03;
                        if self.mbc1_mode & 1 == 1 {
                            // RAM bank is high2 in RAM banking mode
                            self.ram_bank = self.mbc1_bank_high2 & 0x03;
                        }
                    }
                    0x6000..=0x7FFF => {
                        self.mbc1_mode = val & 0x01;
                    }
                    _ => {}
                }
            }
            MbcType::Mbc3 => {
                match addr {
                    0x0000..=0x1FFF => {
                        // RAM enable
                        self.ram_enable = (val & 0x0F) == 0x0A;
                    }
                    0x2000..=0x3FFF => {
                        // ROM bank (7-bit), treat 0 as 1
                        let mut b = (val & 0x7F) as u16;
                        if b == 0 {
                            b = 1;
                        }
                        self.rom_bank = b;
                    }
                    0x4000..=0x5FFF => {
                        // RAM bank (0..3) or RTC select (0x08..0x0C)
                        let v = val & 0x0F;
                        if v <= 0x03 {
                            self.ram_bank = v;
                            self.mbc3_rtc_sel = None;
                        } else if (0x08..=0x0C).contains(&v) {
                            self.mbc3_rtc_sel = Some(v);
                        }
                    }
                    0x6000..=0x7FFF => {
                        // Latch clock: usually write 0 then 1; we ignore and keep simple stub
                        let _ = val;
                    }
                    _ => {}
                }
            }
            MbcType::Mbc5 => {
                match addr {
                    0x0000..=0x1FFF => {
                        self.ram_enable = (val & 0x0F) == 0x0A;
                    }
                    0x2000..=0x2FFF => {
                        self.rom_bank = (self.rom_bank & 0x100) | (val as u16);
                    }
                    0x3000..=0x3FFF => {
                        self.rom_bank = (self.rom_bank & 0x0FF) | (((val as u16) & 0x01) << 8);
                    }
                    0x4000..=0x5FFF => {
                        self.ram_bank = val & 0x0F;
                    }
                    0x6000..=0x7FFF => {
                        // MBC5: rumble enable lives here for some carts; ignore
                    }
                    _ => {}
                }
            }
        }
    }

    #[inline]
    fn bg_tilemap_base(&self) -> u16 {
        if (self.lcdc & 0x08) != 0 { 0x9C00 } else { 0x9800 }
    }

    #[inline]
    fn bg_tiledata_signed(&self) -> bool {
        (self.lcdc & 0x10) == 0 // 0 -> 0x8800 signed, 1 -> 0x8000 unsigned
    }

    fn render_scanline(&mut self) {
        let y = self.ly as usize;
        if y >= 144 {
            return;
        }
        let mut bg_color_idx = [0u8; 160];
        let mut shades = [0u8; 160];

        // Background
        if (self.lcdc & 0x01) != 0 {
            // Build dynamic values with mid-line events
            let scy = self.line_base_scy as u16;
            let mut scx = self.line_base_scx as u16;
            let v = ((self.ly as u16).wrapping_add(scy)) & 0xFF;
            let tilemap = self.bg_tilemap_base();
            let signed = self.bg_tiledata_signed();
            let row_in_tile = (v & 7) as u16;
            // Wrap to 32x32 tilemap
            let tile_row = ((v >> 3) & 31) as u16;
            // Sort events for deterministic order
            if !self.scan_events.is_empty() {
                self.scan_events.sort_by(|a, b| a.x.cmp(&b.x));
            }
            let mut ev_idx = 0usize;
            let mut curr_bgp = self.line_base_bgp;
            for x in 0..160u16 {
                // apply pending events at/after this pixel
                while ev_idx < self.scan_events.len() {
                    let e = self.scan_events[ev_idx];
                    match e.x.cmp(&(x as u16)) {
                        Ordering::Less => {
                            ev_idx += 1;
                            continue;
                        }
                        Ordering::Equal => {
                            match e.kind {
                                RegKind::Scx => scx = e.val as u16,
                                RegKind::Bgp => curr_bgp = e.val,
                                RegKind::Wx => { /* handled in window section */ }
                            }
                            ev_idx += 1;
                            continue;
                        }
                        Ordering::Greater => break,
                    }
                }
                let h = (x.wrapping_add(scx)) & 0xFF;
                let tile_col = ((h >> 3) & 31) as u16;
                let map_index = tile_row * 32 + tile_col; // 0..=1023
                let tile_id = self.ram.read(tilemap + map_index);
                let tile_addr = if signed {
                    // 0x8800 signed addressing: index -128..127 maps to 0x9000 + idx*16
                    let idx = tile_id as i8 as i16;
                    let base = 0x9000i32 + (idx as i32) * 16;
                    base as u16
                } else {
                    0x8000u16 + (tile_id as u16) * 16
                };
                let lo = self.ram.read(tile_addr + row_in_tile * 2);
                let hi = self.ram.read(tile_addr + row_in_tile * 2 + 1);
                let bit = 7 - ((h & 7) as u8);
                let lo_b = (lo >> bit) & 1;
                let hi_b = (hi >> bit) & 1;
                let color = (hi_b << 1) | lo_b;
                // Use running BGP value
                let shade = (curr_bgp >> (color * 2)) & 0x03;
                bg_color_idx[x as usize] = color;
                shades[x as usize] = shade;
            }
        } else {
            // BG disabled: color index treated as 0 so sprites are visible
            for x in 0..160usize {
                bg_color_idx[x] = 0;
                shades[x] = 0;
            }
        }

        // Window overlays BG
        if (self.lcdc & 0x20) != 0 && (self.lcdc & 0x01) != 0 {
            let wy = self.wy as u16;
            let window_may_draw = (self.ly as u16) >= wy && (self.wx as u16) <= 166;
            if window_may_draw {
                // Window tile map select is LCDC bit 6
                let tilemap = if (self.lcdc & 0x40) != 0 { 0x9C00 } else { 0x9800 };
                let signed = self.bg_tiledata_signed();
                // Use internal window line counter instead of (LY-WY)
                let wy_line = self.win_line as u16;
                let row_in_tile = (wy_line & 7) as u16;
                let tile_row = ((wy_line >> 3) & 31) as u16;
                // WX may change mid-line: track running value via event index
                let mut wx_val = self.line_base_wx;
                let mut ev_wx_idx = 0usize;
                let wx = wx_val as i16 - 7;
                if wx as i32 > 159 { /* window starts past right edge: nothing to draw */ }
                for x in 0..160i16 {
                    // apply Wx events at this pixel
                    while ev_wx_idx < self.scan_events.len() {
                        let e = self.scan_events[ev_wx_idx];
                        match e.x.cmp(&(x as u16)) {
                            Ordering::Less => {
                                ev_wx_idx += 1;
                                continue;
                            }
                            Ordering::Equal => {
                                if let RegKind::Wx = e.kind {
                                    wx_val = e.val;
                                }
                                ev_wx_idx += 1;
                                continue;
                            }
                            Ordering::Greater => break,
                        }
                    }
                    let wx = wx_val as i16 - 7;
                    if x < wx {
                        continue;
                    }
                    let wx_col = (x - wx) as u16;
                    if wx_col >= 160 {
                        break;
                    } // clamp window width to visible 160px
                    let tile_col = ((wx_col >> 3) & 31) as u16;
                    let map_index = tile_row * 32 + tile_col; // 0..=1023
                    let tile_id = self.ram.read(tilemap + map_index);
                    let tile_addr = if signed {
                        let idx = tile_id as i8 as i16;
                        let base = 0x9000i32 + (idx as i32) * 16;
                        base as u16
                    } else {
                        0x8000u16 + (tile_id as u16) * 16
                    };
                    let lo = self.ram.read(tile_addr + row_in_tile * 2);
                    let hi = self.ram.read(tile_addr + row_in_tile * 2 + 1);
                    let bit = 7 - ((wx_col & 7) as u8);
                    let lo_b = (lo >> bit) & 1;
                    let hi_b = (hi >> bit) & 1;
                    let color = (hi_b << 1) | lo_b;
                    let shade = (self.bgp >> (color * 2)) & 0x03;
                    let xi = x as usize;
                    if xi < 160 {
                        bg_color_idx[xi] = color;
                        shades[xi] = shade;
                    }
                }
                // Increment window line counter after drawing a visible window line
                self.win_line = self.win_line.wrapping_add(1);
            }
        }

        // Sprites overlay
        if (self.lcdc & 0x02) != 0 {
            let obj_size_8x16 = (self.lcdc & 0x04) != 0;
            let mut sprite_written = [false; 160];
            // DMG 每掃描線最多繪 10 個 sprite
            let mut drawn_on_line = 0u8;
            for i in 0..40u16 {
                let oam = 0xFE00 + i * 4;
                let sy = self.ram.read(oam) as i16 - 16;
                let sx = self.ram.read(oam + 1) as i16 - 8;
                let mut tile = self.ram.read(oam + 2);
                let attr = self.ram.read(oam + 3);
                let palette = if (attr & 0x10) != 0 { self.obp1 } else { self.obp0 };
                let xflip = (attr & 0x20) != 0;
                let yflip = (attr & 0x40) != 0;
                let behind_bg = (attr & 0x80) != 0;
                let height = if obj_size_8x16 { 16 } else { 8 };
                let y = self.ly as i16;
                if y < sy || y >= sy + height {
                    continue;
                }
                if drawn_on_line >= 10 {
                    continue;
                }
                let mut row = (y - sy) as u16;
                if yflip {
                    row = (height - 1) as u16 - row;
                }
                if obj_size_8x16 {
                    // For 8x16 sprites, the tile index refers to the top tile; bottom is +1
                    tile &= 0xFE;
                }
                let tile_index =
                    if obj_size_8x16 { tile.wrapping_add(((row / 8) as u8) & 1) } else { tile };
                let tile_row = row % 8;
                let tile_addr = 0x8000u16 + (tile_index as u16) * 16 + (tile_row as u16) * 2;
                let lo = self.ram.read(tile_addr);
                let hi = self.ram.read(tile_addr + 1);
                for px in 0..8u16 {
                    let mut bit = 7 - px as u8;
                    if xflip {
                        bit = px as u8;
                    }
                    let lo_b = (lo >> bit) & 1;
                    let hi_b = (hi >> bit) & 1;
                    let color = (hi_b << 1) | lo_b;
                    if color == 0 {
                        continue;
                    }
                    let x = sx + px as i16;
                    if x < 0 || x >= 160 {
                        continue;
                    }
                    let xi = x as usize;
                    if sprite_written[xi] {
                        continue;
                    }
                    if behind_bg && bg_color_idx[xi] != 0 {
                        continue;
                    }
                    let shade = (palette >> (color * 2)) & 0x03;
                    shades[xi] = shade;
                    sprite_written[xi] = true;
                }
                drawn_on_line = drawn_on_line.saturating_add(1);
            }
        }

        // Write out
        let base = y * 160;
        for x in 0..160usize {
            self.framebuffer[base + x] = shades[x];
        }
        // Clear events for next line usage
        self.scan_events.clear();
    }

    #[allow(dead_code)]
    pub fn get_fb_pixel(&self, x: usize, y: usize) -> u8 {
        if x < 160 && y < 144 { self.framebuffer[y * 160 + x] } else { 0 }
    }

    /// 直接取得整個 160x144 灰階 framebuffer（0..3），避免逐像素呼叫造成額外開銷
    pub fn framebuffer(&self) -> &[u8] {
        &self.framebuffer
    }

    #[inline]
    pub fn read(&self, addr: u16) -> u8 {
        // During OAM DMA, the CPU bus is blocked for most address ranges. Allow only HRAM
        // (FF80..FFFE) and IE (FFFF). Our PPU/DMA use RAM directly and are unaffected.
        if self.is_dma_active() {
            if !((0xFF80..=0xFFFE).contains(&addr) || addr == 0xFFFF) {
                return 0xFF;
            }
        }
        match addr {
            0x0000..=0x7FFF => {
                // If a ROM is loaded, read via MBC mapping; otherwise fall back to RAM
                if self.rom_banks > 0 {
                    return self.read_rom(addr);
                } else {
                    return self.ram.read(addr);
                }
            }
            0xFF40 => self.lcdc,
            0xFF41 => {
                // Compose STAT: bit7=1; bits3..6 from stat_w; bit2 coincidence; bits1..0 current mode
                let coincidence = if self.ly == self.lyc { 0x04 } else { 0x00 };
                0x80 | (self.stat_w & 0x78) | coincidence | (self.ppu_mode & 0x03)
            }
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFFFF => self.ie,
            0xFF0F => self.ifl | 0xE0, // upper bits often read as 1 on real HW; mask to be safe
            0xFF00 => {
                // Compose per selection
                let res = 0xC0 | (self.p1_sel & 0x30);
                let mut low = 0x0F;
                if (self.p1_sel & 0x10) == 0 {
                    // select dpad
                    low &= self.joyp_dpad;
                }
                if (self.p1_sel & 0x20) == 0 {
                    // select buttons
                    low &= self.joyp_btns;
                }
                res | (low & 0x0F)
            }
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac | 0xF8, // only low 3 bits are used
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            0xFF46 => self.dma,
            0xFF10..=0xFF3F => {
                let idx = (addr - 0xFF10) as usize;
                self.apu_regs[idx]
            }
            // Cart RAM
            0xA000..=0xBFFF => {
                match self.mbc {
                    MbcType::Mbc3 => {
                        // If RTC selected, read RTC reg; else read RAM bank 0..3
                        if let Some(sel) = self.mbc3_rtc_sel {
                            let idx = (sel - 0x08) as usize; // 0..=4
                            self.mbc3_rtc_regs.get(idx).copied().unwrap_or(0)
                        } else if self.ext_ram.is_empty() || !self.ram_enable {
                            0xFF
                        } else {
                            let bank = (self.ram_bank & 0x03) as usize;
                            let base = bank * 0x2000;
                            let off = (addr as usize - 0xA000) & 0x1FFF;
                            self.ext_ram.get(base + off).copied().unwrap_or(0xFF)
                        }
                    }
                    _ => {
                        if self.ext_ram.is_empty() || !self.ram_enable {
                            0xFF
                        } else {
                            let bank = match self.mbc {
                                MbcType::Mbc1 => {
                                    if self.mbc1_mode & 1 == 0 {
                                        0
                                    } else {
                                        (self.ram_bank & 0x03) as usize
                                    }
                                }
                                MbcType::Mbc5 => (self.ram_bank & 0x0F) as usize,
                                _ => 0,
                            };
                            let base = bank * 0x2000;
                            let off = (addr as usize - 0xA000) & 0x1FFF;
                            self.ext_ram.get(base + off).copied().unwrap_or(0xFF)
                        }
                    }
                }
            }
            // VRAM read restriction during mode 3 when LCD is on
            0x8000..=0x9FFF => {
                if (self.lcdc & 0x80) != 0 && self.ppu_mode == 3 {
                    0xFF
                } else {
                    self.ram.read(addr)
                }
            }
            // OAM read restriction during mode 2/3 when LCD is on
            0xFE00..=0xFE9F => {
                if (self.lcdc & 0x80) != 0 && (self.ppu_mode == 2 || self.ppu_mode == 3) {
                    0xFF
                } else {
                    self.ram.read(addr)
                }
            }
            _ => self.ram.read(addr),
        }
    }

    #[inline]
    pub fn write(&mut self, addr: u16, val: u8) {
        // During OAM DMA, block CPU writes to most memory. Allow only HRAM (FF80..FFFE) and
        // IE (FFFF) so programs in HRAM can still use stack/vars and change IE.
        if self.is_dma_active() && !((0xFF80..=0xFFFE).contains(&addr) || addr == 0xFFFF) {
            // Ignore writes while DMA owns the bus (except HRAM/IE)
            return;
        }
        match addr {
            0x0000..=0x7FFF => {
                // If a ROM is loaded, these are MBC control writes; else write to RAM (for tests)
                if self.rom_banks > 0 {
                    self.write_mbc(addr, val);
                } else {
                    self.ram.write(addr, val);
                }
            }
            0xFF40 => {
                if !self.dbg_lcdc_first_write_done {
                    // println!(
                    //     "[PPU] LCDC first write: {:02X} (LCD {} | BG={} WIN={} OBJ={})",
                    //     val,
                    //     if (val & 0x80) != 0 { "ON" } else { "OFF" },
                    //     (val & 0x01) != 0,
                    //     (val & 0x20) != 0,
                    //     (val & 0x02) != 0
                    // );
                    self.dbg_lcdc_first_write_done = true;
                }
                let prev = self.lcdc;
                self.lcdc = val;
                if (prev & 0x80) == 0 && (val & 0x80) != 0 {
                    // LCD turned ON: schedule start at the next M-cycle boundary
                    self.win_line = 0;
                    let sub = self.div_sub as u32;
                    self.lcd_on_delay = if sub == 0 { 4 } else { 4 - sub };
                }
                if (prev & 0x80) != 0 && (val & 0x80) == 0 {
                    // LCD turned OFF: reset LY and PPU timing state
                    self.win_line = 0;
                    self.ly = 0;
                    self.ppu_line_cycle = 0;
                    self.ppu_mode = 0; // HBlank
                    self.lcd_on_delay = 0;
                    self.eval_stat_irq_immediate();
                }
                // Reset window line counter when window enable (bit 5) toggles
                if ((prev ^ val) & 0x20) != 0 {
                    self.win_line = 0;
                }
            }
            0xFF41 => {
                // 只允許寫入 3..6，但寫入後需立即依目前狀態評估是否產生 STAT IRQ
                self.stat_w = val & 0x78; // only bits 3..6 writable
                self.eval_stat_irq_immediate();
            }
            0xFF42 => {
                self.scy = val;
                // SCY mid-line generally has limited visible effect in our simple renderer; ignore
            }
            0xFF43 => {
                self.scx = val;
                self.record_scan_event(RegKind::Scx, val);
            }
            0xFF44 => {
                // writing any value resets LY to 0 and line cycle to start (mode 2)
                self.ly = 0;
                self.ppu_line_cycle = 0;
                self.ppu_mode = 2;
                self.win_line = 0;
                // 依據新 LY 立即檢查 LYC=LY 與當前模式的 STAT 中斷
                self.eval_stat_irq_immediate();
            }
            0xFF45 => {
                // 寫入 LYC 需要立刻更新 coincidence 與可能的 STAT 中斷
                self.lyc = val;
                self.eval_stat_irq_immediate();
            }
            0xFFFF => self.ie = val,
            0xFF0F => self.ifl = val & 0x1F,
            0xFF00 => {
                // Only bits 4..5 are writable selection control on DMG
                self.p1_sel = val & 0x30;
                // Snapshot for readback/debug
                self.p1 = 0xC0 | (self.p1_sel & 0x30);
            }
            0xFF04 => {
                // Reset DIV; may cause a falling-edge tick
                let enabled = (self.tac & 0x04) != 0;
                let prev_input = if enabled { self.current_timer_input() } else { 0 };
                self.div = 0x00;
                self.div_counter = 0;
                self.div_sub = 0;
                if enabled && prev_input == 1 {
                    if self.tima == 0xFF {
                        self.tima = 0x00;
                        self.tima_reload_delay = 4;
                    } else {
                        self.tima = self.tima.wrapping_add(1);
                    }
                }
            }
            0xFF05 => {
                // TIMA write during overflow delay windows
                if self.tima_reload_delay > 1 {
                    // Cycle A: cancel reload and accept write
                    self.tima_reload_delay = 0;
                    self.tima = val;
                } else if self.tima_reload_delay == 1 {
                    // Cycle B: ignore write to TIMA
                } else {
                    self.tima = val;
                }
            }
            0xFF06 => {
                self.tma = val;
                // In cycle B, TIMA is also set immediately
                if self.tima_reload_delay == 1 {
                    self.tima = val;
                }
            }
            0xFF07 => {
                // TAC write; handle 1->0 transitions of selected source while enabled
                let prev_enabled = (self.tac & 0x04) != 0;
                let prev_bit_idx = self.tac_to_div_bit_index();
                let div16: u16 = ((self.div as u16) << 8) | ((self.div_counter as u16) & 0x00FF);
                let prev_input = if prev_enabled { ((div16 >> prev_bit_idx) & 1) as u8 } else { 0 };
                self.tac = val & 0x07;
                let now_enabled = (self.tac & 0x04) != 0;
                if now_enabled {
                    let now_bit_idx = self.tac_to_div_bit_index();
                    let now_input = ((div16 >> now_bit_idx) & 1) as u8;
                    if prev_enabled && prev_input == 1 && now_input == 0 {
                        if self.tima == 0xFF {
                            self.tima = 0x00;
                            // On overflow due to TAC source change, reload after exactly 1 M-cycle
                            self.tima_reload_delay = 4;
                        } else {
                            self.tima = self.tima.wrapping_add(1);
                        }
                    }
                } else {
                    // DMG quirk: disabling when input was 1 also ticks (AND edge)
                    if prev_enabled && prev_input == 1 {
                        if self.tima == 0xFF {
                            self.tima = 0x00;
                            // Disabling when input was 1 also triggers overflow; reload after 1 M-cycle
                            self.tima_reload_delay = 4;
                        } else {
                            self.tima = self.tima.wrapping_add(1);
                        }
                    }
                }
            }
            0xFF46 => {
                // OAM DMA start
                self.dma = val;
                self.dma_active = true;
                self.dma_src_base = (val as u16) << 8;
                self.dma_pos = 0;
                self.dma_cycle_accum = 0;
                self.dma_start_delay = 4; // 1 M-cycle grace
            }
            0xFF47 => {
                self.bgp = val;
                self.record_scan_event(RegKind::Bgp, val);
            }
            0xFF48 => self.obp0 = val,
            0xFF49 => self.obp1 = val,
            0xFF4A => {
                self.wy = val;
                self.win_line = 0;
            }
            0xFF4B => {
                self.wx = val;
                self.record_scan_event(RegKind::Wx, val);
            }
            0xFF10..=0xFF3F => {
                // Mirror write and update synth
                let idx = (addr - 0xFF10) as usize;
                self.apu_regs[idx] = val;
                let mut just_triggered = false;
                if addr == 0xFF26 {
                    // NR52: when disabling master, clear enables
                    if (val & 0x80) == 0 {
                        if let Some(ref synth) = self.apu_synth {
                            if let Ok(mut s) = synth.lock() {
                                s.master_enable = false;
                                s.ch1_enable = false;
                                s.ch2_enable = false;
                            }
                        }
                    }
                }
                if addr == 0xFF14 {
                    if (val & 0x80) != 0 {
                        // trigger
                        just_triggered = true;
                    }
                }
                self.apu_update_synth(just_triggered);
            }
            // Cart RAM / RTC
            0xA000..=0xBFFF => {
                match self.mbc {
                    MbcType::Mbc3 => {
                        if let Some(sel) = self.mbc3_rtc_sel {
                            let idx = (sel - 0x08) as usize;
                            if idx < 5 {
                                self.mbc3_rtc_regs[idx] = val;
                            }
                        } else if !(self.ext_ram.is_empty() || !self.ram_enable) {
                            let bank = (self.ram_bank & 0x03) as usize;
                            let base = bank * 0x2000;
                            let off = (addr as usize - 0xA000) & 0x1FFF;
                            if base + off < self.ext_ram.len() {
                                self.ext_ram[base + off] = val;
                            }
                        }
                    }
                    _ => {
                        if self.ext_ram.is_empty() || !self.ram_enable {
                            // ignore
                        } else {
                            let bank = match self.mbc {
                                MbcType::Mbc1 => {
                                    if self.mbc1_mode & 1 == 0 {
                                        0
                                    } else {
                                        (self.ram_bank & 0x03) as usize
                                    }
                                }
                                MbcType::Mbc5 => (self.ram_bank & 0x0F) as usize,
                                _ => 0,
                            };
                            let base = bank * 0x2000;
                            let off = (addr as usize - 0xA000) & 0x1FFF;
                            if base + off < self.ext_ram.len() {
                                self.ext_ram[base + off] = val;
                            }
                        }
                    }
                }
            }
            // VRAM write restriction during mode 3 when LCD is on
            0x8000..=0x9FFF => {
                if (self.lcdc & 0x80) != 0 && self.ppu_mode == 3 { /* ignore */
                } else {
                    if !self.dbg_vram_first_write_done {
                        // println!(
                        //     "[PPU] First VRAM write @{:04X} = {:02X} | LCDC={:02X} LY={} STAT={:02X}",
                        //     addr,
                        //     val,
                        //     self.lcdc,
                        //     self.ly,
                        //     0x80 | (self.stat_w & 0x78)
                        //         | (if self.ly == self.lyc { 0x04 } else { 0 })
                        //         | (self.ppu_mode & 0x03)
                        // );
                        self.dbg_vram_first_write_done = true;
                    }
                    self.ram.write(addr, val)
                }
            }
            // OAM write restriction during mode 2/3 when LCD is on
            0xFE00..=0xFE9F => {
                if (self.lcdc & 0x80) != 0 && (self.ppu_mode == 2 || self.ppu_mode == 3) { /* ignore */
                } else {
                    self.ram.write(addr, val)
                }
            }
            _ => self.ram.write(addr, val),
        }
    }

    // Optional helpers
    pub fn set_joypad_rows(&mut self, dpad_low_nibble: u8, btn_low_nibble: u8) {
        self.joyp_dpad = dpad_low_nibble & 0x0F;
        self.joyp_btns = btn_low_nibble & 0x0F;
    }
    #[allow(dead_code)]
    pub fn request_interrupt(&mut self, mask: u8) {
        self.ifl |= mask & 0x1F;
    }

    #[inline]
    fn tima_period_cycles(&self) -> u32 {
        match self.tac & 0x03 {
            0x00 => 1024, // 4096 Hz
            0x01 => 16,   // 262144 Hz
            0x02 => 64,   // 65536 Hz
            0x03 => 256,  // 16384 Hz
            _ => 1024,
        }
    }

    // --- Timer helpers (edge-based model) ---
    // Map TAC[1:0] to DIV bit index according to Pan Docs: 00->bit9, 01->bit3, 10->bit5, 11->bit7
    #[inline]
    fn tac_to_div_bit_index(&self) -> u8 {
        match self.tac & 0x03 {
            0x00 => 9,
            0x01 => 3,
            0x02 => 5,
            _ => 7,
        }
    }

    // Read current selected input clock bit from the 16-bit divider
    #[inline]
    fn current_timer_input(&self) -> u8 {
        let div16: u16 = ((self.div as u16) << 8) | ((self.div_counter as u16) & 0x00FF);
        let bit = self.tac_to_div_bit_index();
        ((div16 >> bit) & 1) as u8
    }

    // Advance divider by t-cycles and process TIMA on falling edges of selected DIV bit.
    // Always advance the divider regardless of pending reload; if reload is pending, ignore further ticks until reload occurs.
    fn timer_advance(&mut self, mut c: u32) {
        while c > 0 {
            // Compute time to next relevant event: input toggle or pending reload completion
            let enabled = (self.tac & 0x04) != 0;
            let bit = self.tac_to_div_bit_index();
            let div16: u16 = ((self.div as u16) << 8) | ((self.div_counter as u16) & 0x00FF);
            let mask: u16 = 1u16 << bit;
            let low_mask: u16 = mask - 1;
            let lower = div16 & low_mask;
            let to_toggle: u32 = if enabled { (low_mask - lower + 1) as u32 } else { c };
            let to_reload: u32 =
                if self.tima_reload_delay > 0 { self.tima_reload_delay } else { u32::MAX };
            let step = to_toggle.min(to_reload).min(c);

            // Determine previous input before advancing
            let prev_input = if enabled { ((div16 & mask) != 0) as u8 } else { 0 };

            // Advance divider and sub-phase by 'step' t-cycles
            let add = step;
            let sum = self.div_counter.wrapping_add(add);
            let new_low = sum & 0xFF;
            let carry = sum >> 8;
            self.div_counter = new_low;
            self.div = self.div.wrapping_add(carry as u8);
            self.div_sub = ((self.div_sub as u32 + step) & 3) as u8;
            c -= step;

            // Progress any pending reload delay and complete if it expires in this step
            if self.tima_reload_delay > 0 {
                if to_reload <= step {
                    // Reload completes exactly within this time slice
                    self.tima_reload_delay = 0;
                    self.tima = self.tma;
                    self.ifl |= 0x04; // IF.TIMER
                    if self.dbg_timer {
                        println!(
                            "[TMR] reload TIMA=TMA ({:02X}) IF|=04  div={:02X}:{:02X} sub={}",
                            self.tima, self.div, self.div_counter as u8, self.div_sub
                        );
                    }
                    // After reload, continue; ticks in the same t-cycle window are not processed
                    continue;
                } else {
                    // Not yet completed: decrease remaining delay by the elapsed step
                    self.tima_reload_delay -= step;
                }
            }

            // If we reached a toggle, evaluate falling edge and tick (only if not in reload delay)
            if enabled && to_toggle <= step && self.tima_reload_delay == 0 {
                let new_div16: u16 =
                    ((self.div as u16) << 8) | ((self.div_counter as u16) & 0x00FF);
                let new_input = ((new_div16 & mask) != 0) as u8;
                if prev_input == 1 && new_input == 0 {
                    if self.tima == 0xFF {
                        self.tima = 0x00;
                        self.tima_reload_delay = 4; // reload after one M-cycle
                        if self.dbg_timer {
                            println!(
                                "[TMR] tick: overflow schedule reload+IF in 4t  div={:02X}:{:02X} sub={}",
                                self.div, self.div_counter as u8, self.div_sub
                            );
                        }
                    } else {
                        self.tima = self.tima.wrapping_add(1);
                        if self.dbg_timer {
                            println!(
                                "[TMR] tick: TIMA++ -> {:02X}  div={:02X}:{:02X} sub={}",
                                self.tima, self.div, self.div_counter as u8, self.div_sub
                            );
                        }
                    }
                }
            }
        }
    }

    pub fn step(&mut self, cycles: u64) {
        let c = cycles as u32;
        // Progress OAM DMA first: 1 byte per 4 cycles (after initial start delay)
        if self.dma_active {
            let mut remain = c;
            // Consume initial start delay before bus becomes blocked/bytes start copying
            if self.dma_start_delay > 0 {
                let d = self.dma_start_delay.min(remain);
                self.dma_start_delay -= d;
                remain -= d;
            }
            while remain > 0 && self.dma_pos < 160 {
                let step = (4u32 - self.dma_cycle_accum).min(remain);
                self.dma_cycle_accum += step;
                remain -= step;
                if self.dma_cycle_accum >= 4 {
                    // Perform one DMA byte copy
                    self.dma_cycle_accum = 0;
                    let src = self.dma_src_base.wrapping_add(self.dma_pos);
                    let b = self.read_for_dma(src);
                    // Writes to OAM during DMA succeed regardless of LCD mode
                    self.ram.write(0xFE00u16.wrapping_add(self.dma_pos), b);
                    self.dma_pos += 1;
                    if self.dma_pos >= 160 {
                        self.dma_active = false;
                        break;
                    }
                }
            }
        }
        // Advance timers using edge-based model and 1 M-cycle delayed reload
        self.timer_advance(c);

        // PPU timing: only when LCD is on (LCDC bit 7)
        if (self.lcdc & 0x80) != 0 {
            // Handle LCD turn-on delay before starting PPU counters
            if self.lcd_on_delay > 0 {
                if c >= self.lcd_on_delay {
                    // consume delay and start fresh at LY=0/Mode2 immediately
                    self.lcd_on_delay = 0;
                    self.ly = 0;
                    self.ppu_line_cycle = 0;
                    self.ppu_mode = 2;
                    self.eval_stat_irq_immediate();
                } else {
                    self.lcd_on_delay -= c;
                    return;
                }
            }
            let mut remain = c;
            while remain > 0 {
                // Determine current mode boundaries for this line
                let (mode, boundary) = if self.ly >= 144 {
                    (1u8, 456u32)
                } else if self.ppu_line_cycle < 80 {
                    (2u8, 80u32)
                } else {
                    // Variable Mode 3 length: base 172 + SCX low 3 alignment (rough model)
                    let mode3_len = 172u32 + ((self.scx & 0x07) as u32);
                    let mode3_end = 80u32 + mode3_len;
                    if self.ppu_line_cycle < mode3_end { (3u8, mode3_end) } else { (0u8, 456u32) }
                };
                // Handle mode transition
                if mode != self.ppu_mode {
                    // Mode change: fire STAT interrupts as needed
                    match mode {
                        2 => {
                            if (self.stat_w & 0x20) != 0 {
                                self.ifl |= 0x02;
                            }
                        } // OAM
                        1 => {
                            if (self.stat_w & 0x10) != 0 {
                                self.ifl |= 0x02;
                            }
                        } // VBlank
                        0 => {
                            // Entering HBlank; if we just finished mode 3 on a visible line, render it
                            if self.ppu_mode == 3 && self.ly < 144 {
                                self.render_scanline();
                            }
                            if (self.stat_w & 0x08) != 0 {
                                self.ifl |= 0x02;
                            }
                        } // HBlank
                        3 => {
                            self.start_mode3_line_capture();
                            // Mode 3 doesn't have its own STAT bit; keep as is
                        }
                        _ => {}
                    }
                    self.ppu_mode = mode;
                }
                // Advance by a small step to reach the boundary
                let step = (boundary - self.ppu_line_cycle).min(remain);
                self.ppu_line_cycle += step;
                remain -= step;
                if self.ppu_line_cycle >= 456 {
                    // End of line
                    self.ppu_line_cycle = 0;
                    self.ly = self.ly.wrapping_add(1);
                    if self.ly == 0 {
                        // New frame
                        self.win_line = 0;
                    }
                    if self.ly == 144 {
                        // Entering VBlank
                        self.ifl |= 0x01; // VBlank IF
                        if (self.stat_w & 0x10) != 0 {
                            self.ifl |= 0x02;
                        } // STAT VBlank
                    }
                    if self.ly > 153 {
                        self.ly = 0;
                    }
                    // STAT coincidence
                    if (self.stat_w & 0x40) != 0 && self.ly == self.lyc {
                        self.ifl |= 0x02;
                    }
                    // Entering VBlank is handled when mode becomes 1 at ly>=144 in next loop iteration
                    // Keep mode for next line start (will be recalculated at top)
                }
            }
            // Ensure coincidence interrupt is raised if LY==LYC after processing this batch
            if (self.stat_w & 0x40) != 0 && self.ly == self.lyc {
                self.ifl |= 0x02;
            }
        } else {
            // LCD off: reset PPU timing state
            self.ppu_mode = 0;
            self.ppu_line_cycle = 0;
            self.win_line = 0;
            self.ly = 0;
        }
    }
}
