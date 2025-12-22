use super::cpu::CPU;
use super::register_utils::RegTarget;
use crate::core::cpu::register_utils::FlagOperations;
use crate::core::cycles::CyclesType;
use crate::core::error::{Error, InstructionError, Result};

/// CB 前綴指令分派
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> crate::core::error::Result<CyclesType> {
    match opcode {
        // BIT 指令族 (CB 40~7F)
        0x40..=0x7F => Ok(cpu.bit_b_r(0, RegTarget::from_bits(opcode)?)),
        // SET 指令族 (CB C0~FF)
        0xC0..=0xFF => Ok(cpu.set_b_r(0, RegTarget::from_bits(opcode)?)),
        // RES 指令族 (CB 80~BF)
        0x80..=0xBF => Ok(cpu.res_b_r(0, RegTarget::from_bits(opcode)?)),
        // RL/RR/SLA/SRA/SRL/SWAP stub
        0x10..=0x3F => Ok(cpu.cb_misc(RegTarget::from_bits(opcode)?)),
        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

pub fn bit(cpu: &mut CPU, reg: RegTarget) -> CyclesType {
    // BIT 指令不會寫入 VRAM，只測試位元
    // 預設 BIT 測試 HL 指向記憶體某位元
    let addr = cpu.registers.get_hl();
    let value = cpu.read_byte(addr).unwrap_or(0);
    let bit = (value >> (reg as u8 & 0x07)) & 0x01;
    cpu.registers.set_zero(bit == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(true);
    // ...existing code...
    8
}
pub fn set(cpu: &mut CPU, reg: RegTarget) -> CyclesType {
    // SET 指令會將 HL 指向的記憶體某位元設為 1
    let addr = cpu.registers.get_hl();
    let mut value = cpu.read_byte(addr).unwrap_or(0);
    value |= 0x01 << (reg as u8 & 0x07);
    cpu.write_byte(addr, value).ok();
    cpu.log_vram_write(addr, value, "SET").ok();
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    // ...existing code...
    16
}
pub fn res(cpu: &mut CPU, reg: RegTarget) -> CyclesType {
    // RES 指令會將 HL 指向的記憶體某位元設為 0
    let addr = cpu.registers.get_hl();
    let mut value = cpu.read_byte(addr).unwrap_or(0);
    value &= !(0x01 << (reg as u8 & 0x07));
    cpu.write_byte(addr, value).ok();
    cpu.log_vram_write(addr, value, "RES").ok();
    cpu.registers.set_zero(false);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    // ...existing code...
    16
}
pub fn cb_misc(cpu: &mut CPU, reg: RegTarget) -> CyclesType {
    // RL/RR/SLA/SRA/SRL/SWAP stub
    let addr = cpu.registers.get_hl();
    let mut value = cpu.read_byte(addr).unwrap_or(0);
    // 這裡僅示範 RL (左移+最高位進最低位)
    let carry = (value & 0x80) != 0;
    value = (value << 1) | if carry { 1 } else { 0 };
    cpu.write_byte(addr, value).ok();
    cpu.log_vram_write(addr, value, "RL/SWAP stub").ok();
    cpu.registers.set_zero(value == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(carry);
    // ...existing code...
    16
}
