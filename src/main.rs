#![cfg_attr(not(debug_assertions), allow(unused_imports, unused_variables))]
#![allow(clippy::too_many_arguments, clippy::upper_case_acronyms)]
#[allow(non_snake_case)]
mod GB;
mod interface;

use std::env;
use std::fs;
use std::io::Read;
use std::path::Path;

use interface::audio::{AudioInterface, SimpleAPUSynth};
use interface::sdl3_display::SdlDisplay;
use sdl3::keyboard::Scancode;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

// 可開關的除錯旗標：預設關閉以避免不必要的計算
const DEBUG_HEARTBEAT: bool = false; // 每 50 萬指令心跳偵錯
const DEBUG_FRAME_INFO: bool = false; // 每 30 幀列印 VRAM/鍵盤摘要

// 常數：DMG 時脈/FPS 與畫面大小
const DMG_CPU_HZ: f64 = 4_194_304.0;
const DMG_FPS: f64 = 59.7275;
const SCREEN_W: usize = 160;
const SCREEN_H: usize = 144;

// 解析命令列參數：--speed <f64>、--volume <f32>、<rompath>
fn parse_cli() -> (Option<String>, f64, Option<f32>) {
    let mut rom_arg: Option<String> = None;
    let mut speed_factor: f64 = 1.0;
    let mut user_volume: Option<f32> = None;
    let mut args = env::args().skip(1).peekable();
    while let Some(arg) = args.next() {
        if arg == "--speed" {
            if let Some(val) = args.peek() {
                if !val.starts_with("--") {
                    if let Ok(v) = val.parse::<f64>() {
                        speed_factor = if v > 0.01 { v } else { 0.01 };
                        let _ = args.next();
                        continue;
                    }
                }
            }
            eprintln!("--speed 需要一個數值，例如 --speed 0.99 或 1.02");
        } else if arg == "--volume" {
            if let Some(val) = args.peek() {
                if !val.starts_with("--") {
                    if let Ok(v) = val.parse::<f32>() {
                        user_volume = Some(v.max(0.0).min(2.0));
                        let _ = args.next();
                        continue;
                    }
                }
            }
            eprintln!("--volume 需要一個 0.0..2.0 的數值，例如 --volume 0.3");
        } else if arg.starts_with("--") {
            // 忽略未知旗標
        } else if rom_arg.is_none() {
            rom_arg = Some(arg);
        }
    }
    (rom_arg, speed_factor, user_volume)
}

// 嘗試載入 ROM：優先使用傳入路徑，否則從常見路徑清單挑選
fn try_load_rom(cpu: &mut GB::CPU::CPU, rom_arg: Option<String>) -> bool {
    if let Some(path) = rom_arg {
        let p = Path::new(&path);
        if p.exists() {
            if let Ok(mut f) = fs::File::open(p) {
                let mut buf = Vec::new();
                let _ = f.read_to_end(&mut buf);
                cpu.memory.load_rom(buf);
                return true;
            }
        } else {
            eprintln!("ROM path not found: {} (fallback to defaults)", p.display());
        }
    }
    let candidates = [
        Path::new("roms/interrupt_time.gb"),
    ];
    for p in candidates.iter() {
        if p.exists() {
            if let Ok(mut f) = fs::File::open(p) {
                let mut buf = Vec::new();
                let _ = f.read_to_end(&mut buf);
                cpu.memory.load_rom(buf);
                return true;
            }
        }
    }
    eprintln!("No ROM found. Provide a path: cargo run -- <path-to-rom.gb>");
    false
}

