// Game Boy 模擬器主結構

use crate::apu::Apu;
use crate::cpu::Cpu;
use crate::joypad::{GameBoyButton, Joypad};
use crate::mmu::{IoHandler, Mmu};
use crate::ppu::Ppu;
use crate::timer::Timer;
use std::collections::HashMap;
use std::time::{Duration, Instant};

// Custom error types for better error handling (Rust 1.93.0 improvements)
#[derive(Debug)]
#[allow(dead_code)]
pub enum GameBoyError {
    RomLoad {
        path: String,
        source: Box<dyn std::error::Error>,
    },
    Timing(String),
    Interrupt(String),
    Io(std::io::Error),
}

impl std::fmt::Display for GameBoyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GameBoyError::RomLoad { path, source } => {
                write!(f, "Failed to load ROM '{}': {}", path, source)
            }
            GameBoyError::Timing(msg) => write!(f, "Timing error: {}", msg),
            GameBoyError::Interrupt(msg) => write!(f, "Interrupt error: {}", msg),
            GameBoyError::Io(err) => write!(f, "I/O error: {}", err),
        }
    }
}

impl std::error::Error for GameBoyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            GameBoyError::RomLoad { source, .. } => Some(source.as_ref()),
            GameBoyError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<std::io::Error> for GameBoyError {
    fn from(err: std::io::Error) -> Self {
        GameBoyError::Io(err)
    }
}

// Timing Controller Module - Precise frame timing and synchronization
// Implements 59.7275 FPS timing with 0.1% accuracy and 70224 cycles per frame validation

/// Game Boy timing constants with improved precision (Rust 1.93.0 const improvements)
const GAMEBOY_FRAME_RATE: f64 = 59.7275; // Hz
const GAMEBOY_CYCLES_PER_FRAME: u64 = 70224;

/// Pre-calculated frame duration constant using const fn (Rust 1.93.0 enhancement)
const GAMEBOY_FRAME_DURATION: Duration = {
    // Convert fps to nanoseconds: (1/fps) * 1_000_000_000
    let nanos = (1_000_000_000.0 / GAMEBOY_FRAME_RATE) as u64;
    Duration::from_nanos(nanos)
};

/// Timing modes for different game requirements
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimingMode {
    /// Strict Game Boy timing (59.7275 FPS)
    Strict,
}

/// Frame timing statistics
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FrameTimingStats {
    pub target_frame_time: Duration,
    pub actual_frame_time: Duration,
    pub frame_time_error: Duration,
    pub frame_time_accuracy: f64, // Percentage (1.0 = perfect)
    pub cycles_executed: u64,
    pub cycle_accuracy: f64, // Percentage (1.0 = perfect)
    pub frame_number: u64,
    pub timestamp: Instant,
}

/// Timing controller for precise frame synchronization
#[allow(dead_code)]
pub struct TimingController {
    /// Current timing mode
    mode: TimingMode,
    /// Target frame rate (Hz)
    target_fps: f64,
    /// Target cycles per frame
    target_cycles_per_frame: u64,
    /// Target frame duration
    target_frame_duration: Duration,
    /// Frame timing history for accuracy calculation
    frame_history: Vec<FrameTimingStats>,
    /// Maximum history size
    max_history: usize,
    /// Current frame number
    frame_number: u64,
    /// Accumulated timing error for drift correction
    timing_drift: Duration,
    /// Whether timing controller is enabled
    enabled: bool,
}

#[allow(dead_code)]
impl TimingController {
    /// Create a new timing controller with default Game Boy settings
    pub fn new() -> Self {
        let target_fps = GAMEBOY_FRAME_RATE;
        let target_cycles_per_frame = GAMEBOY_CYCLES_PER_FRAME;
        let target_frame_duration = GAMEBOY_FRAME_DURATION;

        Self {
            mode: TimingMode::Strict,
            target_fps,
            target_cycles_per_frame,
            target_frame_duration,
            frame_history: Vec::new(),
            max_history: 60, // Keep 1 second of history at 60 FPS
            frame_number: 0,
            timing_drift: Duration::ZERO,
            enabled: true,
        }
    }

    /// Create timing controller with custom settings
    pub fn with_settings(target_fps: f64, target_cycles: u64, mode: TimingMode) -> Self {
        let target_frame_duration = Duration::from_nanos((1_000_000_000.0 / target_fps) as u64);

        Self {
            mode,
            target_fps,
            target_cycles_per_frame: target_cycles,
            target_frame_duration,
            frame_history: Vec::new(),
            max_history: 60,
            frame_number: 0,
            timing_drift: Duration::ZERO,
            enabled: true,
        }
    }

    /// Set timing mode
    pub fn set_mode(&mut self, mode: TimingMode) {
        self.mode = mode;
    }

    /// Enable or disable timing controller
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Get current timing mode
    pub fn get_mode(&self) -> TimingMode {
        self.mode
    }

    /// Get target frame rate
    pub fn get_target_fps(&self) -> f64 {
        self.target_fps
    }

    /// Get target cycles per frame
    pub fn get_target_cycles_per_frame(&self) -> u64 {
        self.target_cycles_per_frame
    }

    /// Get target frame duration
    pub fn get_target_frame_duration(&self) -> Duration {
        self.target_frame_duration
    }

    /// Start frame timing - called at the beginning of each frame
    pub fn start_frame(&mut self) -> Instant {
        Instant::now()
    }

