#![allow(dead_code)]
use super::cpu::CPU;
use crate::core::cpu::register_utils::FlagOperations;
use crate::core::cycles::CyclesType;
use crate::core::error::{Error, InstructionError};

/// 未公開/特殊指令分派
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> crate::core::error::Result<CyclesType> {
    match opcode {
        // SGB 指令範例 (如 0xFC)
        0xFC => Ok(undocumented_sgb(cpu)),
        // 未定義指令範例 (如 0xDD, 0xED, 0xFD)
        0xDD | 0xED | 0xFD => Ok(undocumented_nop(cpu)),
        // HALT bug 指令 (如 0x76)
        0x76 => Ok(undocumented_halt_bug(cpu)),
        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

pub fn undocumented_sgb(cpu: &mut CPU) -> CyclesType {
    // SGB: 清空所有旗標
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    // ...existing code...
    4
}

pub fn undocumented_nop(cpu: &mut CPU) -> CyclesType {
    // 未定義 NOP: 清空所有旗標
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    // ...existing code...
    4
}

pub fn undocumented_halt_bug(cpu: &mut CPU) -> CyclesType {
    // HALT bug: 清空所有旗標
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    // ...existing code...
    4
}
