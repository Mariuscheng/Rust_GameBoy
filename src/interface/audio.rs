use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};
use std::sync::{Arc, Mutex};

// Minimal, non-cycle-accurate APU synth for audible output (channels 1 and 2)
#[derive(Debug, Clone)]
pub struct SimpleAPUSynth {
    pub master_enable: bool, // NR52 bit7
    pub master_gain: f32,    // additional master volume 0.0..=2.0 (software gain)
    // CH1 (square)
    pub ch1_enable: bool, // NR14 bit7
    pub ch1_freq_hz: f32,
    pub ch1_duty: u8,    // 0..3 -> 12.5/25/50/75%
    pub ch1_volume: f32, // 0..1
    // CH2 (square)
    pub ch2_enable: bool, // NR24 bit7
    pub ch2_freq_hz: f32,
    pub ch2_duty: u8,
    pub ch2_volume: f32,
    // runtime phases
    phase1: f32,
    phase2: f32,
}

impl Default for SimpleAPUSynth {
    fn default() -> Self {
        Self {
            master_enable: false,
            master_gain: 0.35,
            ch1_enable: false,
            ch1_freq_hz: 0.0,
            ch1_duty: 2,
            ch1_volume: 0.0,
            ch2_enable: false,
            ch2_freq_hz: 0.0,
            ch2_duty: 2,
            ch2_volume: 0.0,
            phase1: 0.0,
            phase2: 0.0,
        }
    }
}

impl SimpleAPUSynth {
    #[inline]
    fn duty_ratio(duty: u8) -> f32 {
        match duty & 0x03 {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            _ => 0.75,
        }
    }
    #[inline]
    pub fn trigger(&mut self) {
        self.phase1 = 0.0;
        self.phase2 = 0.0;
    }
}

struct EmuAudioCallback {
    sample_rate: f32,
    synth: Arc<Mutex<SimpleAPUSynth>>,
}

impl AudioCallback<f32> for EmuAudioCallback {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let mut out = Vec::<f32>::with_capacity(requested as usize);
        let mut guard = self.synth.lock().unwrap();
        let s = &mut *guard;
        let sr = self.sample_rate;
        for _ in 0..requested {
            let mut sample = 0.0f32;
            if s.master_enable && s.ch1_enable && s.ch1_freq_hz > 0.0 && s.ch1_volume > 0.0 {
                s.phase1 += s.ch1_freq_hz / sr;
                if s.phase1 >= 1.0 {
                    s.phase1 -= 1.0;
                }
                let duty1 = SimpleAPUSynth::duty_ratio(s.ch1_duty);
                let v1 = if s.phase1 < duty1 { 1.0 } else { -1.0 };
                sample += v1 * (s.ch1_volume * 0.6);
            }
            if s.master_enable && s.ch2_enable && s.ch2_freq_hz > 0.0 && s.ch2_volume > 0.0 {
                s.phase2 += s.ch2_freq_hz / sr;
                if s.phase2 >= 1.0 {
                    s.phase2 -= 1.0;
                }
                let duty2 = SimpleAPUSynth::duty_ratio(s.ch2_duty);
                let v2 = if s.phase2 < duty2 { 1.0 } else { -1.0 };
                sample += v2 * (s.ch2_volume * 0.6);
            }
            // Apply software master gain
            sample *= s.master_gain.max(0.0).min(2.0);
            if sample > 1.0 {
                sample = 1.0;
            }
            if sample < -1.0 {
                sample = -1.0;
            }
            out.push(sample);
        }
        let _ = stream.put_data_f32(&out);
    }
}

#[allow(dead_code)]
pub struct AudioInterface {
    device: sdl3::audio::AudioStreamWithCallback<EmuAudioCallback>,
}

#[allow(dead_code)]
impl AudioInterface {
    pub fn new_with_synth(synth: Arc<Mutex<SimpleAPUSynth>>) -> Result<Self, String> {
        let sdl_context = sdl3::init().map_err(|e| format!("SDL init error: {:?}", e))?;
        let audio_subsystem = sdl_context
            .audio()
            .map_err(|e| format!("SDL audio error: {:?}", e))?;
        let source_freq = 44100;
        let source_spec = AudioSpec {
            freq: Some(source_freq),
            channels: Some(1),
            format: Some(AudioFormat::f32_sys()),
        };
        let device = audio_subsystem
            .open_playback_stream(
                &source_spec,
                EmuAudioCallback {
                    sample_rate: source_freq as f32,
                    synth,
                },
            )
            .map_err(|e| format!("Audio stream error: {:?}", e))?;
        Ok(Self { device })
    }
    pub fn new_from_sdl(
        sdl: &sdl3::Sdl,
        synth: Arc<Mutex<SimpleAPUSynth>>,
    ) -> Result<Self, String> {
        let audio_subsystem = sdl
            .audio()
            .map_err(|e| format!("SDL audio error: {:?}", e))?;
        let source_freq = 44100;
        let source_spec = AudioSpec {
            freq: Some(source_freq),
            channels: Some(1),
            format: Some(AudioFormat::f32_sys()),
        };
        let device = audio_subsystem
            .open_playback_stream(
                &source_spec,
                EmuAudioCallback {
                    sample_rate: source_freq as f32,
                    synth,
                },
            )
            .map_err(|e| format!("Audio stream error: {:?}", e))?;
        Ok(Self { device })
    }
    pub fn start(&self) -> Result<(), String> {
        self.device
            .resume()
            .map_err(|e| format!("SDL resume error: {:?}", e))
    }
    pub fn stop(&self) -> Result<(), String> {
        self.device
            .pause()
            .map_err(|e| format!("SDL pause error: {:?}", e))
    }
    pub fn play_test_tone(&self) {}
}