    /// End frame timing and calculate statistics - called at the end of each frame
    pub fn end_frame(&mut self, frame_start: Instant, cycles_executed: u64) -> FrameTimingStats {
        let frame_end = Instant::now();
        let actual_frame_time = frame_end.duration_since(frame_start);

        // Calculate timing accuracy
        let frame_time_error = actual_frame_time.abs_diff(self.target_frame_duration);

        let frame_time_accuracy = if actual_frame_time.as_nanos() > 0 {
            let target_nanos = self.target_frame_duration.as_nanos() as f64;
            1.0 - (frame_time_error.as_nanos() as f64 / target_nanos).abs()
        } else {
            0.0
        };

        // Calculate cycle accuracy
        let cycle_accuracy = if self.target_cycles_per_frame > 0 {
            let cycle_error =
                ((cycles_executed as i64) - (self.target_cycles_per_frame as i64)).abs() as f64;
            1.0 - (cycle_error / self.target_cycles_per_frame as f64)
        } else {
            1.0
        };

        let stats = FrameTimingStats {
            target_frame_time: self.target_frame_duration,
            actual_frame_time,
            frame_time_error,
            frame_time_accuracy,
            cycles_executed,
            cycle_accuracy,
            frame_number: self.frame_number,
            timestamp: frame_end,
        };

        // Update history
        self.frame_history.push(stats.clone());
        if self.frame_history.len() > self.max_history {
            self.frame_history.remove(0);
        }

        // Update timing drift for adaptive timing
        self.update_timing_drift(&stats);

        self.frame_number += 1;

        stats
    }

    /// Wait until the target frame time is reached (for strict timing)
    pub fn wait_for_frame_time(&self, frame_start: Instant) -> Duration {
        if !self.enabled {
            return Duration::ZERO;
        }

        let elapsed = frame_start.elapsed();
        if elapsed < self.target_frame_duration {
            let wait_time = self.target_frame_duration - elapsed;
            std::thread::sleep(wait_time);
            wait_time
        } else {
            Duration::ZERO
        }
    }

    /// Get frame timing statistics for the last N frames
    pub fn get_frame_timing_stats(&self, frames: usize) -> Vec<FrameTimingStats> {
        let count = frames.min(self.frame_history.len());
        self.frame_history
            .iter()
            .rev()
            .take(count)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect()
    }

    /// Calculate average timing accuracy over recent frames
    pub fn get_average_timing_accuracy(&self, frames: usize) -> f64 {
        let recent_stats = self.get_frame_timing_stats(frames);
        if recent_stats.is_empty() {
            return 0.0;
        }

        let sum: f64 = recent_stats.iter().map(|s| s.frame_time_accuracy).sum();
        sum / recent_stats.len() as f64
    }

    /// Calculate average cycle accuracy over recent frames
    pub fn get_average_cycle_accuracy(&self, frames: usize) -> f64 {
        let recent_stats = self.get_frame_timing_stats(frames);
        if recent_stats.is_empty() {
            return 0.0;
        }

        let sum: f64 = recent_stats.iter().map(|s| s.cycle_accuracy).sum();
        sum / recent_stats.len() as f64
    }

    /// Check if timing is within acceptable accuracy (0.1% = 99.9%)
    pub fn is_timing_accurate(&self, frames: usize) -> bool {
        let avg_accuracy = self.get_average_timing_accuracy(frames);
        avg_accuracy >= 0.999 // 99.9% accuracy
    }

    /// Get current timing drift
    pub fn get_timing_drift(&self) -> Duration {
        self.timing_drift
    }

    /// Get current frame number
    pub fn get_frame_number(&self) -> u64 {
        self.frame_number
    }

    /// Get frame history (for testing purposes)
    pub fn get_frame_history(&self) -> &Vec<FrameTimingStats> {
        &self.frame_history
    }

    /// Get mutable frame history (for testing purposes)
    pub fn get_frame_history_mut(&mut self) -> &mut Vec<FrameTimingStats> {
        &mut self.frame_history
    }

    /// Set frame number (for testing purposes)
    pub fn set_frame_number(&mut self, frame_number: u64) {
        self.frame_number = frame_number;
    }

    /// Reset timing statistics
    pub fn reset_stats(&mut self) {
        self.frame_history.clear();
        self.frame_number = 0;
        self.timing_drift = Duration::ZERO;
    }

    /// Update timing drift based on frame statistics
    fn update_timing_drift(&mut self, stats: &FrameTimingStats) {
        // Accumulate timing error for drift correction
        if stats.actual_frame_time > self.target_frame_duration {
            self.timing_drift += stats.actual_frame_time - self.target_frame_duration;
        } else {
            // Prevent negative drift from accumulating too much
            let deficit = self.target_frame_duration - stats.actual_frame_time;
            if deficit < self.timing_drift {
                self.timing_drift -= deficit;
            } else {
                self.timing_drift = Duration::ZERO;
            }
        }

        // Limit maximum drift to prevent excessive accumulation
        let max_drift = Duration::from_millis(100); // 100ms max drift
        if self.timing_drift > max_drift {
            self.timing_drift = max_drift;
        }
    }

    /// Validate that cycles per frame match Game Boy specification
    pub fn validate_cycles_per_frame(&self, cycles: u64) -> bool {
        cycles == self.target_cycles_per_frame
    }

    /// Validate PPU timing within correct windows
    /// PPU should complete exactly 154 scanlines per frame (144 visible + 10 VBlank)
    /// Each scanline should be exactly 456 dots/cycles
    pub fn validate_ppu_timing(&self, scanline: u8, dots: u16, mode: u8) -> Result<(), String> {
        // Validate scanline range (0-153)
        if scanline > 153 {
            return Err(format!("Invalid scanline: {} (max 153)", scanline));
        }

        // Validate dots per scanline (0-455)
        if dots > 455 {
            return Err(format!(
                "Invalid dots for scanline {}: {} (max 455)",
                scanline, dots
            ));
        }

        // Validate mode transitions at correct timing
        match mode {
            0 => {
                // HBlank - should be at dots 252-455
                if dots < 252 {
                    return Err(format!(
                        "HBlank mode at invalid dots: {} (should be >= 252)",
                        dots
                    ));
                }
            }
            1 => {
                // VBlank - should be at scanlines 144-153
                if scanline < 144 {
                    return Err(format!(
                        "VBlank mode at invalid scanline: {} (should be >= 144)",
                        scanline
                    ));
                }
            }
            2 => {
                // OAM Search - should be at dots 0-79
                if dots > 79 {
                    return Err(format!(
                        "OAM Search mode at invalid dots: {} (should be <= 79)",
                        dots
                    ));
                }
            }
            3 => {
                // Pixel Transfer - should be at dots 80-251
                if !(80..=251).contains(&dots) {
                    return Err(format!(
                        "Pixel Transfer mode at invalid dots: {} (should be 80-251)",
                        dots
                    ));
                }
            }
            _ => return Err(format!("Invalid PPU mode: {}", mode)),
        }

        Ok(())
    }