// 依 Pan Docs 常見 post-BIOS 預設初始化寄存器與 IO
fn init_post_bios(cpu: &mut GB::CPU::CPU) {
    cpu.registers.set_a(0x01);
    cpu.registers.set_f(0xB0);
    cpu.registers.set_b(0x00);
    cpu.registers.set_c(0x13);
    cpu.registers.set_d(0x00);
    cpu.registers.set_e(0xD8);
    cpu.registers.set_h(0x01);
    cpu.registers.set_l(0x4D);
    cpu.registers.set_sp(0xFFFE);
    cpu.registers.set_pc(0x0100);
    cpu.memory.write(0xFF05, 0x00); // TIMA
    cpu.memory.write(0xFF06, 0x00); // TMA
    cpu.memory.write(0xFF07, 0x00); // TAC
    cpu.memory.write(0xFF10, 0x80);
    cpu.memory.write(0xFF11, 0xBF);
    cpu.memory.write(0xFF12, 0xF3);
    cpu.memory.write(0xFF14, 0xBF);
    cpu.memory.write(0xFF16, 0x3F);
    cpu.memory.write(0xFF17, 0x00);
    cpu.memory.write(0xFF19, 0xBF);
    cpu.memory.write(0xFF1A, 0x7F);
    cpu.memory.write(0xFF1B, 0xFF);
    cpu.memory.write(0xFF1C, 0x9F);
    cpu.memory.write(0xFF1E, 0xBF);
    cpu.memory.write(0xFF20, 0xFF);
    cpu.memory.write(0xFF21, 0x00);
    cpu.memory.write(0xFF22, 0x00);
    cpu.memory.write(0xFF23, 0xBF);
    cpu.memory.write(0xFF24, 0x77);
    cpu.memory.write(0xFF25, 0xF3);
    cpu.memory.write(0xFF26, 0xF1); // (DMG) 0xF1, (SGB) 0xF0
    cpu.memory.write(0xFF40, 0x91); // LCDC ON
    cpu.memory.write(0xFF42, 0x00); // SCY
    cpu.memory.write(0xFF43, 0x00); // SCX
    cpu.memory.write(0xFF45, 0x00); // LYC
    cpu.memory.write(0xFF47, 0xFC); // BGP
    cpu.memory.write(0xFF48, 0xFF); // OBP0
    cpu.memory.write(0xFF49, 0xFF); // OBP1
    cpu.memory.write(0xFF4A, 0x00); // WY
    cpu.memory.write(0xFF4B, 0x00); // WX
    cpu.memory.write(0xFFFF, 0x00); // IE
}

