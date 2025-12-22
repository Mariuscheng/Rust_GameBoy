use crate::core::utils::logger::log_to_file;
// CB 指令 stub function
use super::register_utils::{FlagOperations, RegTarget};
use crate::core::cpu::cpu::CPU;
use crate::core::cycles::{CYCLES_2, CYCLES_3, CyclesType};
use crate::core::error::{Error, InstructionError};

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> crate::core::error::Result<CyclesType> {
    log_to_file(&format!(
        "[BIT] dispatch: opcode={:02X}, PC={:04X}",
        opcode, cpu.registers.pc
    ));
    // 自動補齊 stub，所有未覆蓋指令預設回傳 4 cycles
    Ok(4)
    // ...existing code...

    //     let result = value >> 1;
    //     self.registers.set_zero(result == 0);
    //     self.registers.set_subtract(false);
    //     self.registers.set_half_carry(false);
    //     self.registers.set_carry(carry);
    //     self.set_reg_value(reg, result);
    //     if matches!(reg, RegTarget::HL) {
    //         CYCLES_3
    //     } else {
    //         CYCLES_2
    //     }
    // }
}
// --- CB 前綴指令 stub function ---
pub fn op_cb_10(cpu: &mut CPU) -> CyclesType {
    let b = cpu.registers.b;
    let carry_in = if cpu.registers.get_carry() { 1 } else { 0 };
    let result = (b << 1) | carry_in;
    cpu.registers.b = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry((b & 0x80) != 0);
    2
}

pub fn op_cb_11(cpu: &mut CPU) -> CyclesType {
    let c = cpu.registers.c;
    let carry_in = if cpu.registers.get_carry() { 1 } else { 0 };
    let result = (c << 1) | carry_in;
    cpu.registers.c = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry((c & 0x80) != 0);
    2
}

pub fn op_cb_1_a(cpu: &mut CPU) -> CyclesType {
    let d = cpu.registers.d;
    let carry_in = if cpu.registers.get_carry() { 0x80 } else { 0 };
    let result = (d >> 1) | carry_in;
    cpu.registers.d = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry((d & 0x01) != 0);
    2
}

pub fn op_cb_23(cpu: &mut CPU) -> CyclesType {
    let e = cpu.registers.e;
    let result = e << 1;
    cpu.registers.e = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry((e & 0x80) != 0);
    2
}

pub fn op_cb_2_c(cpu: &mut CPU) -> CyclesType {
    let h = cpu.registers.h;
    let result = (h >> 1) | (h & 0x80);
    cpu.registers.h = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry((h & 0x01) != 0);
    2
}

pub fn op_cb_35(cpu: &mut CPU) -> CyclesType {
    let l = cpu.registers.l;
    let result = (l >> 4) | (l << 4);
    cpu.registers.l = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    2
}

pub fn op_cb_3_f(cpu: &mut CPU) -> CyclesType {
    let a = cpu.registers.a;
    let result = a >> 1;
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry((a & 0x01) != 0);
    2
}

// BIT 指令 (CB 40~7F)
pub fn bit(cpu: &mut CPU, bit: u8, target: RegTarget) -> CyclesType {
    // BIT 指令：檢查目標暫存器 bit 是否為 0，Z=1 表示該 bit 為 0
    let value = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => unsafe { &*cpu.mmu }
            .read_byte(cpu.registers.get_hl())
            .unwrap_or(0),
        RegTarget::Immediate => 0,
    };
    let z = (value & (1 << bit)) == 0;
    cpu.registers.set_zero(z);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(true);
    // BIT 指令不影響 Carry flag
    if matches!(target, RegTarget::HL) {
        3
    } else {
        2
    }
}

// SET 指令 (CB C0~FF)
pub fn set(cpu: &mut CPU, bit: u8, target: RegTarget) -> CyclesType {
    // SET 指令：將目標暫存器 bit 設為 1
    match target {
        RegTarget::A => cpu.registers.a |= 1 << bit,
        RegTarget::B => cpu.registers.b |= 1 << bit,
        RegTarget::C => cpu.registers.c |= 1 << bit,
        RegTarget::D => cpu.registers.d |= 1 << bit,
        RegTarget::E => cpu.registers.e |= 1 << bit,
        RegTarget::H => cpu.registers.h |= 1 << bit,
        RegTarget::L => cpu.registers.l |= 1 << bit,
        RegTarget::HL => {
            let addr = cpu.registers.get_hl();
            let mut val = unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0);
            val |= 1 << bit;
            let _ = unsafe { &mut *cpu.mmu }.write_byte(addr, val);
        }
        RegTarget::Immediate => (),
    }
    // ...existing code...
    if matches!(target, RegTarget::HL) {
        4
    } else {
        2
    }
}

// RES 指令 (CB 80~BF)

