/// DEC r: 指定暫存器減一
pub fn dec_r(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    match target {
        RegTarget::A => cpu.registers.a = cpu.registers.a.wrapping_sub(1),
        RegTarget::B => cpu.registers.b = cpu.registers.b.wrapping_sub(1),
        RegTarget::C => cpu.registers.c = cpu.registers.c.wrapping_sub(1),
        RegTarget::D => cpu.registers.d = cpu.registers.d.wrapping_sub(1),
        RegTarget::E => cpu.registers.e = cpu.registers.e.wrapping_sub(1),
        RegTarget::H => cpu.registers.h = cpu.registers.h.wrapping_sub(1),
        RegTarget::L => cpu.registers.l = cpu.registers.l.wrapping_sub(1),
        RegTarget::HL => {
            let addr = cpu.registers.get_hl();
            let val = unsafe { &*cpu.mmu }
                .read_byte(addr)
                .unwrap_or(0)
                .wrapping_sub(1);
            let _ = unsafe { &mut *cpu.mmu }.write_byte(addr, val);
        }
        _ => {}
    }
    4
}

/// ADD A, r: A += 指定暫存器
pub fn add_a_r(cpu: &mut CPU, target: RegTarget, _use_carry: bool) -> CyclesType {
    let value = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => {
            let addr = cpu.registers.get_hl();
            unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
        }
        _ => 0,
    };
    cpu.registers.a = cpu.registers.a.wrapping_add(value);
    4
}

/// ADC A, r: A += r + Carry
pub fn adc_a_r(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    let carry = if cpu.registers.carry() { 1 } else { 0 };
    let value = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => {
            let addr = cpu.registers.get_hl();
            unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
        }
        _ => 0,
    };
    let a = cpu.registers.a;
    let result = a.wrapping_add(value).wrapping_add(carry);
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers
        .set_half_carry(((a & 0x0F) + (value & 0x0F) + carry) > 0x0F);
    cpu.registers
        .set_carry((a as u16 + value as u16 + carry as u16) > 0xFF);
    4
}

/// SBC A, r: A -= r + Carry
pub fn sbc_a_r(cpu: &mut CPU, target: RegTarget) -> CyclesType {
    let carry = if cpu.registers.carry() { 1 } else { 0 };
    let value = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => {
            let addr = cpu.registers.get_hl();
            unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
        }
        _ => 0,
    };
    let a = cpu.registers.a;
    let result = a.wrapping_sub(value).wrapping_sub(carry);
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(true);
    cpu.registers
        .set_half_carry((a & 0x0F) < ((value & 0x0F) + carry));
    cpu.registers
        .set_carry((a as u16) < (value as u16 + carry as u16));
    4
}

/// DAA: Decimal adjust A after BCD addition/subtraction
pub fn daa(cpu: &mut CPU) -> CyclesType {
    let mut a = cpu.registers.a;
    let mut c = cpu.registers.carry();
    let n = cpu.registers.subtract();
    let h = cpu.registers.half_carry();
    let mut adjust = 0;
    if !n {
        if c || a > 0x99 {
            adjust |= 0x60;
            c = true;
        }
        if h || (a & 0x0F) > 0x09 {
            adjust |= 0x06;
        }
        a = a.wrapping_add(adjust);
    } else {
        if c {
            adjust |= 0x60;
        }
        if h {
            adjust |= 0x06;
        }
        a = a.wrapping_sub(adjust);
    }
    cpu.registers.a = a;
    cpu.registers.set_zero(a == 0);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(c);
    4
}
/// SUB A, r: A -= 指定暫存器
pub fn sub_a_r(cpu: &mut CPU, target: RegTarget, _use_carry: bool) -> CyclesType {
    let value = match target {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => {
            let addr = cpu.registers.get_hl();
            unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
        }
        _ => 0,
    };
    cpu.registers.a = cpu.registers.a.wrapping_sub(value);
    4
}

/// ADD A, n: A += 立即值
pub fn add_a_n(cpu: &mut CPU, _use_carry: bool) -> CyclesType {
    let n = cpu.fetch_byte().unwrap_or(0);
    let a = cpu.registers.a;
    let result = a.wrapping_add(n);
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers
        .set_half_carry(((a & 0x0F) + (n & 0x0F)) > 0x0F);
    cpu.registers.set_carry((a as u16 + n as u16) > 0xFF);
    4
}