fn main() {
    // 初始化 CPU 與載入 ROM（可選）
    let mut cpu = GB::CPU::CPU::new();
    let (rom_arg, speed_factor, user_volume) = parse_cli();
    // 先看命令列參數是否有指定 ROM 路徑；否則落回候選清單
    let _rom_loaded = try_load_rom(&mut cpu, rom_arg);

    // 跳過 BIOS 的常見初始化（Pan Docs post-BIOS defaults）
    init_post_bios(&mut cpu);

    // 建立 SDL 視窗
    let scale = 3u32;
    let mut display = SdlDisplay::new("Rust GB", scale).expect("SDL init failed");
    // 可選：之後可改成重用 Texture；目前先使用內建 blit

    // 音訊：建立簡易 APU 合成器並啟動播放
    let synth = Arc::new(Mutex::new(SimpleAPUSynth::default()));
    if let Some(v) = user_volume {
        if let Ok(mut s) = synth.lock() {
            s.master_gain = v;
        }
        // println!("Volume: {:.2} (可用 --volume 調整)", v);
    }
    // 讓 Bus 能更新音色
    cpu.memory.attach_synth(synth.clone());
    let audio = AudioInterface::new_from_sdl(&display._sdl, synth.clone())
        .map_err(|e| {
            eprintln!("Audio init error: {}", e);
        })
        .ok();
    if let Some(ref a) = audio {
        let _ = a.start();
    }

    // 以 PPU VBlank 作為畫面同步點，避免撕裂
    let mut quit = false;
    let mut frame_counter: u64 = 0;
    let mut instr_counter: u64 = 0; // 心跳：每 50 萬指令印狀態
    let mut prev_in_vblank = false;
    // 可用 --speed <倍率> 微調速度（預設 1.0）
    // 速度/FPS 量測
    let mut perf_last = Instant::now();
    let mut perf_last_cycles = cpu.cycles;
    let mut perf_last_frames = frame_counter;
    // let mut lcdc80_frames: u64 = 0; // 停用：不再自動協助修改 LCDC
    // 熱鍵暫停：不追蹤任何 F-key 邊緣狀態與測試覆蓋
    // 節流設定：以 VBlank 為幀界，目標 ~59.73 FPS（DMG）
    let target_frame = Duration::from_secs_f64((1.0 / DMG_FPS) / speed_factor);
    let mut last_vblank = Instant::now();
    // 週期為基準的節流（LCD 關閉或未達 VBlank 時的備援）
    let mut rt_anchor = Instant::now();
    let mut cycles_anchor = cpu.cycles;

    // 重用一個 shades 緩衝區，避免每幀配置
    let mut shades_buf = vec![0u8; SCREEN_W * SCREEN_H];
    while !quit {
        // 以指令為粒度執行
        cpu.execute_next();
        instr_counter = instr_counter.wrapping_add(1);

        // 若一直沒有進入 VBlank，也定期輸出 LCDC/LY 與 VRAM/TileMap 的非零統計，便於除錯
        if DEBUG_HEARTBEAT && instr_counter % 500_000 == 0 {
            let mut _vram_nonzero = 0usize;
            for addr in 0x8000u16..=0x9FFFu16 {
                if cpu.memory.read(addr) != 0 {
                    _vram_nonzero += 1;
                }
            }
            let mut _bgmap_nonzero = 0usize;
            for addr in 0x9800u16..=0x9BFFu16 {
                if cpu.memory.read(addr) != 0 {
                    _bgmap_nonzero += 1;
                }
            }
            let _lcdc = cpu.memory.read(0xFF40);
            let _ly = cpu.memory.read(0xFF44);
            let _pc = cpu.registers.get_pc();
            let _opcode = cpu.memory.read(_pc);
            let _ie = cpu.memory.read(0xFFFF);
            let _iflag = cpu.memory.read(0xFF0F);
            let _ime = cpu.ime;
            let _halted = cpu.halted;
            println!(
                "Heartbeat ({}k instr) | PC={:04X} OP={:02X} | LY={} | LCDC={:02X} | IE={:02X} IF={:02X} IME={} HALT={} | VRAM!=0: {} / 8192 | BGMap!=0: {} / 1024",
                instr_counter / 1000,
                _pc,
                _opcode,
                _ly,
                _lcdc,
                _ie,
                _iflag,
                _ime,
                _halted,
                _vram_nonzero,
                _bgmap_nonzero
            );
        }

        // 輸入更新（joypad），並檢查是否退出
        quit = display.pump_events_and_update_joypad(|dpad, btns| {
            cpu.memory.set_joypad_rows(dpad, btns);
        });

        // 按鍵測試：偵測邊緣事件並印出
        if DEBUG_FRAME_INFO {
            for sc in [
                Scancode::Right,
                Scancode::Left,
                Scancode::Up,
                Scancode::Down,
                Scancode::Z,
                Scancode::X,
                Scancode::RShift,
                Scancode::Return,
            ] {
                let _ = display.take_keydown(sc);
            }
        }

        // 熱鍵功能已停用：不再改動 LCDC/VRAM 或注入任何測試畫面

        // 在 VBlank 上緣執行 blit，避免撕裂與閃爍
        let in_vblank = cpu.memory.read(0xFF44) >= 144;
        if in_vblank && !prev_in_vblank {
            frame_counter += 1;
            // 直接取用 framebuffer 並複製到 shades_buf
            let fb = cpu.memory.framebuffer();
            shades_buf.copy_from_slice(fb);
            // 不再在 LCD 關閉時強制清白，保留上一幀內容以符合多數真機觀感
            let _ = display.blit_framebuffer(&shades_buf);

            // 每 30 幀偵測一次 VRAM/Tilemap 是否已有內容，方便除錯
            if DEBUG_FRAME_INFO && frame_counter % 30 == 0 {
                let mut _vram_nonzero = 0usize;
                for addr in 0x8000u16..=0x9FFFu16 {
                    if cpu.memory.read(addr) != 0 {
                        _vram_nonzero += 1;
                    }
                }
                let mut _bgmap_nonzero = 0usize;
                for addr in 0x9800u16..=0x9BFFu16 {
                    if cpu.memory.read(addr) != 0 {
                        _bgmap_nonzero += 1;
                    }
                }
                // 額外：列印按鍵狀態摘要（每 30 幀一次）
                let keyboard = display.event_pump.keyboard_state();
                let keys = [
                    ("R", Scancode::Right),
                    ("L", Scancode::Left),
                    ("U", Scancode::Up),
                    ("D", Scancode::Down),
                    ("A", Scancode::Z),
                    ("B", Scancode::X),
                    ("Sel", Scancode::RShift),
                    ("Start", Scancode::Return),
                ];
                let mut pressed: Vec<&str> = Vec::new();
                for (label, sc) in keys.iter() {
                    if keyboard.is_scancode_pressed(*sc) {
                        pressed.push(label);
                    }
                }
                println!(
                    "Frame {} | VRAM!=0: {} / 8192 | BGMap!=0: {} / 1024 | LCDC={:02X} | Keys: {}",
                    frame_counter,
                    _vram_nonzero,
                    _bgmap_nonzero,
                    cpu.memory.read(0xFF40),
                    if pressed.is_empty() { "(none)".to_string() } else { pressed.join(",") }
                );
            }
            // 停用自動協助：不會自動更改 LCDC（完全交給 ROM 控制）

            // 每秒列印一次效能資訊：倍速、FPS、每秒 cycles
            let elapsed = perf_last.elapsed();
            if elapsed.as_secs_f64() >= 1.0 {
                let dc = cpu.cycles.saturating_sub(perf_last_cycles);
                let df = frame_counter.saturating_sub(perf_last_frames);
                let secs = elapsed.as_secs_f64();
                let cps = (dc as f64) / secs;
                let speed_x = cps / DMG_CPU_HZ;
                let fps = (df as f64) / secs;
                if DEBUG_FRAME_INFO {
                    println!("Perf: {:.2}x | {:.1} FPS | {:.0} cycles/s", speed_x, fps, cps);
                }
                perf_last = Instant::now();
                perf_last_cycles = cpu.cycles;
                perf_last_frames = frame_counter;
            }

            // 精準節流到 ~59.73 FPS（依 VBlank 節點）
            let since = last_vblank.elapsed();
            if since < target_frame {
                std::thread::sleep(target_frame - since);
            }
            last_vblank = Instant::now();
            // 每幀重設實時間錨點，避免累積誤差
            rt_anchor = last_vblank;
            cycles_anchor = cpu.cycles;
        }
        // LCD 關閉或未進入 VBlank 的情況：依 CPU 週期節流，維持實時間 1x 速度
        if !in_vblank {
            let elapsed = rt_anchor.elapsed();
            let cycles_since = cpu.cycles.saturating_sub(cycles_anchor);
            // 1 個 CPU 週期 ≈ 1 / (DMG_CPU_HZ * speed_factor) 秒
            let expected =
                Duration::from_secs_f64((cycles_since as f64) / (DMG_CPU_HZ * speed_factor));
            if expected > elapsed {
                let sleep_dur = expected - elapsed;
                // 避免超短 sleep 造成忙迴圈，僅在 > 200 微秒時睡
                if sleep_dur > Duration::from_micros(200) {
                    std::thread::sleep(sleep_dur);
                }
            }
            // 週期性重設錨點以降低誤差
            if elapsed > Duration::from_millis(50) {
                rt_anchor = Instant::now();
                cycles_anchor = cpu.cycles;
            }
        }
        prev_in_vblank = in_vblank;
    }
}
