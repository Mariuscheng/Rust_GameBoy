// Game Boy 模擬器主結構

use crate::apu::Apu;
use crate::cpu::Cpu;
use crate::joypad::Joypad;
use crate::mmu::{IoHandler, Mmu};
use crate::ppu::Ppu;
use crate::timer::Timer;

struct GameBoyIoWrapper {
    ppu: *const Ppu,
    apu: *const Apu,
    timer: *const Timer,
    joypad: *const Joypad,
}

impl GameBoyIoWrapper {
    fn new(ppu: &Ppu, apu: &Apu, timer: &Timer, joypad: &Joypad) -> Self {
        GameBoyIoWrapper {
            ppu: ppu as *const _,
            apu: apu as *const _,
            timer: timer as *const _,
            joypad: joypad as *const _,
        }
    }
}

impl IoHandler for GameBoyIoWrapper {
    fn read_io(&self, address: u16) -> u8 {
        unsafe {
            match address {
                0xFF00 => {
                    if !self.joypad.is_null() {
                        (*self.joypad).read_register()
                    } else {
                        0x0F
                    }
                }
                0xFF04..=0xFF07 => {
                    if !self.timer.is_null() {
                        (*self.timer).read_register(address)
                    } else {
                        0
                    }
                }
                0xFF10..=0xFF3F => {
                    if !self.apu.is_null() {
                        (*self.apu).read_register(address)
                    } else {
                        0
                    }
                }
                0xFF40..=0xFF4B => {
                    if !self.ppu.is_null() {
                        (*self.ppu).read_register(address)
                    } else {
                        0
                    }
                }
                _ => 0,
            }
        }
    }

    fn write_io(&mut self, address: u16, value: u8, interrupt_flags: &mut u8) {
        unsafe {
            match address {
                0xFF00 => {
                    if !self.joypad.is_null() {
                        let joypad = self.joypad as *mut Joypad;
                        (*joypad).write_register(value);
                    }
                }
                0xFF04..=0xFF07 => {
                    if !self.timer.is_null() {
                        let timer = self.timer as *mut Timer;
                        (*timer).write_register(address, value, interrupt_flags);
                    }
                }
                0xFF10..=0xFF3F => {
                    if !self.apu.is_null() {
                        let apu = self.apu as *mut Apu;
                        (*apu).write_register(address, value);
                    }
                }
                0xFF40..=0xFF4B => {
                    if !self.ppu.is_null() {
                        let ppu = self.ppu as *mut Ppu;
                        (*ppu).write_register(address, value, interrupt_flags);
                    }
                }
                _ => {}
            }
        }
    }
}

pub struct GameBoy {
    pub cpu: Cpu,
    pub mmu: Mmu,
    pub ppu: Ppu,
    pub apu: Apu,
    pub timer: Timer,
    pub joypad: Joypad,
    pub cycles: u64,
}

impl GameBoy {
    pub fn new() -> Box<Self> {
        let mut gb = Box::new(GameBoy {
            cpu: Cpu::new(),
            mmu: Mmu::new(),
            ppu: Ppu::new(),
            apu: Apu::new(),
            timer: Timer::new(),
            joypad: Joypad::new(),
            cycles: 0,
        });

        // 設置 I/O 處理器
        let io_wrapper = GameBoyIoWrapper::new(&gb.ppu, &gb.apu, &gb.timer, &gb.joypad);
        gb.mmu.set_io_handler(Box::new(io_wrapper));

        // 設置初始硬體狀態 (模擬啟動後狀態)
        gb.mmu.write_byte(0xFFFF, 0x00); // 關閉所有中斷
        gb.mmu.write_byte(0xFF40, 0x91); // 啟用 LCD, 背景, 圖塊集 0
        gb.mmu.write_byte(0xFF41, 0x85); // STAT
        gb.mmu.write_byte(0xFF44, 0x00); // LY

        gb
    }

    // 載入 ROM
    pub fn load_rom(&mut self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.mmu.load_rom(path)?;
        Ok(())
    }

    // 執行一個完整的幀 (70224 個時鐘循環)
    pub fn run_frame(&mut self) {
        let frame_cycles = 70224; // Game Boy 每幀的時鐘循環數
        let mut frame_cycle_count = 0;

        while frame_cycle_count < frame_cycles {
            // 執行一個 CPU 指令並獲取實際的周期數
            // 注意：Timer 需要在 CPU 執行期間同步更新
            let instruction_cycles = self.step_cpu_with_timing();
            frame_cycle_count += instruction_cycles as u64;
            self.cycles += instruction_cycles as u64;
        }
    }

    // 執行一個 CPU 指令，並在執行期間同步更新 Timer 和 PPU
    fn step_cpu_with_timing(&mut self) -> u32 {
        // 執行 CPU 指令
        let cycles = self.cpu.step(&mut self.mmu);

        // 批量更新 PPU 和 Timer
        let mut if_reg = self.mmu.if_reg;
        for _ in 0..cycles {
            self.ppu.tick(&self.mmu, &mut if_reg);
            self.timer.tick(&mut if_reg);
            self.apu.tick();
        }
        self.mmu.if_reg = if_reg;

        cycles
    }

    // 獲取當前畫面緩衝區
    pub fn get_framebuffer(&self) -> &[u8] {
        self.ppu.get_framebuffer()
    }

    pub fn should_render(&self) -> bool {
        self.ppu.mode == crate::ppu::LcdMode::VBlank && self.ppu.ly == 144
    }
}