/// SUB A, n: A -= 立即值
pub fn sub_a_n(cpu: &mut CPU, _use_carry: bool) -> CyclesType {
    let n = cpu.fetch_byte().unwrap_or(0);
    let a = cpu.registers.a;
    let result = a.wrapping_sub(n);
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry((a & 0x0F) < (n & 0x0F));
    cpu.registers.set_carry(a < n);
    4
}
use super::cpu::CPU;
use super::register_utils::RegTarget;
use crate::core::cpu::register_utils::FlagOperations;
use crate::core::cycles::CyclesType;
use crate::core::error::{Error, InstructionError, Result};
use crate::core::utils::logger::log_to_file;

/// 算術運算指令分派
#[allow(dead_code)]
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> crate::core::error::Result<CyclesType> {
    log_to_file(&format!(
        "[ARITHMETIC] dispatch: opcode={:02X}, PC={:04X}",
        opcode, cpu.registers.pc
    ));
    // ADD A, r (80~87)
    if (opcode & 0xF8) == 0x80 {
        let src = opcode & 0x07;
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        let a = cpu.registers.a;
        let result = a.wrapping_add(value);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(false);
        cpu.registers
            .set_half_carry(((a & 0x0F) + (value & 0x0F)) > 0x0F);
        cpu.registers.set_carry((a as u16 + value as u16) > 0xFF);
        return Ok(4);
    }
    // 其餘指令分支...
    // SUB A, r (90~97)
    if (opcode & 0xF8) == 0x90 {
        let src = opcode & 0x07;
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        let a = cpu.registers.a;
        let result = a.wrapping_sub(value);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        cpu.registers.set_carry(a < value);
        return Ok(4);
    }
    // AND A, r (A0~A7)
    if (opcode & 0xF8) == 0xA0 {
        let src = opcode & 0x07;
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        cpu.registers.a &= value;
        cpu.registers.set_zero(cpu.registers.a == 0);
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(false);
        return Ok(4);
    }
    // OR A, r (B0~B7)
    if (opcode & 0xF8) == 0xB0 {
        let src = opcode & 0x07;
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        cpu.registers.a |= value;
        cpu.registers.set_zero(cpu.registers.a == 0);
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(false);
        cpu.registers.set_carry(false);
        return Ok(4);
    }
    // XOR A, r (A8~AF)
    if (opcode & 0xF8) == 0xA8 {
        let src = opcode & 0x07;
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        cpu.registers.a ^= value;
        cpu.registers.set_zero(cpu.registers.a == 0);
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(false);
        cpu.registers.set_carry(false);
        return Ok(4);
    }
    // CP A, r (B8~BF)
    if (opcode & 0xF8) == 0xB8 {
        let src = opcode & 0x07;
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        let a = cpu.registers.a;
        let result = a.wrapping_sub(value);
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        cpu.registers.set_carry(a < value);
        return Ok(4);
    }
    // ADC A, r (88~8F)
    if (opcode & 0xF8) == 0x88 {
        let src = opcode & 0x07;
        let carry = if cpu.registers.carry() { 1 } else { 0 };
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        let a = cpu.registers.a;
        let result = a.wrapping_add(value).wrapping_add(carry);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(false);
        cpu.registers
            .set_half_carry(((a & 0x0F) + (value & 0x0F) + carry) > 0x0F);
        cpu.registers
            .set_carry((a as u16 + value as u16 + carry as u16) > 0xFF);
        let cycles = if src == 6 { 8 } else { 4 };
        return Ok(cycles);
    }
    // SBC A, r (98~9F)
    if (opcode & 0xF8) == 0x98 {
        let src = opcode & 0x07;
        let carry = if cpu.registers.carry() { 1 } else { 0 };
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        let a = cpu.registers.a;
        let result = a.wrapping_sub(value).wrapping_sub(carry);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(true);
        cpu.registers
            .set_half_carry(((a ^ value ^ result) & 0x10) != 0);
        cpu.registers
            .set_carry((a as u16) < (value as u16 + carry as u16));
        let cycles = if src == 6 { 8 } else { 4 };
        return Ok(cycles);
    }
    // DAA (27)
    if opcode == 0x27 {
        daa(cpu);
        return Ok(4);
    }
    // CPL (2F)
    if opcode == 0x2F {
        cpu.registers.a = !cpu.registers.a;
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry(true);
        // Z, C preserved
        return Ok(4);
    }
    // SCF (37)
    if opcode == 0x37 {
        cpu.registers.set_carry(true);
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(false);
        // Z preserved
        return Ok(4);
    }
    // CCF (3F)
    if opcode == 0x3F {
        let carry = cpu.registers.carry();
        cpu.registers.set_carry(!carry);
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(false);
        // Z preserved
        return Ok(4);
    }
    // ADD A, n (C6)
    if opcode == 0xC6 {
        let value = cpu.memory[(cpu.registers.pc + 1) as usize];
        cpu.registers.pc += 1;
        let a = cpu.registers.a;
        let result = a.wrapping_add(value);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(false);
        cpu.registers
            .set_half_carry(((a & 0x0F) + (value & 0x0F)) > 0x0F);
        cpu.registers.set_carry((a as u16 + value as u16) > 0xFF);
        return Ok(8);
    }
    // ADC A, n (CE)
    if opcode == 0xCE {
        let value = cpu.memory[(cpu.registers.pc + 1) as usize];
        cpu.registers.pc += 1;
        let carry = if cpu.registers.carry() { 1 } else { 0 };
        let a = cpu.registers.a;
        let result = a.wrapping_add(value).wrapping_add(carry);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(false);
        cpu.registers
            .set_half_carry(((a ^ value ^ result) & 0x10) != 0);
        cpu.registers
            .set_carry((a as u16 + value as u16 + carry as u16) > 0xFF);
        return Ok(8);
    }
    // SUB A, n (D6)
    if opcode == 0xD6 {
        let value = cpu.memory[(cpu.registers.pc + 1) as usize];
        cpu.registers.pc += 1;
        let a = cpu.registers.a;
        let result = a.wrapping_sub(value);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(true);
        cpu.registers.set_half_carry((a & 0x0F) < (value & 0x0F));
        cpu.registers.set_carry(a < value);
        return Ok(8);
    }
    // SBC A, n (DE)
    if opcode == 0xDE {
        let value = cpu.memory[(cpu.registers.pc + 1) as usize];
        cpu.registers.pc += 1;
        let carry = if cpu.registers.carry() { 1 } else { 0 };
        let a = cpu.registers.a;
        let result = a.wrapping_sub(value).wrapping_sub(carry);
        cpu.registers.a = result;
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(true);
        cpu.registers
            .set_half_carry((a & 0x0F) < ((value & 0x0F) + carry));
        cpu.registers
            .set_carry((a as u16) < (value as u16 + carry as u16));
        return Ok(8);
    }
    // AND A, n (E6)
    if opcode == 0xE6 {
        let value = cpu.memory[(cpu.registers.pc + 1) as usize];
        cpu.registers.pc += 1;
        cpu.registers.a &= value;
        cpu.registers.set_zero(cpu.registers.a == 0);
        cpu.registers.set_subtract(false);
        cpu.registers.set_half_carry(true);
        cpu.registers.set_carry(false);
        return Ok(8);
    }
    // CP A, r (B8~BF)
    if (opcode & 0xF8) == 0xB8 {
        let src = opcode & 0x07;
        let value = match src {
            0 => cpu.registers.b,
            1 => cpu.registers.c,
            2 => cpu.registers.d,
            3 => cpu.registers.e,
            4 => cpu.registers.h,
            5 => cpu.registers.l,
            6 => {
                let addr = cpu.registers.get_hl();
                unsafe { &*cpu.mmu }.read_byte(addr).unwrap_or(0)
            }
            7 => cpu.registers.a,
            _ => 0,
        };
        let a = cpu.registers.a;
        let result = a.wrapping_sub(value);
        cpu.registers.set_zero(result == 0);
        cpu.registers.set_subtract(true);
        cpu.registers
            .set_half_carry(((a ^ value ^ result) & 0x10) != 0);
        cpu.registers.set_carry(a < value);
        return Ok(4);
        // 其餘指令分支...
    }
}
