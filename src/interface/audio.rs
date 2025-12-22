use sdl3::audio::{AudioCallback, AudioFormat, AudioSpec, AudioStream};
use std::sync::{Arc, Mutex};

// Game Boy APU (Audio Processing Unit) implementation
// Channels: CH1 (Square), CH2 (Square), CH3 (Wave), CH4 (Noise)
// Features: Envelope, Length, Sweep (CH1), Frame Sequencer

#[derive(Debug, Clone)]
pub struct Apu {
    // Frame Sequencer
    frame_sequencer_timer: u16, // Counts down from 8192 (512Hz at 4.194304MHz)
    frame_sequencer_step: u8,   // 0-7, triggers events every 2 steps

    // Channel 1: Square with Sweep
    ch1_enabled: bool,
    ch1_frequency: u16,
    ch1_duty: u8,
    ch1_length_counter: u8,
    ch1_length_enabled: bool,
    ch1_envelope_volume: u8,
    ch1_envelope_period: u8,
    ch1_envelope_direction: bool, // true = increase, false = decrease
    ch1_envelope_timer: u8,
    ch1_sweep_period: u8,
    ch1_sweep_negate: bool,
    ch1_sweep_shift: u8,
    ch1_sweep_timer: u8,
    ch1_sweep_shadow_freq: u16,
    ch1_sweep_enabled: bool,
    ch1_dac_enabled: bool,

    // Channel 2: Square
    ch2_enabled: bool,
    ch2_frequency: u16,
    ch2_duty: u8,
    ch2_length_counter: u8,
    ch2_length_enabled: bool,
    ch2_envelope_volume: u8,
    ch2_envelope_period: u8,
    ch2_envelope_direction: bool,
    ch2_envelope_timer: u8,
    ch2_dac_enabled: bool,

    // Channel 3: Wave
    ch3_enabled: bool,
    ch3_dac_enabled: bool,
    ch3_frequency: u16,
    ch3_length_counter: u8,
    ch3_length_enabled: bool,
    ch3_output_level: u8,   // 0=mute, 1=100%, 2=50%, 3=25%
    ch3_wave_ram: [u8; 16], // 32 4-bit samples
    ch3_wave_position: usize,

    // Channel 4: Noise
    ch4_enabled: bool,
    ch4_dac_enabled: bool,
    ch4_length_counter: u8,
    ch4_length_enabled: bool,
    ch4_envelope_volume: u8,
    ch4_envelope_period: u8,
    ch4_envelope_direction: bool,
    ch4_envelope_timer: u8,
    ch4_clock_shift: u8,
    ch4_lfsr_width: bool, // true = 7-bit, false = 15-bit
    ch4_clock_divider: u8,
    ch4_lfsr: u16,

    // Master controls
    master_enabled: bool,
    left_volume: u8,
    right_volume: u8,
    ch1_left_enable: bool,
    ch1_right_enable: bool,
    ch2_left_enable: bool,
    ch2_right_enable: bool,
    ch3_left_enable: bool,
    ch3_right_enable: bool,
    ch4_left_enable: bool,
    ch4_right_enable: bool,

    // Sample generation
    ch1_phase: u8,
    ch2_phase: u8,
    ch3_phase: u16,
    ch4_phase: u16,
}

