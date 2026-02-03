extern crate sdl3;

use crate::gameboy::GameBoy;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::rect::Rect;

use crossbeam::channel::Receiver;
use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};
use std::time::{Duration, Instant};

struct GbAudio {
    receiver: Receiver<f32>,
}

impl AudioCallback<f32> for GbAudio {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let mut samples = Vec::with_capacity(requested as usize);
        for _ in 0..requested {
            if let Ok(sample) = self.receiver.try_recv() {
                samples.push(sample);
            } else {
                samples.push(0.0);
            }
        }
        let _ = stream.put_data_f32(&samples);
    }
}

pub fn main(rom_path: String) {
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

    // 設置 SDL3 提示以強制啟用 VSync，解決畫面撕裂
    sdl3::hint::set("SDL_RENDER_VSYNC", "1");

    let (tx, rx) = crossbeam::channel::bounded(16384);

    let spec = AudioSpec {
        format: Some(AudioFormat::f32_sys()),
        channels: Some(1),
        freq: Some(44100),
    };

    let stream = audio_subsystem
        .open_playback_stream(&spec, GbAudio { receiver: rx })
        .unwrap();
    stream.resume().unwrap();

    let window = video_subsystem
        .window("GameBoy", 800, 600)
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let mut canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();
    let mut stream_tex = texture_creator
        .create_texture_streaming(PixelFormat::ABGR8888, 160, 144)
        .unwrap();

    // 預先分配 RGBA 緩衝區，避免每幀重複分配
    const W: u32 = 160;
    const H: u32 = 144;
    let mut rgba = vec![0u8; (W * H * 4) as usize];

    // emulator instance
    let mut gb = GameBoy::new();
    gb.load_rom(&rom_path).expect("Failed to load ROM");

    let mut event_pump = sdl_context.event_pump().unwrap();

    // Game Boy 精確幀率: 59.7275 FPS
    let frame_duration = Duration::from_nanos(16_742_706);

    'running: loop {
        let frame_start = Instant::now();

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                } => {
                    gb.mmu.save_external_ram();
                    break 'running;
                }
                Event::KeyDown {
                    scancode: Some(sc), ..
                } => {
                    if let Some(key) = map_scancode(sc) {
                        if gb.joypad.set_key(key, true) {
                            gb.mmu.if_reg |= 0x10;
                        }
                    }
                }
                Event::KeyUp {
                    scancode: Some(sc), ..
                } => {
                    if let Some(key) = map_scancode(sc) {
                        gb.joypad.set_key(key, false);
                    }
                }
                _ => {}
            }
        }

        gb.run_frame();

        // 獲取音訊樣本
        let samples = gb.apu.drain_samples();
        for s in samples {
            let _ = tx.try_send(s);
        }

        let ppu_fb = gb.get_framebuffer();

        // --- expand indexed (0..3) GB pixels to RGBA8888 ---
        const PALETTE: [[u8; 4]; 4] = [
            [255, 255, 255, 255], // White
            [170, 170, 170, 255], // Light gray
            [85, 85, 85, 255],    // Dark gray
            [0, 0, 0, 255],       // Black
        ];

        for (i, &idx) in ppu_fb.iter().enumerate() {
            let color = PALETTE[(idx & 0x03) as usize];
            let dst = i * 4;
            rgba[dst..dst + 4].copy_from_slice(&color);
        }

        // --- upload to streaming texture and draw ---
        stream_tex.update(None, &rgba, (W * 4) as usize).unwrap();

        let (win_w, win_h) = canvas.window().size();
        let scale = (win_w as f32 / W as f32)
            .min(win_h as f32 / H as f32)
            .floor()
            .max(1.0);
        let dest_w = (W as f32 * scale) as u32;
        let dest_h = (H as f32 * scale) as u32;
        let dst_x = (win_w - dest_w) / 2;
        let dst_y = (win_h - dest_h) / 2;
        let dest = Rect::new(dst_x as i32, dst_y as i32, dest_w, dest_h);

        canvas.copy(&stream_tex, None, dest).unwrap();
        canvas.present();

        let elapsed = frame_start.elapsed();
        if elapsed < frame_duration {
            std::thread::sleep(frame_duration - elapsed);
        }
    }
}

fn map_scancode(scancode: sdl3::keyboard::Scancode) -> Option<crate::joypad::JoypadKey> {
    use crate::joypad::JoypadKey;
    use sdl3::keyboard::Scancode;

    match scancode {
        Scancode::Up => Some(JoypadKey::Up),
        Scancode::Down => Some(JoypadKey::Down),
        Scancode::Left => Some(JoypadKey::Left),
        Scancode::Right => Some(JoypadKey::Right),
        Scancode::Z => Some(JoypadKey::A),
        Scancode::X => Some(JoypadKey::B),
        Scancode::Return | Scancode::Space => Some(JoypadKey::Start),
        Scancode::RShift => Some(JoypadKey::Select),
        _ => None,
    }
}
