// Game Boy APU (Audio Processing Unit) implementation
// Channels: CH1 (Square), CH2 (Square), CH3 (Wave), CH4 (Noise)
// Features: Envelope, Length, Sweep (CH1), Frame Sequencer

pub struct OldApu {
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

    // Channel 3: Wave
    ch3_enabled: bool,
    ch3_frequency: u16,
    ch3_length_counter: u8,
    ch3_length_enabled: bool,
    ch3_output_level: u8,   // 0=mute, 1=100%, 2=50%, 3=25%
    ch3_wave_ram: [u8; 16], // 32 4-bit samples
    ch3_wave_position: usize,

    // Channel 4: Noise
    ch4_enabled: bool,
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

impl OldApu {
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

            ch2_enabled: false,
            ch2_frequency: 0,
            ch2_duty: 2,
            ch2_length_counter: 0,
            ch2_length_enabled: false,
            ch2_envelope_volume: 0,
            ch2_envelope_period: 0,
            ch2_envelope_direction: false,
            ch2_envelope_timer: 0,

            ch3_enabled: false,
            ch3_frequency: 0,
            ch3_length_counter: 0,
            ch3_length_enabled: false,
            ch3_output_level: 0,
            ch3_wave_ram: [0; 16],
            ch3_wave_position: 0,

            ch4_enabled: false,
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
        // CH1 frequency: 131072 / (2048 - frequency)
        if self.ch1_frequency < 2048 {
            let freq_hz = 131072.0 / (2048.0 - self.ch1_frequency as f32);
            let cycles_per_sample = 4194304.0 / freq_hz / 8.0; // 8 phases per duty cycle
            self.ch1_phase =
                ((self.ch1_phase as u32 + cycles as u32) % cycles_per_sample as u32) as u8;
        }

        // CH2 similar to CH1
        if self.ch2_frequency < 2048 {
            let freq_hz = 131072.0 / (2048.0 - self.ch2_frequency as f32);
            let cycles_per_sample = 4194304.0 / freq_hz / 8.0;
            self.ch2_phase =
                ((self.ch2_phase as u32 + cycles as u32) % cycles_per_sample as u32) as u8;
        }

        // CH3: 65536 / (2048 - frequency) Hz, 32 samples per cycle
        if self.ch3_frequency < 2048 {
            let freq_hz = 65536.0 / (2048.0 - self.ch3_frequency as f32);
            let cycles_per_sample = 4194304.0 / freq_hz / 32.0;
            self.ch3_phase =
                ((self.ch3_phase as u32 + cycles as u32) % cycles_per_sample as u32) as u16;
            self.ch3_wave_position =
                (self.ch3_phase as usize / (cycles_per_sample as usize / 32)) % 32;
        }