impl Apu {
    pub fn new() -> Self {
        Self {
            frame_sequencer_timer: 8192,
            frame_sequencer_step: 0,

            ch1_enabled: false,
            ch1_frequency: 0,
            ch1_duty: 2,
            ch1_length_counter: 0,
            ch1_length_enabled: false,
            ch1_envelope_volume: 0,
            ch1_envelope_period: 0,
            ch1_envelope_direction: false,
            ch1_envelope_timer: 0,
            ch1_sweep_period: 0,
            ch1_sweep_negate: false,
            ch1_sweep_shift: 0,
            ch1_sweep_timer: 0,
            ch1_sweep_shadow_freq: 0,
            ch1_sweep_enabled: false,
            ch1_dac_enabled: false,

            ch2_enabled: false,
            ch2_frequency: 0,
            ch2_duty: 2,
            ch2_length_counter: 0,
            ch2_length_enabled: false,
            ch2_envelope_volume: 0,
            ch2_envelope_period: 0,
            ch2_envelope_direction: false,
            ch2_envelope_timer: 0,
            ch2_dac_enabled: false,

            ch3_enabled: false,
            ch3_dac_enabled: false,
            ch3_frequency: 0,
            ch3_length_counter: 0,
            ch3_length_enabled: false,
            ch3_output_level: 0,
            ch3_wave_ram: [0; 16],
            ch3_wave_position: 0,

            ch4_enabled: false,
            ch4_dac_enabled: false,
            ch4_length_counter: 0,
            ch4_length_enabled: false,
            ch4_envelope_volume: 0,
            ch4_envelope_period: 0,
            ch4_envelope_direction: false,
            ch4_envelope_timer: 0,
            ch4_clock_shift: 0,
            ch4_lfsr_width: false,
            ch4_clock_divider: 0,
            ch4_lfsr: 0x7FFF,

            master_enabled: false,
            left_volume: 7,
            right_volume: 7,
            ch1_left_enable: true,
            ch1_right_enable: true,
            ch2_left_enable: true,
            ch2_right_enable: true,
            ch3_left_enable: true,
            ch3_right_enable: true,
            ch4_left_enable: true,
            ch4_right_enable: true,

            ch1_phase: 0,
            ch2_phase: 0,
            ch3_phase: 0,
            ch4_phase: 0,
        }
    }

    // Step the APU by the given number of cycles
    pub fn step(&mut self, cycles: u16) {
        // Update channel phases based on frequency
        self.update_phases(cycles);

        // Frame sequencer runs at 512Hz (every 8192 cycles at 4.194304MHz)
        self.frame_sequencer_timer = self.frame_sequencer_timer.saturating_sub(cycles as u16);
        if self.frame_sequencer_timer == 0 {
            self.frame_sequencer_timer = 8192;
            self.frame_sequencer_step = (self.frame_sequencer_step + 1) % 8;

            match self.frame_sequencer_step {
                0 | 4 => {
                    // Length counter clock (256Hz)
                    self.clock_length_counters();
                }
                2 | 6 => {
                    // Length counter clock + Sweep clock (256Hz)
                    self.clock_length_counters();
                    self.clock_sweep();
                }
                7 => {
                    // Envelope clock (64Hz)
                    self.clock_envelopes();
                }
                _ => {}
            }
        }
    }

    fn update_phases(&mut self, cycles: u16) {
        self.update_ch1_phase(cycles);
        self.update_ch2_phase(cycles);
        self.update_ch3_phase(cycles);
        self.update_ch4_phase(cycles);
    }

    fn update_ch1_phase(&mut self, cycles: u16) {
        if self.ch1_frequency < 2048 {
            let freq_hz = 131072.0 / (2048.0 - self.ch1_frequency as f32);
            let freq_hz = freq_hz.max(1.0);
            let cycles_per_sample = (4194304.0 / freq_hz / 8.0).max(8.0);
            self.ch1_phase =
                ((self.ch1_phase as u32 + cycles as u32) % cycles_per_sample as u32) as u8;
        }
    }

    fn update_ch2_phase(&mut self, cycles: u16) {
        if self.ch2_frequency < 2048 {
            let freq_hz = 131072.0 / (2048.0 - self.ch2_frequency as f32);
            let freq_hz = freq_hz.max(1.0);
            let cycles_per_sample = (4194304.0 / freq_hz / 8.0).max(8.0);
            self.ch2_phase =
                ((self.ch2_phase as u32 + cycles as u32) % cycles_per_sample as u32) as u8;
        }
    }

