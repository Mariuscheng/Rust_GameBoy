// Game Boy Emulator 基本功能測試
// 執行 cargo test 可驗證 CPU/MMU/PPU/APU 初始化與主迴圈

#[cfg(feature = "run_tests")]
mod tests {
    use crate::core::apu::APU;
    use crate::core::cpu::cpu::CPU;
    use crate::core::mmu::mmu::MMU;
    use crate::core::ppu::ppu::PPU;

    #[test]
    fn test_cpu_mmu_ppu_apu_init() {
        let mut mmu = MMU::default();
        let mut cpu = CPU {
            ime: false,
            registers: Default::default(),
            mmu: &mut mmu as *mut MMU,
            halted: false,
            stopped: false,
        };
        let mut ppu = PPU {
            mmu: &mut mmu as *mut MMU,
            lcd_enabled: true,
            framebuffer: vec![(0u8, 0u8, 0u8); 160 * 144],
            bgp: 0,
            obp0: 0,
            obp1: 0,
            scx: 0,
            scy: 0,
            wy: 0,
            wx: 0,
            lcdc: 0,
            last_frame_time: std::time::Instant::now(),
            fps_counter: 0,
            mode: 0,
            ly: 0,
            lyc: 0,
            stat: 0,
            dots: 0,
            oam: [0; 0xA0],
            vram: std::ptr::null_mut(),
        };
        let mut apu = APU::new();
        // 基本 step/mix 測試
        cpu.step();
        ppu.step();
        apu.mix(44100, 16);

        // === 自動驗證 framebuffer 內容 ===
        let fb = ppu.get_framebuffer();
        let total = fb.len();
        let black = fb
            .iter()
            .filter(|&&(r, g, b)| r == 0 && g == 0 && b == 0)
            .count();
        let white = fb
            .iter()
            .filter(|&&(r, g, b)| r == 224 && g == 248 && b == 208)
            .count();
        let nonzero = fb
            .iter()
            .filter(|&&(r, g, b)| r != 0 || g != 0 || b != 0)
            .count();
        println!(
            "[TEST] framebuffer pixels: total={}, black={}, white={}, nonzero={}",
            total, black, white, nonzero
        );
        // 若全黑或全白，代表未產生圖案；若有非零像素，代表有圖案或 ROM 畫面
        assert!(total == 160 * 144);
        assert!(black < total); // 不應全黑
        assert!(white < total); // 不應全白
        assert!(nonzero > 0); // 應有非零像素
    }
}
