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

/// Configuration for input processing
#[derive(Debug, Clone)]
pub struct InputConfig {
    /// Maximum number of events in the queue before overflow
    pub max_queue_size: usize,
    /// Timeout for non-blocking event polling (in milliseconds)
    #[allow(dead_code)]
    pub poll_timeout_ms: u32,
    /// Whether to enable low-latency SDL3 settings
    #[allow(dead_code)]
    pub low_latency_mode: bool,
    /// Game-specific input mapping adjustments
    pub game_specific_mapping: Option<String>,
    /// Configurable timing parameters for different games
    pub timing_config: TimingConfig,
    /// Custom key mappings for different input schemes
    pub key_mappings: KeyMappings,
}

/// Timing configuration for different game requirements
#[derive(Debug, Clone)]
pub struct TimingConfig {
    /// Frame rate target (Hz)
    #[allow(dead_code)]
    pub target_fps: f64,
    /// Maximum allowed input latency (milliseconds)
    #[allow(dead_code)]
    pub max_input_latency_ms: u32,
    /// Poll interval for event checking (microseconds)
    #[allow(dead_code)]
    pub poll_interval_us: u64,
    /// Whether to use adaptive timing based on game behavior
    pub adaptive_timing: bool,
}

/// Custom key mappings for different input schemes
#[derive(Debug, Clone)]
pub struct KeyMappings {
    /// Mapping from SDL scancodes to GameBoy keys
    pub scancode_to_key: std::collections::HashMap<sdl3::keyboard::Scancode, JoypadKey>,
    /// Alternative key mappings for different layouts
    pub alternative_mappings: std::collections::HashMap<
        String,
        std::collections::HashMap<sdl3::keyboard::Scancode, JoypadKey>,
    >,
}