    fn update_ch3_phase(&mut self, cycles: u16) {
        if self.ch3_frequency < 2048 && self.ch3_enabled {
            // CH3 frequency calculation: 65536 / (2048 - frequency)
            // Each wave position lasts for (2048 - frequency) * 2 T-cycles
            let period = 2048 - self.ch3_frequency;
            let cycles_per_position = (period * 2) as u32;

            if cycles_per_position > 0 {
                self.ch3_phase += cycles as u16;
                while self.ch3_phase >= cycles_per_position as u16 {
                    self.ch3_phase -= cycles_per_position as u16;
                    self.ch3_wave_position = (self.ch3_wave_position + 1) % 32;
                }
            }
        }
    }

    fn update_ch4_phase(&mut self, cycles: u16) {
        if self.ch4_enabled {
            let divisor = match self.ch4_clock_divider {
                0 => 8,
                n => n * 16,
            };
            let freq_hz = 524288.0 / divisor as f32 / (2.0f32).powi(self.ch4_clock_shift as i32);
            let cycles_per_update = (4194304.0 / freq_hz).max(1.0) as u32;

            self.ch4_phase += cycles as u16;
            while self.ch4_phase >= cycles_per_update as u16 {
                self.ch4_phase -= cycles_per_update as u16;
                self.update_ch4_lfsr();
            }
        }
    }

    fn update_ch4_lfsr(&mut self) {
        let bit = self.ch4_lfsr & 1;
        let feedback = if self.ch4_lfsr_width {
            (bit ^ ((self.ch4_lfsr >> 6) & 1)) != 0
        } else {
            (bit ^ ((self.ch4_lfsr >> 1) & 1)) != 0
        };
        self.ch4_lfsr >>= 1;
        if feedback {
            self.ch4_lfsr |= 0x4000;
        }
        if self.ch4_lfsr_width {
            self.ch4_lfsr &= !0x40;
        }
    }

    // Generate a stereo sample pair
    pub fn generate_sample(&mut self) -> (f32, f32) {
        if !self.master_enabled {
            return (0.0, 0.0);
        }

        let ch1_sample = if self.ch1_enabled {
            self.generate_ch1_sample()
        } else {
            0
        };
        let ch2_sample = if self.ch2_enabled {
            self.generate_ch2_sample()
        } else {
            0
        };
        let ch3_sample = if self.ch3_enabled {
            self.generate_ch3_sample()
        } else {
            0
        };
        let ch4_sample = if self.ch4_enabled {
            self.generate_ch4_sample()
        } else {
            0
        };

        let left_mix = (if self.ch1_left_enable { ch1_sample } else { 0 }
            + if self.ch2_left_enable { ch2_sample } else { 0 }
            + if self.ch3_left_enable { ch3_sample } else { 0 }
            + if self.ch4_left_enable { ch4_sample } else { 0 }) as f32
            / 15.0;

        let right_mix = (if self.ch1_right_enable { ch1_sample } else { 0 }
            + if self.ch2_right_enable { ch2_sample } else { 0 }
            + if self.ch3_right_enable { ch3_sample } else { 0 }
            + if self.ch4_right_enable { ch4_sample } else { 0 }) as f32
            / 15.0;

        let left_vol = self.left_volume as f32 / 7.0;
        let right_vol = self.right_volume as f32 / 7.0;

        (left_mix * left_vol, right_mix * right_vol)
    }

    fn generate_square_sample(&mut self, duty: u8, phase: u8, envelope_volume: u8) -> i8 {
        let duty_patterns = [
            0b00000001, // 12.5%
            0b00000011, // 25%
            0b00001111, // 50%
            0b00111111, // 75%
        ];
        let pattern = duty_patterns[duty as usize % 4];
        let bit = (pattern >> (7 - phase)) & 1;
        if bit != 0 {
            envelope_volume as i8
        } else {
            0
        }
    }

    fn generate_ch1_sample(&mut self) -> i8 {
        self.generate_square_sample(self.ch1_duty, self.ch1_phase, self.ch1_envelope_volume)
    }

    fn generate_ch2_sample(&mut self) -> i8 {
        self.generate_square_sample(self.ch2_duty, self.ch2_phase, self.ch2_envelope_volume)
    }

