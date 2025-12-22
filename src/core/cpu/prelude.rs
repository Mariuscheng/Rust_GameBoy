// Common exports for CPU instruction modules
pub use super::register_utils::FlagOperations;
pub use crate::core::cpu::cpu::CPU;
pub use crate::core::cycles::{CYCLES_2, CYCLES_3, CYCLES_4, CyclesType};

// Common cycle constants
pub const CB_PREFIX_CYCLES: CyclesType = CYCLES_2 as CyclesType;
pub const JUMP_CYCLES: CyclesType = CYCLES_3 as CyclesType;
pub const CALL_CYCLES: CyclesType = CYCLES_4 as CyclesType;

// 指令預設 trait
pub trait Instruction {
    fn execute(&mut self) -> CyclesType;
}

// 常用工具函式
pub fn to_u16(high: u8, low: u8) -> u16 {
    ((high as u16) << 8) | (low as u16)
}

pub fn to_u8(val: u16) -> (u8, u8) {
    ((val >> 8) as u8, (val & 0xFF) as u8)
}

// Define utility functions for flag operations
pub trait FlagUtils {
    fn update_zero_flag(&mut self, value: u8);
    fn update_carry_flag(&mut self, value: bool);
    fn update_half_carry_flag(&mut self, value: bool);
    fn update_subtract_flag(&mut self, value: bool);
}

impl FlagUtils for CPU {
    fn update_zero_flag(&mut self, value: u8) {
        self.registers.set_zero(value == 0);
    }
    fn update_carry_flag(&mut self, value: bool) {
        self.registers.set_carry(value);
    }
    fn update_half_carry_flag(&mut self, value: bool) {
        self.registers.set_half_carry(value);
    }
    fn update_subtract_flag(&mut self, value: bool) {
        self.registers.set_subtract(value);
    }
}