impl Default for TimingConfig {
    fn default() -> Self {
        Self {
            target_fps: 59.7275,      // GameBoy frame rate
            max_input_latency_ms: 16, // One frame at 60 FPS
            poll_interval_us: 1000,   // 1ms poll interval
            adaptive_timing: true,
        }
    }
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
            poll_timeout_ms: 1, // 1ms timeout for non-blocking
            low_latency_mode: true,
            game_specific_mapping: None,
            timing_config: TimingConfig::default(),
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
    /// Create a new InputManager with default configuration
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::with_config(InputConfig::default())
    }

    /// Create a new InputManager with custom configuration
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

    /// Apply timing configuration to SDL3 (called during initialization)
    pub fn apply_timing_config(&self) {
        // Note: SDL3 timing hints are set during initialization
        // This method could be extended to dynamically adjust timing if needed
        if self.config.timing_config.adaptive_timing {
            // Adaptive timing is enabled - could implement dynamic poll interval adjustment
        }
    }

    /// Get current queue status for monitoring
    #[allow(dead_code)]
    pub fn get_queue_status(&self) -> (usize, usize, u64) {
        (
            self.event_queue.len(),
            self.config.max_queue_size,
            self.overflow_count,
        )
    }

    /// Check if queue is near overflow
    #[allow(dead_code)]
    pub fn is_queue_near_overflow(&self) -> bool {
        self.event_queue.len() > self.config.max_queue_size * 3 / 4
    }

    /// Update configuration
    #[allow(dead_code)]
    pub fn update_config(&mut self, config: InputConfig) {
        self.config = config;
        // Resize queue if needed
        if self.event_queue.capacity() < self.config.max_queue_size {
            let mut new_queue = VecDeque::with_capacity(self.config.max_queue_size);
            // Move events to new queue
            while let Some(event) = self.event_queue.pop_front() {
                if new_queue.len() < self.config.max_queue_size {
                    new_queue.push_back(event);
                }
            }
            self.event_queue = new_queue;
        }
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

/// Configure SDL3 for low-latency input processing
fn configure_sdl3_low_latency() {
    // Enable VSync to prevent screen tearing
    sdl3::hint::set("SDL_RENDER_VSYNC", "1");

    // Configure low-latency input settings
    sdl3::hint::set("SDL_HINT_JOYSTICK_THREAD", "1"); // Use threaded joystick input
    sdl3::hint::set("SDL_HINT_GAMECONTROLLER_USE_BUTTON_LABELS", "0"); // Disable button labels for faster processing
    sdl3::hint::set("SDL_HINT_MOUSE_RELATIVE_MODE_WARP", "0"); // Disable mouse warping
    sdl3::hint::set("SDL_HINT_VIDEO_MINIMIZE_ON_FOCUS_LOSS", "0"); // Don't minimize on focus loss

    // Set timer resolution for more precise timing
    sdl3::hint::set("SDL_HINT_TIMER_RESOLUTION", "1"); // 1ms timer resolution

    // Configure event processing for low latency
    sdl3::hint::set("SDL_HINT_EVENT_LOGGING", "0"); // Disable event logging for performance
    sdl3::hint::set("SDL_HINT_WINDOWS_DISABLE_THREAD_NAMING", "1"); // Disable thread naming on Windows

    // Android-specific low latency settings (if applicable)
    sdl3::hint::set("SDL_HINT_ANDROID_BLOCK_ON_PAUSE", "0");
    sdl3::hint::set("SDL_HINT_ANDROID_TRAP_BACK_BUTTON", "0");
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
    let sdl_context = sdl3::init().unwrap();
    let video_subsystem = sdl_context.video().unwrap();
    let audio_subsystem = sdl_context.audio().unwrap();

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
    input_manager.apply_timing_config(); // Apply timing configuration

    let mut event_pump = sdl_context.event_pump().unwrap();

    // Game Boy 精確幀率: 59.7275 FPS
    let frame_duration = Duration::from_nanos(16_742_706);
    let half_frame_duration = frame_duration / 2; // 8.37ms for mid-frame polling

    // Performance monitoring for non-blocking game loop
    let mut frame_count = 0;
    let mut total_input_processing_time = Duration::ZERO;
    let mut max_input_processing_time = Duration::ZERO;
    let mut _input_processing_overruns = 0;
    let mut _frame_timing_overruns = 0;

    'running: loop {
        let frame_start = Instant::now();
        frame_count += 1;

        // Poll events at frame start (non-blocking)
        let input_poll_start = Instant::now();
        input_manager.poll_events(&mut event_pump);
        let _input_poll_time = input_poll_start.elapsed();

        // Process queued events and apply to joypad with timing monitoring
        let input_process_start = Instant::now();
        let input_actions = input_manager.process_events();
        let input_processing_time = input_process_start.elapsed();

        // Update performance monitoring
        total_input_processing_time += input_processing_time;
        max_input_processing_time = max_input_processing_time.max(input_processing_time);

        // Check for input processing overruns (exceeded 1ms budget)
        if input_processing_time > Duration::from_millis(1) {
            _input_processing_overruns += 1;
        }

        // Apply input actions to joypad
        for (key, pressed) in input_actions {
            if gb.joypad.set_key(key, pressed) && pressed {
                gb.mmu.if_reg |= 0x10; // Joypad interrupt
            }
        }

        // Check for quit event (simplified - InputManager handles most events)
        if input_manager.should_quit() || input_manager.escape_pressed() {
            gb.mmu.save_external_ram();

            // Print performance summary before exit
            let _avg_input_processing_time = if frame_count > 0 {
                total_input_processing_time / frame_count
            } else {
                Duration::ZERO
            };

            // Performance summary removed for cleaner output

            break 'running;
        }

        // Run first half of frame
        gb.run_cycles(35112); // Half of 70224 cycles

        // Mid-frame input polling for reduced latency (8.37ms intervals)
        let mid_frame_time = frame_start.elapsed();
        if mid_frame_time >= half_frame_duration {
            // Poll events mid-frame if we're running behind schedule
            input_manager.poll_events(&mut event_pump);
            let mid_frame_actions = input_manager.process_events();
            for (key, pressed) in mid_frame_actions {
                if gb.joypad.set_key(key, pressed) && pressed {
                    gb.mmu.if_reg |= 0x10; // Joypad interrupt
                }
            }
        }

        // Run second half of frame
        gb.run_cycles(35112); // Remaining cycles

        // 獲取音訊樣本
        let samples = gb.apu.drain_samples();
        for s in samples {
            let _ = tx.try_send(s);
        }

        let ppu_fb = gb.get_framebuffer();

        // --- SIMD-optimized expand indexed (0..3) GB pixels to RGBA8888 ---
        // Rust 1.93.0 SIMD improvements allow for better vectorization
        const PALETTE: [[u8; 4]; 4] = [
            [255, 255, 255, 255], // White
            [170, 170, 170, 255], // Light gray
            [85, 85, 85, 255],    // Dark gray
            [0, 0, 0, 255],       // Black
        ];

        // Process pixels in chunks for better SIMD utilization
        // Game Boy resolution: 160x144 = 23040 pixels
        const CHUNK_SIZE: usize = 8; // Process 8 pixels at a time for SIMD
        let chunks = ppu_fb.len() / CHUNK_SIZE;
        let _remainder = ppu_fb.len() % CHUNK_SIZE;

        // Process full chunks
        for chunk_idx in 0..chunks {
            let start_idx = chunk_idx * CHUNK_SIZE;
            let chunk = &ppu_fb[start_idx..start_idx + CHUNK_SIZE];

            for (i, &idx) in chunk.iter().enumerate() {
                let color = PALETTE[(idx & 0x03) as usize];
                let dst = (start_idx + i) * 4;
                rgba[dst..dst + 4].copy_from_slice(&color);
            }
        }

        // Process remaining pixels
        for (i, &idx) in ppu_fb.iter().enumerate().skip(chunks * CHUNK_SIZE) {
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

        // Handle frame timing for split-frame execution
        let frame_end = Instant::now();
        let actual_frame_time = frame_end.duration_since(frame_start);

        // Check for frame timing overruns (exceeded target frame time)
        if actual_frame_time > frame_duration {
            _frame_timing_overruns += 1;
            // Frame timing overrun warning removed for cleaner output
        }

        // Wait for target frame time if needed (non-blocking wait)
        if actual_frame_time < frame_duration {
            std::thread::sleep(frame_duration - actual_frame_time);
        }

        // Periodic performance reporting removed for cleaner output
    }
}