    fn generate_ch3_sample(&mut self) -> i8 {
        let sample_index = self.ch3_wave_position / 2;
        let nibble_shift = if self.ch3_wave_position % 2 == 0 {
            4
        } else {
            0
        };
        let sample = (self.ch3_wave_ram[sample_index] >> nibble_shift) & 0xF;

        match self.ch3_output_level {
            0 => 0,
            1 => sample as i8,
            2 => (sample >> 1) as i8,
            3 => (sample >> 2) as i8,
            _ => 0,
        }
    }

    fn generate_ch4_sample(&mut self) -> i8 {
        let bit = self.ch4_lfsr & 1;
        if bit != 0 {
            self.ch4_envelope_volume as i8
        } else {
            0
        }
    }

    fn clock_length_counter(length_enabled: bool, length_counter: &mut u8, enabled: &mut bool) {
        if length_enabled && *length_counter > 0 {
            *length_counter -= 1;
            if *length_counter == 0 {
                *enabled = false;
            }
        }
    }

    fn clock_length_counters(&mut self) {
        Self::clock_length_counter(
            self.ch1_length_enabled,
            &mut self.ch1_length_counter,
            &mut self.ch1_enabled,
        );
        Self::clock_length_counter(
            self.ch2_length_enabled,
            &mut self.ch2_length_counter,
            &mut self.ch2_enabled,
        );
        Self::clock_length_counter(
            self.ch3_length_enabled,
            &mut self.ch3_length_counter,
            &mut self.ch3_enabled,
        );
        Self::clock_length_counter(
            self.ch4_length_enabled,
            &mut self.ch4_length_counter,
            &mut self.ch4_enabled,
        );
    }

    fn clock_sweep(&mut self) {
        if self.ch1_sweep_timer > 0 {
            self.ch1_sweep_timer -= 1;
        }
        if self.ch1_sweep_timer == 0 && self.ch1_sweep_enabled {
            self.ch1_sweep_timer = if self.ch1_sweep_period == 0 {
                8
            } else {
                self.ch1_sweep_period
            };

            if self.ch1_sweep_period > 0 {
                self.perform_sweep_calculation();
            }
        }
    }

    fn perform_sweep_calculation(&mut self) {
        let delta = self.ch1_sweep_shadow_freq >> self.ch1_sweep_shift;
        let new_freq = self.calculate_sweep_frequency(delta);

        if new_freq > 2047 {
            self.ch1_enabled = false;
        } else if self.ch1_sweep_shift > 0 {
            self.ch1_sweep_shadow_freq = new_freq;
            self.ch1_frequency = new_freq;
            // Check again after update
            self.check_sweep_overflow_after_update(new_freq);
        }
    }

    fn calculate_sweep_frequency(&self, delta: u16) -> u16 {
        if self.ch1_sweep_negate {
            self.ch1_sweep_shadow_freq.wrapping_sub(delta)
        } else {
            self.ch1_sweep_shadow_freq.wrapping_add(delta)
        }
    }

    fn check_sweep_overflow_after_update(&mut self, new_freq: u16) {
        let delta2 = new_freq >> self.ch1_sweep_shift;
        let new_freq2 = self.calculate_sweep_frequency(delta2);
        if new_freq2 > 2047 {
            self.ch1_enabled = false;
        }
    }

    fn clock_envelope(
        envelope_timer: &mut u8,
        envelope_volume: &mut u8,
        envelope_period: u8,
        envelope_direction: bool,
    ) {
        if envelope_period > 0 {
            if *envelope_timer > 0 {
                *envelope_timer -= 1;
            }
            if *envelope_timer == 0 {
                *envelope_timer = envelope_period;
                if envelope_direction && *envelope_volume < 15 {
                    *envelope_volume += 1;
                } else if !envelope_direction && *envelope_volume > 0 {
                    *envelope_volume -= 1;
                }
            }
        }
    }

