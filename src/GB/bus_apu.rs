// Bus APU 相關欄位與邏輯
use crate::interface::audio::{Apu, SimpleAPUSynth};
use std::sync::{Arc, Mutex};

#[allow(dead_code)]
pub struct BusAPU {
    pub apu_dbg_printed: bool,
    pub apu_regs: [u8; 0x30],
    pub apu_synth: Option<Arc<Mutex<SimpleAPUSynth>>>,
    pub apu: Apu, // New APU instance for register access
    pub dbg_timer: bool,
}

impl BusAPU {
    pub fn new() -> Self {
        Self {
            apu_dbg_printed: false,
            apu_regs: [0; 0x30],
            apu_synth: None,
            apu: Apu::new(),
            dbg_timer: std::env::var("GB_DEBUG_TIMER")
                .ok()
                .map(|v| v != "0")
                .unwrap_or(false),
        }
    }
    pub fn attach_synth(
        &mut self,
        synth: std::sync::Arc<std::sync::Mutex<crate::interface::audio::SimpleAPUSynth>>,
    ) {
        self.apu_synth = Some(synth);
        // 初始化時同步一次 APU 狀態到音源
        self.apu_update_synth();
    }

    // APU register access methods
    pub fn read_apu_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => self.apu.read_nr10(),
            0xFF11 => self.apu.read_nr11(),
            0xFF12 => self.apu.read_nr12(),
            0xFF13 => self.apu.read_nr13(),
            0xFF14 => self.apu.read_nr14(),
            0xFF16 => self.apu.read_nr21(),
            0xFF17 => self.apu.read_nr22(),
            0xFF18 => self.apu.read_nr23(),
            0xFF19 => self.apu.read_nr24(),
            0xFF1A => self.apu.read_nr30(),
            0xFF1B => self.apu.read_nr31(),
            0xFF1C => self.apu.read_nr32(),
            0xFF1D => self.apu.read_nr33(),
            0xFF1E => self.apu.read_nr34(),
            0xFF20 => self.apu.read_nr41(),
            0xFF21 => self.apu.read_nr42(),
            0xFF22 => self.apu.read_nr43(),
            0xFF23 => self.apu.read_nr44(),
            0xFF24 => self.apu.read_nr50(),
            0xFF25 => self.apu.read_nr51(),
            0xFF26 => self.apu.read_nr52(),
            0xFF30..=0xFF3F => self.apu.read_wave_ram(addr),
            _ => 0xFF,
        }
    }

    pub fn write_apu_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF10 => self.apu.write_nr10(value),
            0xFF11 => self.apu.write_nr11(value),
            0xFF12 => self.apu.write_nr12(value),
            0xFF13 => self.apu.write_nr13(value),
            0xFF14 => self.apu.write_nr14(value),
            0xFF16 => self.apu.write_nr21(value),
            0xFF17 => self.apu.write_nr22(value),
            0xFF18 => self.apu.write_nr23(value),
            0xFF19 => self.apu.write_nr24(value),
            0xFF1A => self.apu.write_nr30(value),
            0xFF1B => self.apu.write_nr31(value),
            0xFF1C => self.apu.write_nr32(value),
            0xFF1D => self.apu.write_nr33(value),
            0xFF1E => self.apu.write_nr34(value),
            0xFF20 => self.apu.write_nr41(value),
            0xFF21 => self.apu.write_nr42(value),
            0xFF22 => self.apu.write_nr43(value),
            0xFF23 => self.apu.write_nr44(value),
            0xFF24 => self.apu.write_nr50(value),
            0xFF25 => self.apu.write_nr51(value),
            0xFF26 => self.apu.write_nr52(value),
            0xFF30..=0xFF3F => self.apu.write_wave_ram(addr, value),
            _ => {}
        }
        // 每次 register 寫入後嘗試同步到音源（若有 attach 的話）
        self.apu_update_synth();
    }

    // Step the APU (called from bus step). Accept u64 and invoke Apu::step in safe chunks.
    pub fn step_apu(&mut self, mut cycles: u64) {
        // Apu::step accepts u16; split cycles into u16 chunks to avoid truncation
        while cycles > 0 {
            let chunk = std::cmp::min(cycles, u16::MAX as u64) as u16;
            self.apu.step(chunk);
            cycles -= chunk as u64;
        }
    }

    /// Copy current Bus APU state into attached synth (if any).
    pub fn apu_update_synth(&mut self) {
        if let Some(ref synth) = self.apu_synth {
            if let Ok(mut guard) = synth.lock() {
                guard.set_apu_state(self.apu.clone());
            }
        }
    }
}
