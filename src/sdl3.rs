extern crate sdl3;

use crate::gameboy::GameBoy;
use crate::joypad::JoypadKey;
use sdl3::event::Event;
use sdl3::keyboard::Keycode;
use sdl3::pixels::PixelFormat;
use sdl3::rect::Rect;

// Input Manager Module - Non-blocking SDL3 event processing and input management
// Provides event queue management, overflow handling

use sdl3::keyboard::Scancode;
use std::collections::VecDeque;
use std::default::Default;
use std::time::{Duration, Instant};

fn sleep_until(deadline: Instant) {
    // Windows 的 sleep 精度容易抖動：先睡到接近 deadline，最後用短暫自旋補齊
    const SPIN_THRESHOLD: Duration = Duration::from_millis(2);

    loop {
        let now = Instant::now();
        if now >= deadline {
            break;
        }

        let remaining = deadline - now;
        if remaining > SPIN_THRESHOLD {
            std::thread::sleep(remaining - SPIN_THRESHOLD);
        } else {
            std::hint::spin_loop();
        }
    }
}

/// 錯誤類型
#[derive(Debug)]
#[allow(dead_code)]
pub enum EmulatorError {
    SdlInit(String),
    VideoSubsystem(String),
    AudioSubsystem(String),
    AudioStream(String),
    WindowCreation(String),
    CanvasCreation(String),
    TextureCreation(String),
    RomLoad(String),
    EventPump(String),
    TextureUpdate(String),
    CanvasCopy(String),
    InvalidPath(String),
    OpcodesLoad(String),
}

impl std::fmt::Display for EmulatorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmulatorError::SdlInit(msg) => write!(f, "SDL 初始化失敗: {}", msg),
            EmulatorError::VideoSubsystem(msg) => write!(f, "視訊子系統初始化失敗: {}", msg),
            EmulatorError::AudioSubsystem(msg) => write!(f, "音訊子系統初始化失敗: {}", msg),
            EmulatorError::AudioStream(msg) => write!(f, "音訊串流創建失敗: {}", msg),
            EmulatorError::WindowCreation(msg) => write!(f, "視窗創建失敗: {}", msg),
            EmulatorError::CanvasCreation(msg) => write!(f, "畫布創建失敗: {}", msg),
            EmulatorError::TextureCreation(msg) => write!(f, "紋理創建失敗: {}", msg),
            EmulatorError::RomLoad(msg) => write!(f, "ROM 載入失敗: {}", msg),
            EmulatorError::EventPump(msg) => write!(f, "事件泵初始化失敗: {}", msg),
            EmulatorError::TextureUpdate(msg) => write!(f, "紋理更新失敗: {}", msg),
            EmulatorError::CanvasCopy(msg) => write!(f, "畫布複製失敗: {}", msg),
            EmulatorError::InvalidPath(msg) => write!(f, "無效路徑: {}", msg),
            EmulatorError::OpcodesLoad(msg) => write!(f, "操作碼載入失敗: {}", msg),
        }
    }
}

impl std::error::Error for EmulatorError {}

/// Configuration for input processing
#[derive(Debug, Clone)]
pub struct InputConfig {
    pub max_queue_size: usize,
    pub game_specific_mapping: Option<String>,
    pub key_mappings: KeyMappings,
}

/// Custom key mappings
#[derive(Debug, Clone)]
pub struct KeyMappings {
    pub scancode_to_key: std::collections::HashMap<sdl3::keyboard::Scancode, JoypadKey>,
    pub alternative_mappings: std::collections::HashMap<
        String,
        std::collections::HashMap<sdl3::keyboard::Scancode, JoypadKey>,
    >,
}

impl Default for KeyMappings {
    fn default() -> Self {
        Self {
            scancode_to_key: JoypadKey::get_keyboard_mapping(),
            alternative_mappings: std::collections::HashMap::new(),
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            max_queue_size: 1000,
            game_specific_mapping: None,
            key_mappings: KeyMappings::default(),
        }
    }
}

/// Represents a queued input event
#[derive(Debug, Clone)]
pub struct QueuedEvent {
    pub event: Event,
    pub timestamp: Instant,
    pub processed: bool,
}

/// Main input manager for handling SDL3 events
pub struct InputManager {
    event_queue: VecDeque<QueuedEvent>,
    config: InputConfig,
    last_poll_time: Instant,
    overflow_count: u64,
}

impl InputManager {
    pub fn with_config(config: InputConfig) -> Self {
        Self {
            event_queue: VecDeque::with_capacity(config.max_queue_size),
            config,
            last_poll_time: Instant::now(),
            overflow_count: 0,
        }
    }