    fn clock_envelopes(&mut self) {
        Self::clock_envelope(
            &mut self.ch1_envelope_timer,
            &mut self.ch1_envelope_volume,
            self.ch1_envelope_period,
            self.ch1_envelope_direction,
        );
        Self::clock_envelope(
            &mut self.ch2_envelope_timer,
            &mut self.ch2_envelope_volume,
            self.ch2_envelope_period,
            self.ch2_envelope_direction,
        );
        Self::clock_envelope(
            &mut self.ch4_envelope_timer,
            &mut self.ch4_envelope_volume,
            self.ch4_envelope_period,
            self.ch4_envelope_direction,
        );
    }

    // Register access methods (for MMU integration)
    pub fn read_nr50(&self) -> u8 {
        (self.left_volume & 0x7) | ((self.right_volume & 0x7) << 4)
    }

    pub fn write_nr50(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.left_volume = value & 0x7;
        self.right_volume = (value >> 4) & 0x7;
    }

    pub fn read_nr51(&self) -> u8 {
        (if self.ch4_left_enable { 0x80 } else { 0 })
            | (if self.ch3_left_enable { 0x40 } else { 0 })
            | (if self.ch2_left_enable { 0x20 } else { 0 })
            | (if self.ch1_left_enable { 0x10 } else { 0 })
            | (if self.ch4_right_enable { 0x08 } else { 0 })
            | (if self.ch3_right_enable { 0x04 } else { 0 })
            | (if self.ch2_right_enable { 0x02 } else { 0 })
            | (if self.ch1_right_enable { 0x01 } else { 0 })
    }

    pub fn write_nr51(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch4_left_enable = (value & 0x80) != 0;
        self.ch3_left_enable = (value & 0x40) != 0;
        self.ch2_left_enable = (value & 0x20) != 0;
        self.ch1_left_enable = (value & 0x10) != 0;
        self.ch4_right_enable = (value & 0x08) != 0;
        self.ch3_right_enable = (value & 0x04) != 0;
        self.ch2_right_enable = (value & 0x02) != 0;
        self.ch1_right_enable = (value & 0x01) != 0;
    }

    pub fn read_nr52(&self) -> u8 {
        let master_bit = (self.master_enabled as u8) << 7;
        if !self.master_enabled {
            0x70 | master_bit
        } else {
            0x70 | master_bit
                | ((self.ch4_enabled as u8) << 3)
                | ((self.ch3_enabled as u8) << 2)
                | ((self.ch2_enabled as u8) << 1)
                | (self.ch1_enabled as u8)
        }
    }

    pub fn write_nr52(&mut self, value: u8) {
        let new_enabled = (value & 0x80) != 0;
        if !new_enabled {
            // Power off: reset all registers
            self.master_enabled = false;
            self.ch1_enabled = false;
            self.ch2_enabled = false;
            self.ch3_enabled = false;
            self.ch4_enabled = false;
            // Reset all registers to 0
            self.left_volume = 0;
            self.right_volume = 0;
            self.ch1_left_enable = false;
            self.ch1_right_enable = false;
            self.ch2_left_enable = false;
            self.ch2_right_enable = false;
            self.ch3_left_enable = false;
            self.ch3_right_enable = false;
            self.ch4_left_enable = false;
            self.ch4_right_enable = false;
        } else if !self.master_enabled {
            // Power on: reset frame sequencer
            self.master_enabled = true;
            self.frame_sequencer_step = 0;
        }
    }

    // CH1 registers
    pub fn read_nr10(&self) -> u8 {
        ((self.ch1_sweep_shift << 4) | ((self.ch1_sweep_negate as u8) << 3) | self.ch1_sweep_period)
            | 0x80
    }

    pub fn write_nr10(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch1_sweep_period = value & 0x7;
        self.ch1_sweep_negate = (value & 0x08) != 0;
        self.ch1_sweep_shift = (value >> 4) & 0x7;
    }

    pub fn read_nr11(&self) -> u8 {
        (self.ch1_duty << 6) | 0x3F
    }

    pub fn write_nr11(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch1_duty = (value >> 6) & 0x3;
        self.ch1_length_counter = 64 - (value & 0x3F);
    }

