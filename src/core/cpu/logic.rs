use super::cpu::CPU;
use super::register_utils::{FlagOperations, RegTarget};
use crate::core::cycles::{CYCLES_1, CYCLES_2, CyclesType};
use crate::core::error::{Error, InstructionError};

impl CPU {
    // --- 算術/邏輯指令 ---
    pub fn or_a_n(&mut self) -> Result<CyclesType, Error> {
        let value = self.fetch_byte()?;
        self.registers.a |= value;
        self.update_logic_flags(self.registers.a, false);
        Ok(CYCLES_2)
    }

    pub fn xor_a_r(&mut self, reg: RegTarget) -> Result<CyclesType, Error> {
        let value: u8 = match reg {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.read_byte(addr)?
            }
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    reg as u8,
                )));
            }
        };

        self.registers.a ^= value;
        self.update_logic_flags(self.registers.a, false);
        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn xor_a_n(&mut self) -> Result<CyclesType, Error> {
        let value = self.fetch_byte()?;
        self.registers.a ^= value;
        self.update_logic_flags(self.registers.a, false);
        Ok(CYCLES_2)
    }

    pub fn cp_a_r(&mut self, reg: RegTarget) -> Result<CyclesType, Error> {
        let value: u8 = match reg {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.read_byte(addr)?
            }
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    reg as u8,
                )));
            }
        };

        self.cp_a(value);
        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn cp_a_n(&mut self) -> Result<CyclesType, Error> {
        let value = self.fetch_byte()?;
        self.cp_a(value);
        Ok(CYCLES_2)
    }

    fn cp_a(&mut self, value: u8) {
        let (result, borrow) = self.registers.a.overflowing_sub(value);
        let half_borrow = (self.registers.a & 0xF) < (value & 0xF);

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry(half_borrow);
        self.registers.set_carry(borrow);
    }

    fn update_logic_flags(&mut self, result: u8, half_carry: bool) {
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(half_carry);
        self.registers.set_carry(false);
    }
}

// --- 算術/邏輯指令 stub function ---
pub fn add_a_b(cpu: &mut CPU) -> CyclesType {
    let a = cpu.registers.a;
    let b = cpu.registers.b;
    let (result, carry) = a.overflowing_add(b);
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(((a & 0xF) + (b & 0xF)) > 0xF);
    cpu.registers.set_carry(carry);
    1
}

pub fn sub_a_c(cpu: &mut CPU) -> CyclesType {
    let a = cpu.registers.a;
    let c = cpu.registers.c;
    let (result, borrow) = a.overflowing_sub(c);
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry((a & 0xF) < (c & 0xF));
    cpu.registers.set_carry(borrow);
    1
}

pub fn and_a_d(cpu: &mut CPU) -> CyclesType {
    let result = cpu.registers.a & cpu.registers.d;
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(true);
    cpu.registers.set_carry(false);
    1
}

pub fn or_a_e(cpu: &mut CPU) -> CyclesType {
    let result = cpu.registers.a | cpu.registers.e;
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    1
}

pub fn xor_a_h(cpu: &mut CPU) -> CyclesType {
    let result = cpu.registers.a ^ cpu.registers.h;
    cpu.registers.a = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry(false);
    cpu.registers.set_carry(false);
    1
}

pub fn cp_a_l(cpu: &mut CPU) -> CyclesType {
    let a = cpu.registers.a;
    let l = cpu.registers.l;
    let (result, borrow) = a.overflowing_sub(l);
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry((a & 0xF) < (l & 0xF));
    cpu.registers.set_carry(borrow);
    1
}

pub fn inc_b(cpu: &mut CPU) -> CyclesType {
    let b = cpu.registers.b;
    let result = b.wrapping_add(1);
    cpu.registers.b = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(false);
    cpu.registers.set_half_carry((b & 0xF) + 1 > 0xF);
    1
}

pub fn dec_c(cpu: &mut CPU) -> CyclesType {
    let c = cpu.registers.c;
    let result = c.wrapping_sub(1);
    cpu.registers.c = result;
    cpu.registers.set_zero(result == 0);
    cpu.registers.set_subtract(true);
    cpu.registers.set_half_carry((c & 0xF) == 0);
    1
}

// AND 指令 (A0~A7)
pub fn and_a_r(cpu: &mut CPU, src: RegTarget) -> CyclesType {
    // TODO: 實作 AND A,r
    4
}

// OR 指令 (B0~B7)
pub fn or_a_r(cpu: &mut CPU, src: RegTarget) -> CyclesType {
    // TODO: 實作 OR A,r
    4
}

// XOR 指令 (A8~AF)
pub fn xor_a_r(cpu: &mut CPU, src: RegTarget) -> CyclesType {
    // TODO: 實作 XOR A,r
    4
}

// CP 指令 (B8~BF)
pub fn cp_a_r(cpu: &mut CPU, src: RegTarget) -> CyclesType {
    // TODO: 實作 CP A,r
    4
}

// CPL 指令 (2F)
pub fn cpl(cpu: &mut CPU) -> CyclesType {
    // TODO: 實作 CPL
    4
}

// SCF 指令 (37)
pub fn scf(cpu: &mut CPU) -> CyclesType {
    // TODO: 實作 SCF
    4
}

// CCF 指令 (3F)
pub fn ccf(cpu: &mut CPU) -> CyclesType {
    // TODO: 實作 CCF
    4
}

// DAA 指令 (27)
pub fn daa(cpu: &mut CPU) -> CyclesType {
    // TODO: 實作 DAA
    4
}

// --- 邏輯運算指令分派 ---
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> crate::core::error::Result<CyclesType> {
    // 自動補齊 stub，所有未覆蓋指令預設回傳 4 cycles
    Ok(4)
}