        // CH4: Noise frequency based on clock divider and shift
        let divisor = match self.ch4_clock_divider {
            0 => 8,
            n => n * 16,
        };
        let freq_hz = 524288.0 / divisor as f32 / (2.0f32).powi(self.ch4_clock_shift as i32);
        let cycles_per_sample = 4194304.0 / freq_hz;
        self.ch4_phase =
            ((self.ch4_phase as u32 + cycles as u32) % cycles_per_sample as u32) as u16;
        // Update LFSR when phase wraps
        if self.ch4_phase < cycles as u16 {
            self.update_ch4_lfsr();
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

    fn generate_ch1_sample(&mut self) -> i8 {
        let duty_patterns = [
            0b00000001, // 12.5%
            0b00000011, // 25%
            0b00001111, // 50%
            0b00111111, // 75%
        ];
        let pattern = duty_patterns[self.ch1_duty as usize % 4];
        let bit = (pattern >> (7 - self.ch1_phase)) & 1;
        if bit != 0 {
            self.ch1_envelope_volume as i8
        } else {
            0
        }
    }

    fn generate_ch2_sample(&mut self) -> i8 {
        let duty_patterns = [0b00000001, 0b00000011, 0b00001111, 0b00111111];
        let pattern = duty_patterns[self.ch2_duty as usize % 4];
        let bit = (pattern >> (7 - self.ch2_phase)) & 1;
        if bit != 0 {
            self.ch2_envelope_volume as i8
        } else {
            0
        }
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

    fn clock_length_counters(&mut self) {
        if self.ch1_length_enabled && self.ch1_length_counter > 0 {
            self.ch1_length_counter -= 1;
            if self.ch1_length_counter == 0 {
                self.ch1_enabled = false;
            }
        }
        if self.ch2_length_enabled && self.ch2_length_counter > 0 {
            self.ch2_length_counter -= 1;
            if self.ch2_length_counter == 0 {
                self.ch2_enabled = false;
            }
        }
        if self.ch3_length_enabled && self.ch3_length_counter > 0 {
            self.ch3_length_counter -= 1;
            if self.ch3_length_counter == 0 {
                self.ch3_enabled = false;
            }
        }
        if self.ch4_length_enabled && self.ch4_length_counter > 0 {
            self.ch4_length_counter -= 1;
            if self.ch4_length_counter == 0 {
                self.ch4_enabled = false;
            }
        }
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
                let delta = self.ch1_sweep_shadow_freq >> self.ch1_sweep_shift;
                let new_freq = if self.ch1_sweep_negate {
                    self.ch1_sweep_shadow_freq.wrapping_sub(delta)
                } else {
                    self.ch1_sweep_shadow_freq.wrapping_add(delta)
                };

                if new_freq > 2047 {
                    self.ch1_enabled = false;
                } else if self.ch1_sweep_shift > 0 {
                    self.ch1_sweep_shadow_freq = new_freq;
                    self.ch1_frequency = new_freq;
                    // Check again after update
                    let delta2 = new_freq >> self.ch1_sweep_shift;
                    let new_freq2 = if self.ch1_sweep_negate {
                        new_freq.wrapping_sub(delta2)
                    } else {
                        new_freq.wrapping_add(delta2)
                    };
                    if new_freq2 > 2047 {
                        self.ch1_enabled = false;
                    }
                }
            }
        }
    }

    fn clock_envelopes(&mut self) {
        if self.ch1_envelope_period > 0 {
            if self.ch1_envelope_timer > 0 {
                self.ch1_envelope_timer -= 1;
            }
            if self.ch1_envelope_timer == 0 {
                self.ch1_envelope_timer = self.ch1_envelope_period;
                if self.ch1_envelope_direction && self.ch1_envelope_volume < 15 {
                    self.ch1_envelope_volume += 1;
                } else if !self.ch1_envelope_direction && self.ch1_envelope_volume > 0 {
                    self.ch1_envelope_volume -= 1;
                }
            }
        }

        if self.ch2_envelope_period > 0 {
            if self.ch2_envelope_timer > 0 {
                self.ch2_envelope_timer -= 1;
            }
            if self.ch2_envelope_timer == 0 {
                self.ch2_envelope_timer = self.ch2_envelope_period;
                if self.ch2_envelope_direction && self.ch2_envelope_volume < 15 {
                    self.ch2_envelope_volume += 1;
                } else if !self.ch2_envelope_direction && self.ch2_envelope_volume > 0 {
                    self.ch2_envelope_volume -= 1;
                }
            }
        }

        if self.ch4_envelope_period > 0 {
            if self.ch4_envelope_timer > 0 {
                self.ch4_envelope_timer -= 1;
            }
            if self.ch4_envelope_timer == 0 {
                self.ch4_envelope_timer = self.ch4_envelope_period;
                if self.ch4_envelope_direction && self.ch4_envelope_volume < 15 {
                    self.ch4_envelope_volume += 1;
                } else if !self.ch4_envelope_direction && self.ch4_envelope_volume > 0 {
                    self.ch4_envelope_volume -= 1;
                }
            }
        }
    }

