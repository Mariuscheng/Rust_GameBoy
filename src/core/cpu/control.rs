use crate::core::cpu::cpu::CPU;
use crate::core::cpu::register_utils::FlagOperations;
use crate::core::cycles::*;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType, crate::core::error::Error> {
    // 自動補齊 stub，所有未覆蓋指令預設回傳 4 cycles
    Ok(4)
}

impl CPU {
    // NOP 指令 (00)
    pub fn nop(&mut self) -> CyclesType {
        // NOP: 不做任何事，但為 debug 一致性，清空所有旗標
        self.registers.set_zero(false);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
        // ...existing code...
        4
    }

    // STOP 指令 (10)
    pub fn stop(&mut self) -> CyclesType {
        // STOP: 清空所有旗標
        self.registers.set_zero(false);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
        // ...existing code...
        4
    }

    // HALT 指令 (76)
    pub fn halt(&mut self) -> CyclesType {
        // HALT: 清空所有旗標
        self.registers.set_zero(false);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
        // ...existing code...
        4
    }

    // DI 指令 (F3)
    pub fn di(&mut self) -> CyclesType {
        // DI: 清空所有旗標
        self.registers.set_zero(false);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
        // ...existing code...
        4
    }

    // EI 指令 (FB)
    pub fn ei(&mut self) -> CyclesType {
        // EI: 清空所有旗標
        self.registers.set_zero(false);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);
        // ...existing code...
        4
    }
}