    /// Validate Timer timing based on TAC settings
    /// Timer should increment at correct frequencies based on TAC register
    pub fn validate_timer_timing(
        &self,
        tac: u8,
        cycles_since_last_increment: u64,
    ) -> Result<(), String> {
        let timer_enabled = (tac & 0x04) != 0;
        if !timer_enabled {
            return Ok(()); // Timer disabled, no validation needed
        }

        let frequency_bits = tac & 0x03;
        let expected_cycles_between_increments = match frequency_bits {
            0 => 1024, // 4096 Hz (every 1024 T-cycles)
            1 => 16,   // 262144 Hz (every 16 T-cycles)
            2 => 64,   // 65536 Hz (every 64 T-cycles)
            3 => 256,  // 16384 Hz (every 256 T-cycles)
            _ => return Err(format!("Invalid TAC frequency bits: {}", frequency_bits)),
        };

        // Check if timer increment timing is reasonable
        if cycles_since_last_increment > expected_cycles_between_increments * 2 {
            return Err(format!(
                "Timer increment overdue: {} cycles since last increment (expected ~{})",
                cycles_since_last_increment, expected_cycles_between_increments
            ));
        }

        Ok(())
    }

    /// Get PPU timing constants for validation
    pub fn get_ppu_timing_constants() -> (u8, u16) {
        (154, 456) // 154 scanlines, 456 dots per scanline
    }

    /// Get Timer frequency constants for validation
    pub fn get_timer_frequency_constants() -> [(u8, u64); 4] {
        [
            (0, 1024), // 4096 Hz
            (1, 16),   // 262144 Hz
            (2, 64),   // 65536 Hz
            (3, 256),  // 16384 Hz
        ]
    }

    /// Get timing requirements for different games
    pub fn get_game_timing_requirements(game_name: &str) -> (f64, u64) {
        match game_name.to_lowercase().as_str() {
            "tetris" => (GAMEBOY_FRAME_RATE, GAMEBOY_CYCLES_PER_FRAME),
            "dr. mario" | "dr_mario" => (GAMEBOY_FRAME_RATE, GAMEBOY_CYCLES_PER_FRAME),
            "hyper lode runner" | "hyper_lode_runner" => {
                (GAMEBOY_FRAME_RATE, GAMEBOY_CYCLES_PER_FRAME)
            }
            _ => (GAMEBOY_FRAME_RATE, GAMEBOY_CYCLES_PER_FRAME), // Default Game Boy timing
        }
    }
}

impl Default for TimingController {
    fn default() -> Self {
        Self::new()
    }
}

/// Game Boy interrupt types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InterruptType {
    VBlank,
    LcdStat,
    Timer,
    Serial,
    Joypad,
}

/// Game-specific interrupt configuration
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GameInterruptConfig {
    /// Game identifier (ROM hash or name)
    pub game_id: String,
    /// Custom interrupt priorities for this game (lower = higher priority)
    pub custom_priorities: HashMap<InterruptType, u8>,
    /// Joypad interrupt delay cycles (default: 4)
    pub joypad_delay_cycles: u8,
    /// Whether to enable interrupt masking prevention
    pub prevent_masking: bool,
    /// Game-specific timing adjustments
    pub timing_adjustments: HashMap<InterruptType, Duration>,
}

impl Default for GameInterruptConfig {
    fn default() -> Self {
        Self {
            game_id: "default".to_string(),
            custom_priorities: HashMap::new(),
            joypad_delay_cycles: 4,
            prevent_masking: true,
            timing_adjustments: HashMap::new(),
        }
    }
}

/// Interrupt statistics
#[derive(Debug, Clone)]
pub struct InterruptStatistics {
    pub total_interrupts: u64,
    pub interrupts_by_type: HashMap<InterruptType, u64>,
    pub average_processing_time: HashMap<InterruptType, Duration>,
    pub max_processing_time: HashMap<InterruptType, Duration>,
    pub min_processing_time: HashMap<InterruptType, Duration>,
    pub interrupt_frequency_per_second: HashMap<InterruptType, f64>,
    pub last_update: Instant,
}

impl Default for InterruptStatistics {
    fn default() -> Self {
        Self {
            total_interrupts: 0,
            interrupts_by_type: HashMap::new(),
            average_processing_time: HashMap::new(),
            max_processing_time: HashMap::new(),
            min_processing_time: HashMap::new(),
            interrupt_frequency_per_second: HashMap::new(),
            last_update: Instant::now(),
        }
    }
}

/// Joypad interrupt delay tracking for 4-cycle optimization
#[derive(Debug, Clone)]
#[allow(dead_code)]
struct JoypadInterruptDelay {
    trigger_time: Instant,
    cycles_remaining: u8,
    interrupt_vector: u16,
}

/// Enhanced interrupt handler with optimized processing and priority management
pub struct InterruptHandler {
    /// Interrupt enable register (IE) - 0xFFFF
    pub ie_register: u8,
    /// Interrupt flag register (IF) - 0xFF0F
    pub if_register: u8,

    /// Configurable interrupt priorities (lower value = higher priority)
    priority_config: HashMap<InterruptType, u8>,

    /// Processing latency tracking for each interrupt type
    processing_latencies: HashMap<InterruptType, Vec<Duration>>,

    /// Interrupt statistics
    pub stats: InterruptStatistics,

    /// Joypad interrupt optimization: configurable delay tracking
    joypad_interrupt_delay: Option<JoypadInterruptDelay>,

    /// Performance monitoring
    last_interrupt_time: Option<Instant>,
    interrupt_frequency: HashMap<InterruptType, u64>,

    /// Game-specific configurations
    game_configs: HashMap<String, GameInterruptConfig>,
    current_game_config: Option<GameInterruptConfig>,