    // Register access methods (for MMU integration)
    pub fn read_nr50(&self) -> u8 {
        (self.left_volume & 0x7) | ((self.right_volume & 0x7) << 4)
    }

    pub fn write_nr50(&mut self, value: u8) {
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
        0x70 | (if self.master_enabled { 0x80 } else { 0 })
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
        (self.ch1_sweep_period & 0x7)
            | (if self.ch1_sweep_negate { 0x08 } else { 0 })
            | ((self.ch1_sweep_shift & 0x7) << 4)
    }

    pub fn write_nr10(&mut self, value: u8) {
        self.ch1_sweep_period = value & 0x7;
        self.ch1_sweep_negate = (value & 0x08) != 0;
        self.ch1_sweep_shift = (value >> 4) & 0x7;
    }

    pub fn read_nr11(&self) -> u8 {
        (self.ch1_duty << 6) | 0x3F
    }

    pub fn write_nr11(&mut self, value: u8) {
        self.ch1_duty = (value >> 6) & 0x3;
        self.ch1_length_counter = 64 - (value & 0x3F);
    }

    pub fn read_nr12(&self) -> u8 {
        (self.ch1_envelope_period & 0x7)
            | (if self.ch1_envelope_direction { 0x08 } else { 0 })
            | ((self.ch1_envelope_volume & 0xF) << 4)
    }

    pub fn write_nr12(&mut self, value: u8) {
        self.ch1_envelope_period = value & 0x7;
        self.ch1_envelope_direction = (value & 0x08) != 0;
        self.ch1_envelope_volume = (value >> 4) & 0xF;
    }

    pub fn read_nr13(&self) -> u8 {
        self.ch1_frequency as u8
    }

    pub fn write_nr13(&mut self, value: u8) {
        self.ch1_frequency = (self.ch1_frequency & 0xFF00) | value as u16;
    }

