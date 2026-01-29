use crate::interface::audio::{AudioInterface, SimpleAPUSynth};
use crate::interface::sdl3_display::SdlDisplay;
use std::sync::{Arc, Mutex};

/// 顯示和音訊管理器
pub struct DisplayManager {
    pub display: SdlDisplay,
    pub audio: Option<AudioInterface>,
    pub synth: Arc<Mutex<SimpleAPUSynth>>,
}

impl DisplayManager {
    pub fn new(title: &str, scale: u32, user_volume: Option<f32>) -> Result<Self, String> {
        // 建立 SDL 視窗
        let display = SdlDisplay::new(title, scale)?;

        // 音訊：建立簡易 APU 合成器並啟動播放
        let synth = Arc::new(Mutex::new(SimpleAPUSynth::default()));
        if let Some(v) = user_volume {
            if let Ok(mut s) = synth.lock() {
                s.master_gain = v;
            }
        }

        let audio = AudioInterface::new_from_sdl(&display._sdl, synth.clone())
            .map_err(|e| {
                eprintln!("Audio init error: {}", e);
            })
            .ok();
        if let Some(ref a) = audio {
            // Start audio playback
            if let Err(e) = a.start() {
                eprintln!("Audio start error: {}", e);
            }
        }

        Ok(Self {
            display,
            audio,
            synth,
        })
    }

    pub fn attach_synth(&self, cpu_memory: &mut crate::GB::BUS::Bus) {
        cpu_memory.attach_synth(self.synth.clone());
    }

    pub fn pump_events_and_update_joypad<F: FnMut(u8, u8)>(&mut self, set_p1: F) -> bool {
        let quit = self.display.pump_events_and_update_joypad(set_p1);
        // 更新音量
        self.update_volume();

        // 檢查 T 鍵播放測試音調
        if self.display.take_keydown(sdl3::keyboard::Scancode::T) {
            if let Some(ref audio) = self.audio {
                audio.play_test_tone(&self.synth);
            } else {
                println!("Audio interface not available");
            }
        }

        quit
    }

    pub fn blit_framebuffer(&mut self, shades: &[u8]) -> Result<(), String> {
        self.display.blit_framebuffer(shades)
    }

    pub fn tick_apu(&self) {
        // SimpleAPUSynth doesn't need ticking - audio is generated in real-time in the callback
    }

    pub fn update_volume(&self) {
        if let Ok(mut apu) = self.synth.lock() {
            apu.master_gain = self.display.volume;
            // 調試輸出
            static mut LAST_VOLUME: f32 = -1.0;
            unsafe {
                if (apu.master_gain - LAST_VOLUME).abs() > 0.01 {
                    println!("APU 音量設置為: {:.2}", apu.master_gain);
                    LAST_VOLUME = apu.master_gain;
                }
            }
        }
    }
}