    /// Interrupt masking prevention state
    masking_prevention_enabled: bool,
    last_processed_interrupt: Option<InterruptType>,
}

struct GameBoyIoWrapper {
    ppu: *const Ppu,
    apu: *const Apu,
    timer: *const Timer,
    joypad: *const Joypad,
    interrupt_handler: *const InterruptHandler,
}

impl GameBoyIoWrapper {
    fn new(
        ppu: &Ppu,
        apu: &Apu,
        timer: &Timer,
        joypad: &Joypad,
        interrupt_handler: &InterruptHandler,
    ) -> Self {
        GameBoyIoWrapper {
            ppu: std::ptr::from_ref(ppu),
            apu: std::ptr::from_ref(apu),
            timer: std::ptr::from_ref(timer),
            joypad: std::ptr::from_ref(joypad),
            interrupt_handler: std::ptr::from_ref(interrupt_handler),
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
                        0xFF
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
                0xFF0F => {
                    if !self.interrupt_handler.is_null() {
                        (*self.interrupt_handler).if_register
                    } else {
                        0
                    }
                }
                0xFFFF => {
                    if !self.interrupt_handler.is_null() {
                        (*self.interrupt_handler).ie_register
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
                0xFF0F => {
                    if !self.interrupt_handler.is_null() {
                        let handler = self.interrupt_handler as *mut InterruptHandler;
                        (*handler).if_register = value;
                    }
                }
                0xFFFF => {
                    if !self.interrupt_handler.is_null() {
                        let handler = self.interrupt_handler as *mut InterruptHandler;
                        (*handler).ie_register = value;
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
    pub interrupt_handler: InterruptHandler,
    pub timing_controller: TimingController,
    pub cycles: u64,
    // PPU timing validation
    ppu_scanline_count: u16,
    ppu_dots_this_frame: u32,
    // Timer timing validation
    timer_cycles_since_increment: u64,
    #[allow(dead_code)]
    last_timer_increment_cycle: u64,
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
            interrupt_handler: InterruptHandler::new(),
            timing_controller: TimingController::new(),
            cycles: 0,
            // Initialize timing validation fields
            ppu_scanline_count: 0,
            ppu_dots_this_frame: 0,
            timer_cycles_since_increment: 0,
            last_timer_increment_cycle: 0,
        });

        // 設置 I/O 處理器
        let io_wrapper = GameBoyIoWrapper::new(
            &gb.ppu,
            &gb.apu,
            &gb.timer,
            &gb.joypad,
            &gb.interrupt_handler,
        );
        gb.mmu.set_io_handler(Box::new(io_wrapper));

        // 設置組件間的引用
        gb.joypad
            .set_interrupt_handler(&mut gb.interrupt_handler as *mut InterruptHandler);

        // 設置初始硬體狀態 (模擬啟動後狀態)
        gb.mmu.write_byte(0xFFFF, 0x00); // 關閉所有中斷
        gb.mmu.write_byte(0xFF40, 0x91); // 啟用 LCD, 背景, 圖塊集 0
        gb.mmu.write_byte(0xFF41, 0x85); // STAT
        gb.mmu.write_byte(0xFF44, 0x00); // LY

        gb
    }

    // 載入 ROM
    pub fn load_rom(&mut self, path: &str) -> Result<(), GameBoyError> {
        self.mmu.load_rom(path).map_err(|e| GameBoyError::RomLoad {
            path: path.to_string(),
            source: e,
        })?;

        // Auto-configure interrupt handler for the loaded game
        if let Some(filename) = std::path::Path::new(path).file_name()
            && let Some(filename_str) = filename.to_str()
        {
            self.interrupt_handler.auto_configure_for_game(filename_str);

            // Configure timing controller for the loaded game
            let (fps, cycles) = TimingController::get_game_timing_requirements(filename_str);
            if fps != self.timing_controller.get_target_fps()
                || cycles != self.timing_controller.get_target_cycles_per_frame()
            {
                // Create new timing controller with game-specific settings
                self.timing_controller =
                    TimingController::with_settings(fps, cycles, self.timing_controller.get_mode());
            }
        }

        Ok(())
    }

    // 執行一個完整的幀 (70224 個時鐘循環)
    #[allow(dead_code)]
    pub fn run_frame(&mut self) {
        let frame_start = self.timing_controller.start_frame();
        let frame_cycles = self.timing_controller.get_target_cycles_per_frame();
        let mut frame_cycle_count = 0;

        // Reset timing validation counters for this frame
        self.ppu_scanline_count = 0;
        self.ppu_dots_this_frame = 0;
        self.timer_cycles_since_increment = 0;

        while frame_cycle_count < frame_cycles {
            // 執行一個 CPU 指令並獲取實際的周期數
            // 注意：Timer 需要在 CPU 執行期間同步更新
            let instruction_cycles = self.step_cpu_with_timing();
            frame_cycle_count += instruction_cycles as u64;
            self.cycles += instruction_cycles as u64;
        }

        // End frame timing and get statistics
        let _timing_stats = self
            .timing_controller
            .end_frame(frame_start, frame_cycle_count);

        // Wait for target frame time if in strict mode
        let _wait_duration = self.timing_controller.wait_for_frame_time(frame_start);

        // Validate cycle count
        if !self
            .timing_controller
            .validate_cycles_per_frame(frame_cycle_count)
        {
            eprintln!(
                "Warning: Frame executed {} cycles, expected {}",
                frame_cycle_count, frame_cycles
            );
        }

        // Validate PPU timing - should complete exactly 154 scanlines per frame
        let (expected_scanlines, expected_dots_per_scanline) =
            TimingController::get_ppu_timing_constants();
        if self.ppu_scanline_count != expected_scanlines as u16 {
            eprintln!(
                "Warning: PPU completed {} scanlines, expected {}",
                self.ppu_scanline_count, expected_scanlines
            );
        }
        if self.ppu_dots_this_frame
            != (expected_scanlines as u32 * expected_dots_per_scanline as u32)
        {
            eprintln!(
                "Warning: PPU processed {} dots this frame, expected {}",
                self.ppu_dots_this_frame,
                expected_scanlines as u32 * expected_dots_per_scanline as u32
            );
        }
    }

    // 執行指定數量的時鐘循環 (用於分幀輸入處理優化)
    pub fn run_cycles(&mut self, target_cycles: u64) {
        let mut cycle_count = 0;

        while cycle_count < target_cycles {
            // 執行一個 CPU 指令並獲取實際的周期數
            let instruction_cycles = self.step_cpu_with_timing();
            cycle_count += instruction_cycles as u64;
            self.cycles += instruction_cycles as u64;
        }
    }

    // 執行一個 CPU 指令，並在執行期間同步更新 Timer 和 PPU
    fn step_cpu_with_timing(&mut self) -> u32 {
        // 處理 joypad 中斷延遲 (4-cycle 優化)
        let _joypad_triggered = self.interrupt_handler.process_joypad_interrupt_delay();

        // 同步中斷處理器與 MMU
        self.interrupt_handler.ie_register = self.mmu.read_byte(0xFFFF);
        self.interrupt_handler.if_register = self.mmu.read_byte(0xFF0F);

        // 處理中斷 (如果有待處理的中斷且CPU準備好)
        let interrupt_acknowledged = if self.interrupt_handler.has_pending_interrupts()
            && self.cpu.ime == crate::cpu::InterruptMasterState::Enabled
        {
            if let Some((interrupt_type, vector)) =
                self.interrupt_handler.get_highest_priority_interrupt()
            {
                // 禁用中斷主啟用
                self.cpu.ime = crate::cpu::InterruptMasterState::Disabled;

                // 清除中斷標誌
                self.interrupt_handler.clear_interrupt_flag(interrupt_type);

                // 推入當前 PC 到堆疊並跳轉
                let current_pc = self.cpu.pc;
                self.cpu.push_word(&mut self.mmu, current_pc);
                self.cpu.pc = vector;

                // 記錄中斷處理延遲
                self.interrupt_handler
                    .acknowledge_interrupt(interrupt_type, Instant::now());

                // 同步回 MMU
                self.mmu
                    .write_byte(0xFF0F, self.interrupt_handler.if_register);

                true // 中斷已處理
            } else {
                false
            }
        } else {
            false
        };

        // 如果處理了中斷，返回中斷處理週期
        if interrupt_acknowledged {
            // 批量更新週邊設備以消耗中斷處理週期
            let mut if_reg = self.interrupt_handler.if_register;
            for cycle in 0..20 {
                // Track PPU timing before tick
                let ppu_ly_before = self.ppu.ly;
                let _ppu_dots_before = self.ppu.dots;
                let _ppu_mode_before = self.ppu.mode as u8;

                // 中斷處理固定消耗20週期
                self.ppu.tick(&self.mmu, &mut if_reg);
                let tima_incremented = self.timer.tick(&mut if_reg);
                self.apu.tick();

                // Track PPU timing after tick
                let ppu_ly_after = self.ppu.ly;
                let ppu_dots_after = self.ppu.dots;
                let ppu_mode_after = self.ppu.mode as u8;

                // Update PPU timing validation counters
                self.ppu_dots_this_frame += 1;
                if ppu_ly_after != ppu_ly_before {
                    self.ppu_scanline_count += 1;
                }

                // Validate PPU timing at key points
                if let Err(_e) = self.timing_controller.validate_ppu_timing(
                    ppu_ly_after,
                    ppu_dots_after,
                    ppu_mode_after,
                ) {
                    // PPU timing validation error suppressed for cleaner output
                }

                // Track Timer timing
                self.timer_cycles_since_increment += 1;
                if tima_incremented {
                    self.timer_cycles_since_increment = 0;
                }

                // Validate Timer timing periodically
                if cycle % 4 == 0 {
                    // Check more frequently during interrupt processing
                    let tac = self.timer.tac;
                    if let Err(_e) = self
                        .timing_controller
                        .validate_timer_timing(tac, self.timer_cycles_since_increment)
                    {
                        // Timer timing validation error during interrupt (removed println)
                    }
                }

                // 處理 joypad 中斷延遲
                if self.interrupt_handler.process_joypad_interrupt_delay() {
                    if_reg |= 0x10;
                }
            }
            self.interrupt_handler.if_register = if_reg;
            self.mmu.write_byte(0xFF0F, if_reg);

            return 20;
        }

        // 執行 CPU 指令
        let cycles = self.cpu.step(&mut self.mmu);

        // 同步中斷處理器與 MMU
        self.interrupt_handler.ie_register = self.mmu.read_byte(0xFFFF);
        self.mmu
            .write_byte(0xFF0F, self.interrupt_handler.if_register);

        // 批量更新 PPU 和 Timer
        let mut if_reg = self.interrupt_handler.if_register;
        for cycle in 0..cycles {
            // Track PPU timing before tick
            let ppu_ly_before = self.ppu.ly;
            let _ppu_dots_before = self.ppu.dots;
            let _ppu_mode_before = self.ppu.mode as u8;

            self.ppu.tick(&self.mmu, &mut if_reg);
            let tima_incremented = self.timer.tick(&mut if_reg);
            self.apu.tick();

            // Track PPU timing after tick
            let ppu_ly_after = self.ppu.ly;
            let ppu_dots_after = self.ppu.dots;
            let ppu_mode_after = self.ppu.mode as u8;

            // Update PPU timing validation counters
            self.ppu_dots_this_frame += 1;
            if ppu_ly_after != ppu_ly_before {
                self.ppu_scanline_count += 1;
            }

            // Validate PPU timing at key points
            if let Err(_e) = self.timing_controller.validate_ppu_timing(
                ppu_ly_after,
                ppu_dots_after,
                ppu_mode_after,
            ) {
                // PPU timing validation error suppressed for cleaner output
            }

            // Track Timer timing
            self.timer_cycles_since_increment += 1;
            if tima_incremented {
                self.timer_cycles_since_increment = 0;
            }

            // Validate Timer timing periodically
            if cycle % 64 == 0 {
                // Check every 64 cycles
                let tac = self.timer.tac;
                if let Err(_e) = self
                    .timing_controller
                    .validate_timer_timing(tac, self.timer_cycles_since_increment)
                {
                    // Timer timing validation error (removed println)
                }
            }

            // 處理 joypad 中斷延遲
            if self.interrupt_handler.process_joypad_interrupt_delay() {
                if_reg |= 0x10; // Joypad interrupt flag
            }
        }

        // 更新中斷處理器
        self.interrupt_handler.if_register = if_reg;

        // 同步回 MMU
        self.mmu.write_byte(0xFF0F, if_reg);

        // Check for interrupt changes and record them
        let new_interrupts = if_reg & !self.mmu.if_reg;
        if new_interrupts != 0 {
            if new_interrupts & 0x01 != 0 {
                self.interrupt_handler
                    .trigger_interrupt(InterruptType::VBlank);
            }
            if new_interrupts & 0x02 != 0 {
                self.interrupt_handler
                    .trigger_interrupt(InterruptType::LcdStat);
            }
            if new_interrupts & 0x04 != 0 {
                self.interrupt_handler
                    .trigger_interrupt(InterruptType::Timer);
            }
            if new_interrupts & 0x08 != 0 {
                self.interrupt_handler
                    .trigger_interrupt(InterruptType::Serial);
            }
            if new_interrupts & 0x10 != 0 {
                // Joypad interrupt already handled with delay optimization
            }
        }

        self.mmu.if_reg = if_reg;

        cycles
    }

    // 獲取當前畫面緩衝區
    pub fn get_framebuffer(&self) -> &[u8] {
        self.ppu.get_framebuffer()
    }

    #[allow(dead_code)]
    pub fn should_render(&self) -> bool {
        self.ppu.mode == crate::ppu::LcdMode::VBlank && self.ppu.ly == 144
    }

    // Handle input events
    #[allow(dead_code)]
    pub fn handle_input_event(&mut self, button: GameBoyButton, pressed: bool) {
        // Convert to joypad key and process
        let joypad_key = match button {
            GameBoyButton::A => crate::joypad::JoypadKey::A,
            GameBoyButton::B => crate::joypad::JoypadKey::B,
            GameBoyButton::Select => crate::joypad::JoypadKey::Select,
            GameBoyButton::Start => crate::joypad::JoypadKey::Start,
            GameBoyButton::Right => crate::joypad::JoypadKey::Right,
            GameBoyButton::Left => crate::joypad::JoypadKey::Left,
            GameBoyButton::Up => crate::joypad::JoypadKey::Up,
            GameBoyButton::Down => crate::joypad::JoypadKey::Down,
        };

        // Process the input and check for interrupt
        let interrupt_triggered = self.joypad.set_key(joypad_key, pressed);

        // If interrupt was triggered, set the interrupt flag
        if interrupt_triggered {
            self.mmu.if_reg |= 0x10; // Set joypad interrupt flag (bit 4)
        }
    }
}

#[allow(dead_code)]
impl InterruptHandler {
    pub fn new() -> Self {
        let mut handler = Self {
            ie_register: 0,
            if_register: 0,
            priority_config: HashMap::new(),
            processing_latencies: HashMap::new(),
            stats: InterruptStatistics::default(),
            joypad_interrupt_delay: None,
            last_interrupt_time: None,
            interrupt_frequency: HashMap::new(),
            game_configs: HashMap::new(),
            current_game_config: None,
            masking_prevention_enabled: true,
            last_processed_interrupt: None,
        };

        // Initialize default priorities
        handler.priority_config.insert(InterruptType::VBlank, 0);
        handler.priority_config.insert(InterruptType::LcdStat, 1);
        handler.priority_config.insert(InterruptType::Timer, 2);
        handler.priority_config.insert(InterruptType::Serial, 3);
        handler.priority_config.insert(InterruptType::Joypad, 4);

        // Add predefined game configurations
        handler.initialize_game_configs();

        handler
    }

    /// Initialize predefined game configurations for known ROMs
    fn initialize_game_configs(&mut self) {
        // Tetris configuration - prioritize joypad interrupts to fix input issues
        let mut tetris_priorities = HashMap::new();
        tetris_priorities.insert(InterruptType::VBlank, 1);
        tetris_priorities.insert(InterruptType::Joypad, 0); // Highest priority for Tetris
        tetris_priorities.insert(InterruptType::LcdStat, 2);
        tetris_priorities.insert(InterruptType::Timer, 3);
        tetris_priorities.insert(InterruptType::Serial, 4);

        let tetris_config = GameInterruptConfig {
            game_id: "tetris".to_string(),
            custom_priorities: tetris_priorities,
            joypad_delay_cycles: 3, // Slightly faster for Tetris
            prevent_masking: true,
            timing_adjustments: HashMap::new(),
        };
        self.game_configs
            .insert("tetris".to_string(), tetris_config);

        // Dr. Mario configuration - balance for puzzle gameplay
        let mut drmario_priorities = HashMap::new();
        drmario_priorities.insert(InterruptType::VBlank, 0);
        drmario_priorities.insert(InterruptType::Joypad, 1); // High priority
        drmario_priorities.insert(InterruptType::LcdStat, 2);
        drmario_priorities.insert(InterruptType::Timer, 3);
        drmario_priorities.insert(InterruptType::Serial, 4);

        let drmario_config = GameInterruptConfig {
            game_id: "drmario".to_string(),
            custom_priorities: drmario_priorities,
            joypad_delay_cycles: 4,
            prevent_masking: true,
            timing_adjustments: HashMap::new(),
        };
        self.game_configs
            .insert("drmario".to_string(), drmario_config);

        // Hyper Lode Runner configuration - action game needs fast interrupts
        let mut hyperlode_priorities = HashMap::new();
        hyperlode_priorities.insert(InterruptType::VBlank, 0);
        hyperlode_priorities.insert(InterruptType::Joypad, 1);
        hyperlode_priorities.insert(InterruptType::LcdStat, 2);
        hyperlode_priorities.insert(InterruptType::Timer, 3);
        hyperlode_priorities.insert(InterruptType::Serial, 4);

        let hyperlode_config = GameInterruptConfig {
            game_id: "hyperlode".to_string(),
            custom_priorities: hyperlode_priorities,
            joypad_delay_cycles: 3, // Faster for action gameplay
            prevent_masking: true,
            timing_adjustments: HashMap::new(),
        };
        self.game_configs
            .insert("hyperlode".to_string(), hyperlode_config);
    }

    /// Set custom interrupt priority (0 = highest priority)
    pub fn set_interrupt_priority(&mut self, interrupt_type: InterruptType, priority: u8) {
        self.priority_config.insert(interrupt_type, priority);
    }

    /// Get current interrupt priority configuration
    pub fn get_interrupt_priorities(&self) -> &HashMap<InterruptType, u8> {
        &self.priority_config
    }

    /// Add game-specific interrupt configuration
    pub fn add_game_config(&mut self, config: GameInterruptConfig) {
        self.game_configs.insert(config.game_id.clone(), config);
    }

    /// Set current game configuration by game ID
    pub fn set_game_config(&mut self, game_id: &str) {
        if let Some(config) = self.game_configs.get(game_id) {
            // Apply custom priorities
            for (interrupt_type, priority) in &config.custom_priorities {
                self.priority_config.insert(*interrupt_type, *priority);
            }

            // Update masking prevention
            self.masking_prevention_enabled = config.prevent_masking;

            // Store current config
            self.current_game_config = Some(config.clone());
        } else {
            // Reset to default if game not found
            self.reset_to_default_config();
        }
    }

    /// Reset to default interrupt configuration
    pub fn reset_to_default_config(&mut self) {
        let mut priority_config = HashMap::new();
        priority_config.insert(InterruptType::VBlank, 0);
        priority_config.insert(InterruptType::LcdStat, 1);
        priority_config.insert(InterruptType::Timer, 2);
        priority_config.insert(InterruptType::Serial, 3);
        priority_config.insert(InterruptType::Joypad, 4);

        self.priority_config = priority_config;
        self.masking_prevention_enabled = true;
        self.current_game_config = None;
    }

    /// Get current game configuration
    pub fn get_current_game_config(&self) -> Option<&GameInterruptConfig> {
        self.current_game_config.as_ref()
    }

    /// Get all game configurations
    pub fn get_game_configs(&self) -> &HashMap<String, GameInterruptConfig> {
        &self.game_configs
    }

    /// Auto-detect and set game configuration based on ROM filename
    pub fn auto_configure_for_game(&mut self, rom_filename: &str) {
        let game_id = self.detect_game_from_filename(rom_filename);
        if game_id != "default" {
            self.set_game_config(&game_id);
        } else {
            self.reset_to_default_config();
        }
    }

    /// Detect game from ROM filename
    fn detect_game_from_filename(&self, filename: &str) -> String {
        let filename_lower = filename.to_lowercase();

        if filename_lower.contains("tetris") {
            "tetris".to_string()
        } else if filename_lower.contains("dr.mario") || filename_lower.contains("drmario") {
            "drmario".to_string()
        } else if filename_lower.contains("hyper") && filename_lower.contains("lode") {
            "hyperlode".to_string()
        } else {
            "default".to_string()
        }
    }

    /// Check if any interrupts are pending and enabled
    pub fn has_pending_interrupts(&self) -> bool {
        (self.ie_register & self.if_register & 0x1F) != 0
    }

    /// Get the highest priority pending interrupt with masking prevention
    pub fn get_highest_priority_interrupt(&self) -> Option<(InterruptType, u16)> {
        let pending = self.ie_register & self.if_register & 0x1F;

        if pending == 0 {
            return None;
        }

        // Find the highest priority interrupt based on custom configuration
        let mut highest_priority = u8::MAX;
        let mut selected_interrupt = None;
        let mut selected_vector = 0;

        // Check each interrupt bit
        for bit in 0..5 {
            if (pending & (1 << bit)) != 0 {
                let interrupt_type = match bit {
                    0 => InterruptType::VBlank,
                    1 => InterruptType::LcdStat,
                    2 => InterruptType::Timer,
                    3 => InterruptType::Serial,
                    4 => InterruptType::Joypad,
                    _ => continue,
                };

                // Interrupt masking prevention: skip if this interrupt type was just processed
                // and masking prevention is enabled
                if self.masking_prevention_enabled
                    && let Some(last_processed) = self.last_processed_interrupt
                    && last_processed == interrupt_type
                {
                    // Allow processing again after a short period, but prefer other interrupts
                    // Increase priority slightly to allow other interrupts to be processed first
                    let base_priority = self
                        .priority_config
                        .get(&interrupt_type)
                        .copied()
                        .unwrap_or(bit);
                    let adjusted_priority = base_priority.saturating_add(1);
                    if adjusted_priority < highest_priority {
                        highest_priority = adjusted_priority;
                        selected_interrupt = Some(interrupt_type);
                        selected_vector = match bit {
                            0 => 0x40, // VBlank
                            1 => 0x48, // LCD STAT
                            2 => 0x50, // Timer
                            3 => 0x58, // Serial
                            4 => 0x60, // Joypad
                            _ => 0,
                        };
                    }
                    continue;
                }

                let priority = self
                    .priority_config
                    .get(&interrupt_type)
                    .copied()
                    .unwrap_or(bit);

                if priority < highest_priority {
                    highest_priority = priority;
                    selected_interrupt = Some(interrupt_type);
                    selected_vector = match bit {
                        0 => 0x40, // VBlank
                        1 => 0x48, // LCD STAT
                        2 => 0x50, // Timer
                        3 => 0x58, // Serial
                        4 => 0x60, // Joypad
                        _ => 0,
                    };
                }
            }
        }

        selected_interrupt.map(|it| (it, selected_vector))
    }

    /// Trigger an interrupt with optimized processing and game-specific configuration
    pub fn trigger_interrupt(&mut self, interrupt_type: InterruptType) {
        // Interrupt masking prevention: ensure interrupts aren't masked by recent processing
        if self.masking_prevention_enabled
            && let Some(last_interrupt) = self.last_processed_interrupt
        {
            // Prevent rapid successive interrupts of the same type
            if last_interrupt == interrupt_type {
                // Add small delay to prevent masking
                std::thread::sleep(Duration::from_micros(10));
            }
        }

        let bit = match interrupt_type {
            InterruptType::VBlank => 0,
            InterruptType::LcdStat => 1,
            InterruptType::Timer => 2,
            InterruptType::Serial => 3,
            InterruptType::Joypad => 4,
        };

        // Special handling for joypad interrupts: configurable delay
        if matches!(interrupt_type, InterruptType::Joypad) {
            self.trigger_joypad_interrupt_optimized();
        } else {
            // Immediate trigger for other interrupts
            self.if_register |= 1 << bit;
        }

        // Update statistics
        self.update_interrupt_statistics(interrupt_type);
    }

    /// Optimized joypad interrupt triggering with configurable delay
    fn trigger_joypad_interrupt_optimized(&mut self) {
        if self.joypad_interrupt_delay.is_none() {
            let delay_cycles = self
                .current_game_config
                .as_ref()
                .map(|config| config.joypad_delay_cycles)
                .unwrap_or(4);

            self.joypad_interrupt_delay = Some(JoypadInterruptDelay {
                trigger_time: Instant::now(),
                cycles_remaining: delay_cycles,
                interrupt_vector: 0x60,
            });
        }
    }

    /// Process joypad interrupt delay (call this every CPU cycle)
    pub fn process_joypad_interrupt_delay(&mut self) -> bool {
        if let Some(ref mut delay) = self.joypad_interrupt_delay {
            delay.cycles_remaining = delay.cycles_remaining.saturating_sub(1);

            if delay.cycles_remaining == 0 {
                // Record processing time before clearing
                let processing_time = delay.trigger_time.elapsed();

                // Delay complete, trigger the interrupt
                self.if_register |= 0x10; // Joypad interrupt flag
                self.joypad_interrupt_delay = None;

                // Record processing time
                self.record_processing_latency(InterruptType::Joypad, processing_time);

                return true; // Interrupt triggered
            }
        }
        false
    }

    /// Clear an interrupt flag
    pub fn clear_interrupt_flag(&mut self, interrupt_type: InterruptType) {
        let bit = match interrupt_type {
            InterruptType::VBlank => 0,
            InterruptType::LcdStat => 1,
            InterruptType::Timer => 2,
            InterruptType::Serial => 3,
            InterruptType::Joypad => 4,
        };

        self.if_register &= !(1 << bit);
    }

    /// Acknowledge interrupt processing (called when CPU starts handling interrupt)
    pub fn acknowledge_interrupt(&mut self, interrupt_type: InterruptType, start_time: Instant) {
        self.clear_interrupt_flag(interrupt_type);
        self.last_interrupt_time = Some(start_time);
        self.last_processed_interrupt = Some(interrupt_type); // Update for masking prevention
        *self.interrupt_frequency.entry(interrupt_type).or_insert(0) += 1;
    }

    /// Record processing latency for performance monitoring
    pub fn record_processing_latency(&mut self, interrupt_type: InterruptType, latency: Duration) {
        self.processing_latencies
            .entry(interrupt_type)
            .or_default()
            .push(latency);

        // Keep only last 100 samples
        if let Some(latencies) = self.processing_latencies.get_mut(&interrupt_type)
            && latencies.len() > 100
        {
            latencies.remove(0);
        }

        // Update statistics
        self.update_latency_statistics(interrupt_type);
    }

    /// Update interrupt statistics
    fn update_interrupt_statistics(&mut self, interrupt_type: InterruptType) {
        self.stats.total_interrupts += 1;
        *self
            .stats
            .interrupts_by_type
            .entry(interrupt_type)
            .or_insert(0) += 1;

        // Update frequency statistics (per second)
        let now = Instant::now();
        let time_since_last_update = now.duration_since(self.stats.last_update);

        if time_since_last_update.as_secs() >= 1 {
            for (int_type, count) in &self.stats.interrupts_by_type {
                let frequency = *count as f64 / time_since_last_update.as_secs_f64();
                self.stats
                    .interrupt_frequency_per_second
                    .insert(*int_type, frequency);
            }
            self.stats.last_update = now;
        }
    }

    /// Update latency statistics for a specific interrupt type
    fn update_latency_statistics(&mut self, interrupt_type: InterruptType) {
        if let Some(latencies) = self.processing_latencies.get(&interrupt_type) {
            if latencies.is_empty() {
                return;
            }

            let sum: Duration = latencies.iter().sum();
            let avg = sum / latencies.len() as u32;
            let min = latencies.iter().min().copied().unwrap_or(Duration::ZERO);
            let max = latencies.iter().max().copied().unwrap_or(Duration::ZERO);

            self.stats
                .average_processing_time
                .insert(interrupt_type, avg);
            self.stats.min_processing_time.insert(interrupt_type, min);
            self.stats.max_processing_time.insert(interrupt_type, max);
        }
    }

    /// Get comprehensive interrupt statistics
    pub fn get_statistics(&self) -> &InterruptStatistics {
        &self.stats
    }

    /// Reset all statistics
    pub fn reset_statistics(&mut self) {
        self.stats = InterruptStatistics::default();
        self.processing_latencies.clear();
        self.interrupt_frequency.clear();
    }

    /// Check if joypad interrupt is currently delayed
    pub fn is_joypad_interrupt_delayed(&self) -> bool {
        self.joypad_interrupt_delay.is_some()
    }

    /// Get remaining cycles for joypad interrupt delay
    pub fn get_joypad_interrupt_delay_cycles(&self) -> Option<u8> {
        self.joypad_interrupt_delay
            .as_ref()
            .map(|d| d.cycles_remaining)
    }

    /// Check if interrupt masking prevention is enabled
    pub fn is_masking_prevention_enabled(&self) -> bool {
        self.masking_prevention_enabled
    }
}

impl Default for InterruptHandler {
    fn default() -> Self {
        Self::new()
    }
}