    pub fn read_nr14(&self) -> u8 {
        (if self.ch1_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr14(&mut self, value: u8) {
        self.ch1_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;
        self.ch1_frequency = (self.ch1_frequency & 0x00FF) | (((value & 0x7) as u16) << 8);

        if trigger {
            self.trigger_ch1();
        }
    }

    fn trigger_ch1(&mut self) {
        self.ch1_enabled = true;
        if self.ch1_length_counter == 0 {
            self.ch1_length_counter = 64;
        }
        self.ch1_envelope_timer = if self.ch1_envelope_period == 0 {
            8
        } else {
            self.ch1_envelope_period
        };
        self.ch1_envelope_volume = (self.read_nr12() >> 4) & 0xF;
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
        self.ch2_duty = (value >> 6) & 0x3;
        self.ch2_length_counter = 64 - (value & 0x3F);
    }

    pub fn read_nr22(&self) -> u8 {
        (self.ch2_envelope_period & 0x7)
            | (if self.ch2_envelope_direction { 0x08 } else { 0 })
            | ((self.ch2_envelope_volume & 0xF) << 4)
    }

    pub fn write_nr22(&mut self, value: u8) {
        self.ch2_envelope_period = value & 0x7;
        self.ch2_envelope_direction = (value & 0x08) != 0;
        self.ch2_envelope_volume = (value >> 4) & 0xF;
    }

    pub fn read_nr23(&self) -> u8 {
        self.ch2_frequency as u8
    }

    pub fn write_nr23(&mut self, value: u8) {
        self.ch2_frequency = (self.ch2_frequency & 0xFF00) | value as u16;
    }

    pub fn read_nr24(&self) -> u8 {
        (if self.ch2_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr24(&mut self, value: u8) {
        self.ch2_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;
        self.ch2_frequency = (self.ch2_frequency & 0x00FF) | (((value & 0x7) as u16) << 8);

        if trigger {
            self.trigger_ch2();
        }
    }

    fn trigger_ch2(&mut self) {
        self.ch2_enabled = true;
        if self.ch2_length_counter == 0 {
            self.ch2_length_counter = 64;
        }
        self.ch2_envelope_timer = if self.ch2_envelope_period == 0 {
            8
        } else {
            self.ch2_envelope_period
        };
        self.ch2_envelope_volume = (self.read_nr22() >> 4) & 0xF;
    }

    // CH3 registers
    pub fn read_nr30(&self) -> u8 {
        (if self.ch3_enabled { 0x80 } else { 0 }) | 0x7F
    }

    pub fn write_nr30(&mut self, value: u8) {
        self.ch3_enabled = (value & 0x80) != 0;
    }

    pub fn read_nr31(&self) -> u8 {
        0xFF
    }

    pub fn write_nr31(&mut self, value: u8) {
        self.ch3_length_counter = (256 - value as u16) as u8;
    }

    pub fn read_nr32(&self) -> u8 {
        (self.ch3_output_level << 5) | 0x9F
    }

    pub fn write_nr32(&mut self, value: u8) {
        self.ch3_output_level = (value >> 5) & 0x3;
    }

    pub fn read_nr33(&self) -> u8 {
        self.ch3_frequency as u8
    }

    pub fn write_nr33(&mut self, value: u8) {
        self.ch3_frequency = (self.ch3_frequency & 0xFF00) | value as u16;
    }

    pub fn read_nr34(&self) -> u8 {
        (if self.ch3_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr34(&mut self, value: u8) {
        self.ch3_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;
        self.ch3_frequency = (self.ch3_frequency & 0x00FF) | (((value & 0x7) as u16) << 8);

        if trigger {
            self.trigger_ch3();
        }
    }

    fn trigger_ch3(&mut self) {
        self.ch3_enabled = true;
        if self.ch3_length_counter == 0 {
            self.ch3_length_counter = 255;
        }
        self.ch3_wave_position = 0;
    }

    // Wave RAM access (0xFF30-0xFF3F)
    pub fn read_wave_ram(&self, addr: u16) -> u8 {
        let index = (addr - 0xFF30) as usize;
        if index < 16 {
            self.ch3_wave_ram[index]
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
        self.ch4_length_counter = 64 - (value & 0x3F);
    }

    pub fn read_nr42(&self) -> u8 {
        (self.ch4_envelope_period & 0x7)
            | (if self.ch4_envelope_direction { 0x08 } else { 0 })
            | ((self.ch4_envelope_volume & 0xF) << 4)
    }

    pub fn write_nr42(&mut self, value: u8) {
        self.ch4_envelope_period = value & 0x7;
        self.ch4_envelope_direction = (value & 0x08) != 0;
        self.ch4_envelope_volume = (value >> 4) & 0xF;
    }

    pub fn read_nr43(&self) -> u8 {
        (self.ch4_clock_shift & 0xF)
            | (if self.ch4_lfsr_width { 0x08 } else { 0 })
            | ((self.ch4_clock_divider & 0x7) << 4)
    }

    pub fn write_nr43(&mut self, value: u8) {
        self.ch4_clock_shift = value & 0xF;
        self.ch4_lfsr_width = (value & 0x08) != 0;
        self.ch4_clock_divider = (value >> 4) & 0x7;
    }

    pub fn read_nr44(&self) -> u8 {
        (if self.ch4_length_enabled { 0x40 } else { 0 }) | 0xBF
    }

    pub fn write_nr44(&mut self, value: u8) {
        self.ch4_length_enabled = (value & 0x40) != 0;
        let trigger = (value & 0x80) != 0;

        if trigger {
            self.trigger_ch4();
        }
    }

    fn trigger_ch4(&mut self) {
        self.ch4_enabled = true;
        if self.ch4_length_counter == 0 {
            self.ch4_length_counter = 64;
        }
        self.ch4_envelope_timer = if self.ch4_envelope_period == 0 {
            8
        } else {
            self.ch4_envelope_period
        };
        self.ch4_envelope_volume = (self.read_nr42() >> 4) & 0xF;
        self.ch4_lfsr = 0x7FFF;
    }
}