// RL 指令 (CB 10~17)
pub fn rl(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    // RL 指令：左移一位，最低位補入 Carry，最高位移出成新 Carry
    let val = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => unsafe { &*cpu.mmu }
            .read_byte(cpu.registers.get_hl())
            .unwrap_or(0),
        RegTarget::Immediate => 0,
    };
    let carry_in = if cpu.registers.get_carry() { 1 } else { 0 };
    let result = (val << 1) | carry_in;
    let new_carry = (val & 0x80) != 0;
    match target {
        RegTarget::A => cpu.registers.a = result,
        RegTarget::B => cpu.registers.b = result,
        RegTarget::C => cpu.registers.c = result,
        RegTarget::D => cpu.registers.d = result,
        RegTarget::E => cpu.registers.e = result,
        RegTarget::H => cpu.registers.h = result,
        RegTarget::L => cpu.registers.l = result,
        RegTarget::HL => {
            let _ = unsafe { &mut *cpu.mmu }.write_byte(cpu.registers.get_hl(), result);
        }
        RegTarget::Immediate => (),
    }
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(new_carry);
    // ...existing code...
    if matches!(target, RegTarget::HL) {
        4
    } else {
        2
    }
}

// RR 指令 (CB 18~1F)
pub fn rr(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    // RR 指令：右移一位，最高位補入 Carry，最低位移出成新 Carry
    let val = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => unsafe { &*cpu.mmu }
            .read_byte(cpu.registers.get_hl())
            .unwrap_or(0),
        RegTarget::Immediate => 0,
    };
    let carry_in = if cpu.registers.get_carry() { 0x80 } else { 0 };
    let result = (val >> 1) | carry_in;
    let new_carry = (val & 0x01) != 0;
    match target {
        RegTarget::A => cpu.registers.a = result,
        RegTarget::B => cpu.registers.b = result,
        RegTarget::C => cpu.registers.c = result,
        RegTarget::D => cpu.registers.d = result,
        RegTarget::E => cpu.registers.e = result,
        RegTarget::H => cpu.registers.h = result,
        RegTarget::L => cpu.registers.l = result,
        RegTarget::HL => {
            let _ = unsafe { &mut *cpu.mmu }.write_byte(cpu.registers.get_hl(), result);
        }
        RegTarget::Immediate => (),
    }
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(new_carry);
    // ...existing code...
    if matches!(target, RegTarget::HL) {
        4
    } else {
        2
    }
}

// SLA 指令 (CB 20~27)
pub fn sla(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    // SLA 指令：左移一位，最低位補 0，最高位移出成新 Carry
    let val = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => unsafe { &*cpu.mmu }
            .read_byte(cpu.registers.get_hl())
            .unwrap_or(0),
        RegTarget::Immediate => 0,
    };
    let result = val << 1;
    let new_carry = (val & 0x80) != 0;
    match target {
        RegTarget::A => cpu.registers.a = result,
        RegTarget::B => cpu.registers.b = result,
        RegTarget::C => cpu.registers.c = result,
        RegTarget::D => cpu.registers.d = result,
        RegTarget::E => cpu.registers.e = result,
        RegTarget::H => cpu.registers.h = result,
        RegTarget::L => cpu.registers.l = result,
        RegTarget::HL => {
            let _ = unsafe { &mut *cpu.mmu }.write_byte(cpu.registers.get_hl(), result);
        }
        RegTarget::Immediate => (),
    }
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(new_carry);
    // ...existing code...
    if matches!(target, RegTarget::HL) {
        4
    } else {
        2
    }
}

// SRA 指令 (CB 28~2F)
pub fn sra(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    // SRA 指令：右移一位，最高位不變，最低位移出成新 Carry
    let val = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => unsafe { &*cpu.mmu }
            .read_byte(cpu.registers.get_hl())
            .unwrap_or(0),
        RegTarget::Immediate => 0,
    };
    let result = (val >> 1) | (val & 0x80);
    let new_carry = (val & 0x01) != 0;
    match target {
        RegTarget::A => cpu.registers.a = result,
        RegTarget::B => cpu.registers.b = result,
        RegTarget::C => cpu.registers.c = result,
        RegTarget::D => cpu.registers.d = result,
        RegTarget::E => cpu.registers.e = result,
        RegTarget::H => cpu.registers.h = result,
        RegTarget::L => cpu.registers.l = result,
        RegTarget::HL => {
            let _ = unsafe { &mut *cpu.mmu }.write_byte(cpu.registers.get_hl(), result);
        }
        RegTarget::Immediate => (),
    }
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(new_carry);
    // ...existing code...
    if matches!(target, RegTarget::HL) {
        4
    } else {
        2
    }
}

// SRL 指令 (CB 38~3F)
pub fn srl(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    // SRL 指令：右移一位，最高位補 0，最低位移出成新 Carry
    let val = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => unsafe { &*cpu.mmu }
            .read_byte(cpu.registers.get_hl())
            .unwrap_or(0),
        RegTarget::Immediate => 0,
    };
    let result = val >> 1;
    let new_carry = (val & 0x01) != 0;
    match target {
        RegTarget::A => cpu.registers.a = result,
        RegTarget::B => cpu.registers.b = result,
        RegTarget::C => cpu.registers.c = result,
        RegTarget::D => cpu.registers.d = result,
        RegTarget::E => cpu.registers.e = result,
        RegTarget::H => cpu.registers.h = result,
        RegTarget::L => cpu.registers.l = result,
        RegTarget::HL => {
            unsafe { &mut *cpu.mmu }
                .write_byte(cpu.registers.get_hl(), result)
                .ok();
        }
        RegTarget::Immediate => (),
    }
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(new_carry);
    // ...existing code...
    if matches!(target, RegTarget::HL) {
        4
    } else {
        2
    }
}
