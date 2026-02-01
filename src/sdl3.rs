extern crate sdl3;

use crate::gameboy::GameBoy;
use sdl3::event::Event;
use sdl3::gpu::{Device, ShaderFormat};
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::rect::Rect;

use crossbeam::channel::Receiver;
use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};
use std::time::Duration;

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

    let (tx, rx) = crossbeam::channel::bounded(8192);

    let spec = AudioSpec {
        format: Some(AudioFormat::f32_sys()),
        channels: Some(1),
        freq: Some(44100),
    };

    let stream = audio_subsystem
        .open_playback_stream(&spec, GbAudio { receiver: rx })
        .unwrap();
    stream.resume().unwrap();

    let gpu_subsystem = Device::new(ShaderFormat::SPIRV, false).unwrap();

    let window = video_subsystem
        .window("GameBoy", 800, 600)
        .position_centered()
        .build()
        .unwrap();

    // attach GPU subsystem to the window (still useful if you later use shaders)
    gpu_subsystem.with_window(&window).unwrap();

    // capture window size BEFORE consuming the window into a canvas
    let (win_w, win_h) = window.size();

    // CPU-side streaming texture (simple, reliable): update every frame from PPU RGBA buffer
    let mut canvas = window.into_canvas();
    let texture_creator = canvas.texture_creator();
    let mut stream_tex = texture_creator
        .create_texture_streaming(PixelFormat::ABGR8888, 160, 144)
        .unwrap();

    // emulator instance
    let mut gb = GameBoy::new();
    gb.load_rom(&rom_path).expect("Failed to load ROM");

    let mut event_pump = sdl_context.event_pump().unwrap();
    let mut i = 0;
    'running: loop {
        i = (i + 1) % 255;

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
                    keycode: Some(key), ..
                } => {
                    update_joypad(&mut gb, key, true);
                }
                Event::KeyUp {
                    keycode: Some(key), ..
                } => {
                    update_joypad(&mut gb, key, false);
                }
                _ => {}
            }
        }

        // --- advance emulator one frame and get PPU framebuffer (indexed 0..=3) ---
        gb.run_frame();

        // 獲取音訊樣本並發送到通道
        let samples = gb.apu.drain_samples();
        for s in samples {
            // 如果通道滿了就丟棄，避免阻塞主執行緒
            let _ = tx.try_send(s);
        }
        let ppu_fb = gb.get_framebuffer(); // &[u8] length 160*144

        // --- expand indexed (0..3) GB pixels to RGBA8888 ---
        // Game Boy classic 4-shade grayscale (0 = white, 3 = black)
        // dmg-acid2 requires: $FF, $AA, $55, $00
        const W: usize = 160;
        const H: usize = 144;
        let mut rgba = vec![0u8; W * H * 4];
        for (i, &idx) in ppu_fb.iter().enumerate() {
            let shade = match idx {
                0 => 0xFFu8, // White
                1 => 0xAAu8, // Light gray
                2 => 0x55u8, // Dark gray
                _ => 0x00u8, // Black
            };
            let dst = i * 4;
            rgba[dst] = shade;
            rgba[dst + 1] = shade;
            rgba[dst + 2] = shade;
            rgba[dst + 3] = 0xFF;
        }

        // --- upload to streaming texture and draw ---
        stream_tex.update(None, &rgba, (W * 4) as usize).unwrap();

        // compute destination rect (scale integer factor, center)
        let scale_x = win_w as f32 / W as f32;
        let scale_y = win_h as f32 / H as f32;
        let scale = scale_x.min(scale_y).floor().max(1.0) as u32;
        let dest_w = (W as u32 * scale) as u32;
        let dest_h = (H as u32 * scale) as u32;
        let dst_x = ((win_w as i32 - dest_w as i32) / 2).max(0);
        let dst_y = ((win_h as i32 - dest_h as i32) / 2).max(0);
        let dest = Rect::new(dst_x, dst_y, dest_w, dest_h);

        canvas.copy(&stream_tex, None, dest).unwrap();
        canvas.present();

        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / 60));
    }
}

fn update_joypad(gb: &mut GameBoy, key: Keycode, pressed: bool) {
    use crate::joypad::JoypadKey;

    let button = match key {
        Keycode::Up => Some(JoypadKey::Up),
        Keycode::Down => Some(JoypadKey::Down),
        Keycode::Left => Some(JoypadKey::Left),
        Keycode::Right => Some(JoypadKey::Right),
        Keycode::Z => Some(JoypadKey::A),
        Keycode::X => Some(JoypadKey::B),
        Keycode::Return => Some(JoypadKey::Start),
        Keycode::Backspace => Some(JoypadKey::Select),
        _ => None,
    };

    if let Some(b) = button {
        if gb.joypad.set_key(b, pressed) {
            gb.mmu.if_reg |= 0x10; // 觸發 Joypad 中斷
        }
    }
}
