#[derive(Default)]
pub struct WaveChannel {
    pub enabled: bool,
    pub frequency: u16,
    pub volume: u8,
    pub wave_ram: [u8; 16],
}

#[derive(Default)]
pub struct NoiseChannel {
    pub enabled: bool,
    pub frequency: u16,
    pub volume: u8,
    pub length: u8,
    pub envelope: u8,
}
/// Game Boy APU (音效處理單元) - 音頻通道結構

#[derive(Default)]
pub struct SquareChannel {
    pub enabled: bool,
    pub frequency: u16,
    pub volume: u8,
    pub duty: u8,
    pub length: u8,
    pub envelope: u8,
}
pub struct APU {
    pub ch1: SquareChannel, // NR10~NR14
    pub ch2: SquareChannel, // NR21~NR24
    pub ch3: WaveChannel,   // NR30~NR34
    pub ch4: NoiseChannel,  // NR41~NR44
    pub left_enable: u8,
    pub right_enable: u8,
    pub master_volume: u8,
}

impl APU {
    /// 混合所有通道並套用音量控制，產生左右聲道音訊
    pub fn mix(&self, sample_rate: u32, duration_ms: u32) -> (Vec<u8>, Vec<u8>) {
        let mut left = vec![0u8; (sample_rate * duration_ms / 1000) as usize];
        let mut right = vec![0u8; (sample_rate * duration_ms / 1000) as usize];
        let ch1 = self.ch1.generate_wave(sample_rate, duration_ms);
        let ch2 = self.ch2.generate_wave(sample_rate, duration_ms);
        let ch3 = self.ch3.generate_wave(sample_rate, duration_ms);
        let ch4 = self.ch4.generate_wave(sample_rate, duration_ms);
        for i in 0..left.len() {
            if self.left_enable & 0x01 != 0 {
                left[i] = left[i].saturating_add(ch1[i] * self.master_volume / 7);
            }
            if self.left_enable & 0x02 != 0 {
                left[i] = left[i].saturating_add(ch2[i] * self.master_volume / 7);
            }
            if self.left_enable & 0x04 != 0 {
                left[i] = left[i].saturating_add(ch3[i] * self.master_volume / 7);
            }
            if self.left_enable & 0x08 != 0 {
                left[i] = left[i].saturating_add(ch4[i] * self.master_volume / 7);
            }
            if self.right_enable & 0x01 != 0 {
                right[i] = right[i].saturating_add(ch1[i] * self.master_volume / 7);
            }
            if self.right_enable & 0x02 != 0 {
                right[i] = right[i].saturating_add(ch2[i] * self.master_volume / 7);
            }
            if self.right_enable & 0x04 != 0 {
                right[i] = right[i].saturating_add(ch3[i] * self.master_volume / 7);
            }
            if self.right_enable & 0x08 != 0 {
                right[i] = right[i].saturating_add(ch4[i] * self.master_volume / 7);
            }
        }
        (left, right)
    }
    pub fn new() -> Self {
        Self {
            ch1: SquareChannel::default(),
            ch2: SquareChannel::default(),
            ch3: WaveChannel::default(),
            ch4: NoiseChannel::default(),
            left_enable: 0x0F,
            right_enable: 0x0F,
            master_volume: 7,
        }
    }
}

impl SquareChannel {
    /// 產生方波音訊 (8-bit PCM)
    pub fn generate_wave(&self, sample_rate: u32, duration_ms: u32) -> Vec<u8> {
        if !self.enabled || self.frequency == 0 || self.volume == 0 {
            return vec![0; (sample_rate * duration_ms / 1000) as usize];
        }
        let samples = (sample_rate * duration_ms / 1000) as usize;
        let freq = self.frequency as f32;
        let duty = match self.duty & 0x03 {
            0 => 0.125, // 12.5%
            1 => 0.25,  // 25%
            2 => 0.5,   // 50%
            3 => 0.75,  // 75%
            _ => 0.5,
        };
        let mut buf = Vec::with_capacity(samples);
        for i in 0..samples {
            let t = (i as f32 * freq / sample_rate as f32) % 1.0;
            let v = if t < duty { self.volume * 16 } else { 0 };
            buf.push(v);
        }
        buf
    }
}

impl WaveChannel {
    /// 產生自訂波形音訊 (8-bit PCM)
    pub fn generate_wave(&self, sample_rate: u32, duration_ms: u32) -> Vec<u8> {
        if !self.enabled || self.frequency == 0 || self.volume == 0 {
            return vec![0; (sample_rate * duration_ms / 1000) as usize];
        }
        let samples = (sample_rate * duration_ms / 1000) as usize;
        let freq = self.frequency as f32;
        let mut buf = Vec::with_capacity(samples);
        for i in 0..samples {
            let t = ((i as f32 * freq / sample_rate as f32) * 32.0) as usize % 32;
            let wave_idx = t / 2;
            let wave_byte = self.wave_ram[wave_idx % 16];
            let sample = if t % 2 == 0 {
                wave_byte >> 4
            } else {
                wave_byte & 0x0F
            };
            buf.push(sample * self.volume);
        }
        buf
    }
}

impl NoiseChannel {
    /// 產生雜訊音訊 (8-bit PCM)
    pub fn generate_wave(&self, sample_rate: u32, duration_ms: u32) -> Vec<u8> {
        use rand::Rng;
        if !self.enabled || self.volume == 0 {
            return vec![0; (sample_rate * duration_ms / 1000) as usize];
        }
        let samples = (sample_rate * duration_ms / 1000) as usize;
        let mut buf = Vec::with_capacity(samples);
        let mut rng = rand::rng();
        for _ in 0..samples {
            let v = rng.random_range(0..=self.volume * 16);
            buf.push(v);
        }
        buf
    }
}