    pub fn read_nr12(&self) -> u8 {
        (self.ch1_envelope_period & 0x7)
            | (if self.ch1_envelope_direction { 0x08 } else { 0 })
            | ((self.ch1_envelope_volume & 0xF) << 4)
    }

    pub fn write_nr12(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch1_envelope_period = value & 0x7;
        self.ch1_envelope_direction = (value & 0x08) != 0;
        self.ch1_envelope_volume = (value >> 4) & 0xF;
        self.ch1_dac_enabled = (value & 0xF8) != 0;
    }

    pub fn read_nr13(&self) -> u8 {
        0xFF
    }

    pub fn write_nr13(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch1_frequency = (self.ch1_frequency & 0xFF00) | value as u16;
    }

    pub fn read_nr14(&self) -> u8 {
        (if self.ch1_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr14(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch1_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;
        self.ch1_frequency = (self.ch1_frequency & 0x00FF) | (((value & 0x7) as u16) << 8);

        if trigger {
            self.trigger_ch1();
        }
    }

    fn trigger_envelope(
        envelope_timer: &mut u8,
        envelope_volume: &mut u8,
        envelope_period: u8,
        initial_volume: u8,
    ) {
        *envelope_timer = if envelope_period == 0 {
            8
        } else {
            envelope_period
        };
        *envelope_volume = initial_volume;
    }

    fn trigger_length_counter(length_counter: &mut u8, default_value: u8) {
        if *length_counter == 0 {
            *length_counter = default_value;
        }
    }

    fn trigger_ch1(&mut self) {
        if self.ch1_dac_enabled {
            self.ch1_enabled = true;
        }
        Self::trigger_length_counter(&mut self.ch1_length_counter, 64);
        let initial_volume = (self.read_nr12() >> 4) & 0xF;
        Self::trigger_envelope(
            &mut self.ch1_envelope_timer,
            &mut self.ch1_envelope_volume,
            self.ch1_envelope_period,
            initial_volume,
        );
        self.ch1_sweep_shadow_freq = self.ch1_frequency;
        self.ch1_sweep_timer = if self.ch1_sweep_period == 0 {
            8
        } else {
            self.ch1_sweep_period
        };
        self.ch1_sweep_enabled = self.ch1_sweep_period > 0 || self.ch1_sweep_shift > 0;
        if self.ch1_sweep_shift > 0 {
            // Calculate new frequency immediately
            self.clock_sweep();
        }
    }

    // CH2 registers
    pub fn read_nr21(&self) -> u8 {
        (self.ch2_duty << 6) | 0x3F
    }

    pub fn write_nr21(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch2_duty = (value >> 6) & 0x3;
        self.ch2_length_counter = 64 - (value & 0x3F);
    }

    pub fn read_nr22(&self) -> u8 {
        (self.ch2_envelope_period & 0x7)
            | (if self.ch2_envelope_direction { 0x08 } else { 0 })
            | ((self.ch2_envelope_volume & 0xF) << 4)
    }

    pub fn write_nr22(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch2_envelope_period = value & 0x7;
        self.ch2_envelope_direction = (value & 0x08) != 0;
        self.ch2_envelope_volume = (value >> 4) & 0xF;
        self.ch2_dac_enabled = (value & 0xF8) != 0;
    }

    pub fn read_nr23(&self) -> u8 {
        0xFF
    }

    pub fn write_nr23(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch2_frequency = (self.ch2_frequency & 0xFF00) | value as u16;
    }

    pub fn read_nr24(&self) -> u8 {
        (if self.ch2_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr24(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch2_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;
        self.ch2_frequency = (self.ch2_frequency & 0x00FF) | (((value & 0x7) as u16) << 8);

        if trigger {
            self.trigger_ch2();
        }
    }

    fn trigger_ch2(&mut self) {
        if self.ch2_dac_enabled {
            self.ch2_enabled = true;
        }
        Self::trigger_length_counter(&mut self.ch2_length_counter, 64);
        let initial_volume = (self.read_nr22() >> 4) & 0xF;
        Self::trigger_envelope(
            &mut self.ch2_envelope_timer,
            &mut self.ch2_envelope_volume,
            self.ch2_envelope_period,
            initial_volume,
        );
    }

    // CH3 registers
    pub fn read_nr30(&self) -> u8 {
        (if self.ch3_dac_enabled { 0x80 } else { 0 }) | 0x7F
    }

    pub fn write_nr30(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch3_dac_enabled = (value & 0x80) != 0;
    }

    pub fn read_nr31(&self) -> u8 {
        0xFF
    }

    pub fn write_nr31(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch3_length_counter = (256 - value as u16) as u8;
    }

    pub fn read_nr32(&self) -> u8 {
        (self.ch3_output_level << 5) | 0x9F
    }

    pub fn write_nr32(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch3_output_level = (value >> 5) & 0x3;
    }

    pub fn read_nr33(&self) -> u8 {
        0xFF
    }

    pub fn write_nr33(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch3_frequency = (self.ch3_frequency & 0xFF00) | value as u16;
    }

    pub fn read_nr34(&self) -> u8 {
        (if self.ch3_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr34(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch3_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;
        self.ch3_frequency = (self.ch3_frequency & 0x00FF) | (((value & 0x7) as u16) << 8);

        if trigger {
            self.trigger_ch3();
        }
    }

    fn trigger_ch3(&mut self) {
        if self.ch3_dac_enabled {
            self.ch3_enabled = true;
            Self::trigger_length_counter(&mut self.ch3_length_counter, 0);
            self.ch3_wave_position = 0;
        }
    }

    // Wave RAM access (0xFF30-0xFF3F)
    pub fn read_wave_ram(&self, addr: u16) -> u8 {
        let index = (addr - 0xFF30) as usize;
        if index < 16 {
            if self.ch3_enabled {
                // When CH3 is enabled, reading wave RAM returns the current playing sample
                let sample_index = self.ch3_wave_position / 2;
                if sample_index < 16 {
                    self.ch3_wave_ram[sample_index]
                } else {
                    0xFF
                }
            } else {
                self.ch3_wave_ram[index]
            }
        } else {
            0
        }
    }

    pub fn write_wave_ram(&mut self, addr: u16, value: u8) {
        let index = (addr - 0xFF30) as usize;
        if index < 16 {
            self.ch3_wave_ram[index] = value;
        }
    }

    // CH4 registers
    pub fn read_nr41(&self) -> u8 {
        0xFF
    }

    pub fn write_nr41(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch4_length_counter = 64 - (value & 0x3F);
    }

    pub fn read_nr42(&self) -> u8 {
        (self.ch4_envelope_period & 0x7)
            | (if self.ch4_envelope_direction { 0x08 } else { 0 })
            | ((self.ch4_envelope_volume & 0xF) << 4)
    }

    pub fn write_nr42(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch4_envelope_period = value & 0x7;
        self.ch4_envelope_direction = (value & 0x08) != 0;
        self.ch4_envelope_volume = (value >> 4) & 0xF;
        self.ch4_dac_enabled = (value & 0xF8) != 0;
    }

    pub fn read_nr43(&self) -> u8 {
        (self.ch4_clock_shift & 0xF)
            | (if self.ch4_lfsr_width { 0x08 } else { 0 })
            | ((self.ch4_clock_divider & 0x7) << 4)
    }

    pub fn write_nr43(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch4_clock_shift = value & 0xF;
        self.ch4_lfsr_width = (value & 0x08) != 0;
        self.ch4_clock_divider = (value >> 4) & 0x7;
    }

    pub fn read_nr44(&self) -> u8 {
        (if self.ch4_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr44(&mut self, value: u8) {
        if !self.master_enabled {
            return;
        }
        self.ch4_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;

        if trigger {
            self.trigger_ch4();
        }
    }

    fn trigger_ch4(&mut self) {
        if self.ch4_dac_enabled {
            self.ch4_enabled = true;
        }
        Self::trigger_length_counter(&mut self.ch4_length_counter, 64);
        let initial_volume = (self.read_nr42() >> 4) & 0xF;
        Self::trigger_envelope(
            &mut self.ch4_envelope_timer,
            &mut self.ch4_envelope_volume,
            self.ch4_envelope_period,
            initial_volume,
        );
        self.ch4_lfsr = 0x7FFF;
    }
}

// Minimal, non-cycle-accurate APU synth for audible output (channels 1 and 2)
#[derive(Debug, Clone)]
pub struct SimpleAPUSynth {
    pub master_gain: f32, // additional master volume 0.0..=2.0 (software gain)
    // Internal new APU
    apu: Apu,
}

impl Default for SimpleAPUSynth {
    fn default() -> Self {
        Self {
            master_gain: 0.35,
            apu: Apu::new(),
        }
    }
}

impl SimpleAPUSynth {
    /// Replace the internal APU state with the provided one (used to sync MMU writes).
    pub fn set_apu_state(&mut self, apu: Apu) {
        self.apu = apu;
    }

    /// Return a clone of the internal APU state.
    pub fn get_apu_state(&self) -> Apu {
        self.apu.clone()
    }

    /// Step the internal APU by `cycles` (u16). This delegates to `Apu::step`.
    pub fn step_apu(&mut self, cycles: u16) {
        self.apu.step(cycles);
    }

    /// Set software master gain (0.0..=2.0)
    pub fn set_master_gain(&mut self, g: f32) {
        self.master_gain = g.max(0.0).min(2.0);
    }

    /// Enable APU master power (equivalent to writing NR52 bit7)
    pub fn power_on(&mut self) {
        self.apu.write_nr52(0x80);
    }

    /// Power off APU (resets registers)
    pub fn power_off(&mut self) {
        self.apu.write_nr52(0x00);
    }

    /// Trigger a simple CH1 test tone with provided frequency value (GB frequency reg value)
    /// This is a convenience helper for manual testing.
    pub fn play_test_tone_ch1(&mut self, freq_reg: u16) {
        // Ensure master and DAC are enabled
        self.apu.write_nr52(0x80);
        // NR12: envelope volume (0xF << 4) and direction 0
        self.apu.write_nr12((0xF << 4) as u8);
        // NR11: duty + length (use default duty 2)
        self.apu.write_nr11((2u8 << 6) | 0x00);
        // NR13/NR14: set frequency low and high with trigger
        let low = (freq_reg & 0xFF) as u8;
        let high = (((freq_reg >> 8) & 0x7) as u8) | 0x80; // trigger bit
        self.apu.write_nr13(low);
        self.apu.write_nr14(high);
    }
}

struct EmuAudioCallback {
    synth: Arc<Mutex<SimpleAPUSynth>>,
}

impl AudioCallback<f32> for EmuAudioCallback {
    fn callback(&mut self, stream: &mut AudioStream, requested: i32) {
        let mut out = Vec::<f32>::with_capacity(requested as usize);
        let mut guard = self.synth.lock().unwrap();
        let s = &mut *guard;

        // Step the APU by the number of cycles for these samples
        // At 44.1kHz, each sample is about 95 cycles (4.194304MHz / 44100)
        let cycles_per_sample = 95;
        let total_cycles = cycles_per_sample * requested as u16;

        s.apu.step(total_cycles);

        for _ in 0..requested {
            // Generate stereo sample from APU
            let (left, right) = s.apu.generate_sample();

            // Mix to mono for now (average left and right)
            let mono_sample = (left + right) * 0.5;

            // Apply software master gain
            let sample = mono_sample * s.master_gain.max(0.0).min(2.0);

            // Clamp to [-1.0, 1.0]
            let sample = sample.max(-1.0).min(1.0);

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
            .open_playback_stream(&source_spec, EmuAudioCallback { synth })
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
            .open_playback_stream(&source_spec, EmuAudioCallback { synth })
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
