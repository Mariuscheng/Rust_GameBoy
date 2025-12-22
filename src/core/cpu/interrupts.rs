#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interrupt {
    VBlank,
    LcdStat,
    Timer,
    Serial,
    Joypad,
}

impl Interrupt {
    pub fn to_bit(&self) -> u8 {
        match self {
            Interrupt::VBlank => 0,
            Interrupt::LcdStat => 1,
            Interrupt::Timer => 2,
            Interrupt::Serial => 3,
            Interrupt::Joypad => 4,
        }
    }

    pub fn from_bit(bit: u8) -> Option<Self> {
        match bit {
            0 => Some(Interrupt::VBlank),
            1 => Some(Interrupt::LcdStat),
            2 => Some(Interrupt::Timer),
            3 => Some(Interrupt::Serial),
            4 => Some(Interrupt::Joypad),
            _ => None,
        }
    }
}

pub struct InterruptRegisters {
    pub enable: u8,
    pub flag: u8,
}

impl InterruptRegisters {
    pub fn new() -> Self {
        Self { enable: 0, flag: 0 }
    }

    pub fn is_interrupt_enabled(&self, interrupt: Interrupt) -> bool {
        let bit = interrupt.to_bit();
        (self.enable & (1 << bit)) != 0
    }

    pub fn set_interrupt_enable(&mut self, interrupt: Interrupt, enabled: bool) {
        let bit = interrupt.to_bit();
        if enabled {
            self.enable |= 1 << bit;
        } else {
            self.enable &= !(1 << bit);
        }
    }

    pub fn is_interrupt_requested(&self, interrupt: Interrupt) -> bool {
        let bit = interrupt.to_bit();
        (self.flag & (1 << bit)) != 0
    }

    pub fn request_interrupt(&mut self, interrupt: Interrupt) {
        let bit = interrupt.to_bit();
        self.flag |= 1 << bit;
    }

    pub fn clear_interrupt_flag(&mut self, interrupt: Interrupt) {
        let bit = interrupt.to_bit();
        self.flag &= !(1 << bit);
    }

    pub fn has_pending_enabled_interrupts(&self) -> bool {
        (self.enable & self.flag) != 0
    }

    pub fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        let active = self.enable & self.flag;
        if active == 0 {
            return None;
        }

        // 優先順序：VBlank > LCD STAT > Timer > Serial > Joypad
        for i in 0..=4 {
            if (active & (1 << i)) != 0 {
                return Interrupt::from_bit(i);
            }
        }

        None
    }
}

// --- GameBoy CPU 中斷相關 stub function ---
pub fn enable_interrupt(registers: &mut InterruptRegisters, interrupt: Interrupt) {
    registers.set_interrupt_enable(interrupt, true);
}
pub fn disable_interrupt(registers: &mut InterruptRegisters, interrupt: Interrupt) {
    registers.set_interrupt_enable(interrupt, false);
}
pub fn request_interrupt(registers: &mut InterruptRegisters, interrupt: Interrupt) {
    registers.request_interrupt(interrupt);
}
pub fn clear_interrupt(registers: &mut InterruptRegisters, interrupt: Interrupt) {
    registers.clear_interrupt_flag(interrupt);
}
pub fn is_interrupt_enabled(registers: &InterruptRegisters, interrupt: Interrupt) -> bool {
    registers.is_interrupt_enabled(interrupt)
}
pub fn is_interrupt_requested(registers: &InterruptRegisters, interrupt: Interrupt) -> bool {
    registers.is_interrupt_requested(interrupt)
}
pub fn has_pending_enabled_interrupts(registers: &InterruptRegisters) -> bool {
    registers.has_pending_enabled_interrupts()
}
pub fn get_highest_priority_interrupt(registers: &InterruptRegisters) -> Option<Interrupt> {
    registers.get_highest_priority_interrupt()
}
