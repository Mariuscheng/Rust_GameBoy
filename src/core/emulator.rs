// Emulator struct: 封裝 CPU、PPU、MMU 等核心元件，供主迴圈與 display backend 使用
use crate::core::cpu::cpu::CPU;
use crate::core::ppu::ppu::PPU;
use crate::core::mmu::mmu::MMU;

pub struct Emulator {
    pub cpu: CPU,
    pub ppu: PPU,
    pub mmu: MMU,
}

impl Emulator {
    pub fn step(&mut self) {
        // 這裡僅示意，實際應根據 emu 主流程調用 CPU/PPU/MMU 等
        self.cpu.step();
        // self.ppu.step(); // 若有 ppu step 可加上
        // 其他同步邏輯
    }
}