    /// Poll events from SDL3 event pump in non-blocking mode
    /// Returns the number of events processed
    pub fn poll_events(&mut self, event_pump: &mut sdl3::EventPump) -> usize {
        let poll_start = Instant::now();
        let mut events_processed = 0;

        // Non-blocking poll with timeout (simulate with poll_event)
        // Note: SDL3 doesn't have poll_event_timeout, so we use poll_event
        // and rely on the caller to call this frequently enough
        while let Some(event) = event_pump.poll_event() {
            if self.event_queue.len() >= self.config.max_queue_size {
                // Handle overflow
                self.overflow_count += 1;
                // Note: Could add overflow event to diagnostics here
                // Remove oldest event to make room
                self.event_queue.pop_front();
            }

            let queued_event = QueuedEvent {
                event,
                timestamp: Instant::now(),
                processed: false,
            };

            self.event_queue.push_back(queued_event);
            events_processed += 1;

            // Record in diagnostics
            // Note: Could add input polling event to diagnostics here
        }

        self.last_poll_time = poll_start;
        events_processed
    }

    /// Process queued events and return input actions
    /// Returns a vector of (key, pressed) tuples
    /// Special handling for Start button to ensure immediate response at startup
    /// Ensures non-blocking processing with timeout protection
    pub fn process_events(&mut self) -> Vec<(JoypadKey, bool)> {
        let mut actions = Vec::new();
        let process_start = Instant::now();
        let max_processing_time = Duration::from_millis(1); // Max 1ms for input processing

        // Process events in queue with timeout protection
        let mut i = 0;
        while i < self.event_queue.len() {
            // Check if we've exceeded processing time budget
            if process_start.elapsed() > max_processing_time {
                eprintln!(
                    "Warning: Input processing exceeded time budget, deferring remaining events"
                );
                break;
            }

            let should_process = if let Some(queued_event) = self.event_queue.get(i) {
                !queued_event.processed
            } else {
                false
            };

            if should_process {
                let event_clone = self.event_queue[i].event.clone();
                let timestamp = self.event_queue[i].timestamp;
                let action = self.process_single_event(&event_clone, timestamp);

                if let Some((key, pressed)) = action {
                    // Special handling for Start button - ensure immediate response at startup
                    if key == JoypadKey::Start && pressed {
                        // For Start button presses, prioritize immediate processing
                        // This ensures startup responsiveness without queuing delays
                        actions.insert(0, (key, pressed)); // Insert at front for priority
                    } else {
                        actions.push((key, pressed));
                    }
                }

                self.event_queue[i].processed = true;
            }
            i += 1;
        }

        // Clean up processed events (keep some buffer for timing analysis)
        while self.event_queue.len() > self.config.max_queue_size / 2 {
            if let Some(front) = self.event_queue.front() {
                if front.processed {
                    self.event_queue.pop_front();
                } else {
                    break;
                }
            }
        }

        actions
    }

    /// Process a single SDL event and return input action if applicable
    fn process_single_event(
        &self,
        event: &Event,
        _timestamp: Instant,
    ) -> Option<(JoypadKey, bool)> {
        match event {
            Event::KeyDown {
                scancode: Some(sc), ..
            } => self.map_scancode(*sc).map(|key| (key, true)),
            Event::KeyUp {
                scancode: Some(sc), ..
            } => self.map_scancode(*sc).map(|key| (key, false)),
            _ => None,
        }
    }

    /// Map SDL scancode to JoypadKey with game-specific adjustments
    fn map_scancode(&self, scancode: Scancode) -> Option<JoypadKey> {
        // Apply game-specific mapping if configured
        if let Some(game) = &self.config.game_specific_mapping
            && let Some(alt_mapping) = self.config.key_mappings.alternative_mappings.get(game)
        {
            return alt_mapping.get(&scancode).copied();
        }

        // Use default mapping
        self.config
            .key_mappings
            .scancode_to_key
            .get(&scancode)
            .copied()
    }

    /// Check if the last event was a quit event
    pub fn should_quit(&self) -> bool {
        self.event_queue
            .back()
            .is_some_and(|e| matches!(e.event, Event::Quit { .. }))
    }

    /// Check if the last event was an escape key press
    pub fn escape_pressed(&self) -> bool {
        self.event_queue.back().is_some_and(|e| {
            matches!(
                e.event,
                Event::KeyDown {
                    keycode: Some(Keycode::Escape),
                    ..
                }
            )
        })
    }
}

use crossbeam::channel::Receiver;
use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};

/// Configure SDL3 for stable rendering
fn configure_sdl3_low_latency() {
    sdl3::hint::set("SDL_RENDER_VSYNC", "1"); // Enable VSync
    sdl3::hint::set("SDL_HINT_RENDER_SCALE_QUALITY", "0");
}

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
    let sdl_context = sdl3::init().expect("SDL 初始化失敗");
    let video_subsystem = sdl_context.video().expect("視訊子系統初始化失敗");
    let audio_subsystem = sdl_context.audio().expect("音訊子系統初始化失敗");

    // Configure SDL3 for low-latency input processing
    configure_sdl3_low_latency();

    let (tx, rx) = crossbeam::channel::bounded::<f32>(16384);

    let spec = AudioSpec {
        format: Some(AudioFormat::f32_sys()),
        channels: Some(1),
        freq: Some(44100),
    };

    let stream = audio_subsystem
        .open_playback_stream(&spec, GbAudio { receiver: rx })
        .expect("音訊串流創建失敗");
    stream.resume().expect("音訊串流恢復失敗");

    let window = video_subsystem
        .window("GameBoy", 800, 600)
        .position_centered()
        .resizable()
        .build()
        .expect("視窗創建失敗");

    let mut canvas = window.into_canvas();
    canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.present();

    let texture_creator = canvas.texture_creator();
    let mut stream_tex = texture_creator
        .create_texture_streaming(PixelFormat::ABGR8888, 160, 144)
        .expect("紋理創建失敗");

    // 預先分配 RGBA 緩衝區，避免每幀重複分配
    const W: u32 = 160;
    const H: u32 = 144;
    let mut rgba = vec![0u8; (W * H * 4) as usize];

    // emulator instance
    let mut gb = GameBoy::new();
    gb.load_rom(&rom_path).expect("ROM 載入失敗");

    // Create input manager
    let mut input_config = InputConfig {
        game_specific_mapping: Some("tetris".to_string()), // Configure for Tetris
        ..Default::default()
    };

    // Set up Tetris-specific key mappings
    let tetris_mapping = JoypadKey::get_keyboard_mapping();

    input_config
        .key_mappings
        .alternative_mappings
        .insert("tetris".to_string(), tetris_mapping);

    let mut input_manager = InputManager::with_config(input_config);

    let mut event_pump = sdl_context.event_pump().expect("事件泵初始化失敗");

    let frame_duration = Duration::from_micros(16743); // 59.7275 FPS = 16.743ms
    let mut next_frame = Instant::now();

    loop {
        // Poll events
        input_manager.poll_events(&mut event_pump);
        let input_actions = input_manager.process_events();

        for (key, pressed) in input_actions {
            if gb.joypad.set_key(key, pressed) && pressed {
                gb.mmu.if_reg |= 0x10;
            }
        }

        if input_manager.should_quit() || input_manager.escape_pressed() {
            gb.mmu.save_external_ram();
            return;
        }

        // Run emulation (sync to VBlank so we always present whole frames)
        gb.run_frame();

        // Audio
        let samples = gb.apu.drain_samples();
        for s in samples {
            let _ = tx.try_send(s);
        }

        // Render
        let ppu_fb = gb.get_present_framebuffer();
        const PALETTE: [[u8; 4]; 4] = [
            [255, 255, 255, 255],
            [170, 170, 170, 255],
            [85, 85, 85, 255],
            [0, 0, 0, 255],
        ];

        for (i, &idx) in ppu_fb.iter().enumerate() {
            let color = PALETTE[(idx & 0x03) as usize];
            let dst = i * 4;
            rgba[dst..dst + 4].copy_from_slice(&color);
        }

        stream_tex.update(None, &rgba, (W * 4) as usize).ok();

        canvas.clear();
        let (win_w, win_h) = canvas.window().size();
        let scale = (win_w as f32 / W as f32)
            .min(win_h as f32 / H as f32)
            .floor()
            .max(1.0);
        let dest_w = (W as f32 * scale) as u32;
        let dest_h = (H as f32 * scale) as u32;
        let dest = Rect::new(
            ((win_w - dest_w) / 2) as i32,
            ((win_h - dest_h) / 2) as i32,
            dest_w,
            dest_h,
        );
        canvas.copy(&stream_tex, None, dest).ok();
        canvas.present();

        // Frame pacing: 累加 deadline + sleep-then-spin，避免忽快忽慢
        next_frame += frame_duration;
        let now = Instant::now();
        if next_frame > now {
            sleep_until(next_frame);
        } else {
            // 落後時不要直接 next_frame = now（會導致節奏漂移/抖動）
            // 只把 next_frame 往前推到「剛好超過 now」即可
            while next_frame + frame_duration <= now {
                next_frame += frame_duration;
            }
        }
    }
}
