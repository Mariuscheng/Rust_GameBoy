use crate::core::cpu::cpu::CPU;
use crate::core::cpu::flags::Flag;
use crate::core::cpu::register_utils::FlagOperations;
use crate::core::cpu::registers::Registers;
use crate::core::mmu::mmu::MMU;
use crate::core::utils::logger;

// 主流程指令解碼與執行
pub fn decode_and_execute(cpu: &mut CPU, mmu: &mut MMU, opcode: u8) {
    let registers = &mut cpu.registers;
    match opcode {
        // LD (a16),A
        0xEA => {
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            let a = registers.get_a();
            mmu.write_byte(addr, a).ok();
            registers.pc += 3;
        }
        // LD A,(a16)
        0xFA => {
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            let val = mmu.read_byte(addr).unwrap_or(0);
            registers.set_a(val);
            registers.pc += 3;
        }
        // LD (FF00+n),A
        0xE0 => {
            let n = mmu.memory[(registers.pc + 1) as usize];
            let addr = 0xFF00 | (n as u16);
            let a = registers.get_a();
            mmu.write_byte(addr, a).ok();
            registers.pc += 2;
        }
        // LD A,(FF00+n)
        0xF0 => {
            let n = mmu.memory[(registers.pc + 1) as usize];
            let addr = 0xFF00 | (n as u16);
            let val = mmu.read_byte(addr).unwrap_or(0);
            registers.set_a(val);
            registers.pc += 2;
        }
        // LD (a16),SP
        0x08 => {
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            let sp = registers.sp;
            mmu.write_byte(addr, (sp & 0xFF) as u8).ok();
            mmu.write_byte(addr + 1, (sp >> 8) as u8).ok();
            registers.pc += 3;
        }
        // LD SP,HL
        0xF9 => {
            registers.sp = registers.get_hl();
            registers.pc += 1;
        }
        // ADD HL,rr
        0x09 => {
            // ADD HL,BC
            let hl = registers.get_hl();
            let bc = registers.get_bc();
            let result = hl.wrapping_add(bc);
            // Flags: N=0, H = carry from bit11, C = carry from bit15, Z unaffected
            let h = ((hl & 0x0FFF) + (bc & 0x0FFF)) > 0x0FFF;
            let c = (hl as u32 + bc as u32) > 0xFFFF;
            registers.flags.set_subtract(false);
            registers.flags.set_half_carry(h);
            if c {
                registers.flags.set_carry(true);
            } else {
                registers.flags.set_carry(false);
            }
            registers.set_hl(result);
            registers.pc += 1;
        }
        0x19 => {
            // ADD HL,DE
            let hl = registers.get_hl();
            let de = registers.get_de();
            let result = hl.wrapping_add(de);
            let h = ((hl & 0x0FFF) + (de & 0x0FFF)) > 0x0FFF;
            let c = (hl as u32 + de as u32) > 0xFFFF;
            registers.flags.set_subtract(false);
            registers.flags.set_half_carry(h);
            registers.flags.set_carry(c);
            registers.set_hl(result);
            registers.pc += 1;
        }
        0x29 => {
            // ADD HL,HL
            let hl = registers.get_hl();
            let result = hl.wrapping_add(hl);
            let h = ((hl & 0x0FFF) + (hl & 0x0FFF)) > 0x0FFF;
            let c = (hl as u32 + hl as u32) > 0xFFFF;
            registers.flags.set_subtract(false);
            registers.flags.set_half_carry(h);
            registers.flags.set_carry(c);
            registers.set_hl(result);
            registers.pc += 1;
        }
        0x39 => {
            // ADD HL,SP
            let hl = registers.get_hl();
            let sp = registers.sp;
            let result = hl.wrapping_add(sp);
            let h = ((hl & 0x0FFF) + (sp & 0x0FFF)) > 0x0FFF;
            let c = (hl as u32 + sp as u32) > 0xFFFF;
            registers.flags.set_subtract(false);
            registers.flags.set_half_carry(h);
            registers.flags.set_carry(c);
            registers.set_hl(result);
            registers.pc += 1;
        }
        // 音效/背景/精靈/OAM/VRAM 相關指令 stub
        0xE2 => {
            /* LD (C),A (FF00+C) */
            let addr = 0xFF00 | (registers.get_c() as u16);
            let a = registers.get_a();
            mmu.write_byte(addr, a).ok();
            registers.pc += 1;
        }
        0xF2 => {
            /* LD A,(C) (FF00+C) */
            let addr = 0xFF00 | (registers.get_c() as u16);
            let val = mmu.read_byte(addr).unwrap_or(0);
            registers.set_a(val);
            registers.pc += 1;
        }
        // ...分支繼續...
        0xDA => {
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            if registers.get_flag(Flag::C) {
                registers.pc = addr;
            }
        }
        // CALL nn
        0xCD => {
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            let ret = registers.pc + 3;
            registers.sp = registers.sp.wrapping_sub(2);
            mmu.write_word(registers.sp, ret);
            registers.pc = addr;
        }
        // CALL cc,nn (條件呼叫)
        0xC4 => {
            // CALL NZ,nn
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            if !registers.get_flag(Flag::Z) {
                let ret = registers.pc + 3;
                registers.sp = registers.sp.wrapping_sub(2);
                mmu.write_word(registers.sp, ret);
                registers.pc = addr;
            }
        }
        0xCC => {
            // CALL Z,nn
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            if registers.get_flag(Flag::Z) {
                let ret = registers.pc + 3;
                registers.sp = registers.sp.wrapping_sub(2);
                mmu.write_word(registers.sp, ret);
                registers.pc = addr;
            }
        }
        0xD4 => {
            // CALL NC,nn
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            if !registers.get_flag(Flag::C) {
                let ret = registers.pc + 3;
                registers.sp = registers.sp.wrapping_sub(2);
                mmu.write_word(registers.sp, ret);
                registers.pc = addr;
            }
        }
        0xDC => {
            // CALL C,nn
            let lo = mmu.memory[(registers.pc + 1) as usize];
            let hi = mmu.memory[(registers.pc + 2) as usize];
            let addr = ((hi as u16) << 8) | (lo as u16);
            if registers.get_flag(Flag::C) {
                let ret = registers.pc + 3;
                registers.sp = registers.sp.wrapping_sub(2);
                mmu.write_word(registers.sp, ret);
                registers.pc = addr;
            }
        }
        // RET
        0xC9 => {
            let ret = mmu.read_word(registers.sp);
            registers.sp = registers.sp.wrapping_add(2);
            registers.pc = ret;
        }
        // PUSH qq
        0xC5 => {
            // PUSH BC
            registers.sp = registers.sp.wrapping_sub(2);
            mmu.write_word(registers.sp, registers.get_bc());
        }
        0xD5 => {
            // PUSH DE
            registers.sp = registers.sp.wrapping_sub(2);
            mmu.write_word(registers.sp, registers.get_de());
        }
        0xE5 => {
            // PUSH HL
            registers.sp = registers.sp.wrapping_sub(2);
            mmu.write_word(registers.sp, registers.get_hl());
        }
        0xF5 => {
            // PUSH AF
            registers.sp = registers.sp.wrapping_sub(2);
            mmu.write_word(registers.sp, registers.get_af());
        }
        // POP qq
        0xC1 => {
            // POP BC
            let value = mmu.read_word(registers.sp);
            registers.set_bc(value);
            registers.sp = registers.sp.wrapping_add(2);
        }
        0xD1 => {
            // POP DE
            let value = mmu.read_word(registers.sp);
            registers.set_de(value);
            registers.sp = registers.sp.wrapping_add(2);
        }
        0xE1 => {
            // POP HL
            let value = mmu.read_word(registers.sp);
            registers.set_hl(value);
            registers.sp = registers.sp.wrapping_add(2);
        }
        0xF1 => {
            // POP AF
            let value = mmu.read_word(registers.sp);
            registers.set_af(value);
            registers.sp = registers.sp.wrapping_add(2);
        }
        // ADD A,r
        0x80..=0x87 => {
            let src = opcode & 0x07;
            let a = registers.get_a();
            let value = registers.get_by_index(src);
            let (result, carry) = a.overflowing_add(value);
            let half_carry = ((a & 0xF) + (value & 0xF)) > 0xF;
            registers.set_a(result);
            registers.update_flags(result == 0, false, half_carry, carry);
        }
        // ADD A,n
        0xC6 => {
            let value = mmu.memory[(registers.pc + 1) as usize];
            let a = registers.get_a();
            let (result, carry) = a.overflowing_add(value);
            let half_carry = ((a & 0xF) + (value & 0xF)) > 0xF;
            registers.set_a(result);
            registers.update_flags(result == 0, false, half_carry, carry);
        }
        // SUB r
        0x90..=0x97 => {
            let src = opcode & 0x07;
            let a = registers.get_a();
            let value = registers.get_by_index(src);
            let (result, borrow) = a.overflowing_sub(value);
            let half_carry = (a & 0xF) < (value & 0xF);
            registers.set_a(result);
            registers.update_flags(result == 0, true, half_carry, borrow);
        }
        // SUB n
        0xD6 => {
            let value = mmu.memory[(registers.pc + 1) as usize];
            let a = registers.get_a();
            let (result, borrow) = a.overflowing_sub(value);
            let half_carry = (a & 0xF) < (value & 0xF);
            registers.set_a(result);
            registers.update_flags(result == 0, true, half_carry, borrow);
        }
        // AND r
        0xA0..=0xA7 => {
            let src = opcode & 0x07;
            let a = registers.get_a();
            let value = registers.get_by_index(src);
            let result = a & value;
            registers.set_a(result);
            registers.update_flags(result == 0, false, true, false);
        }
        // AND n
        0xE6 => {
            let value = mmu.memory[(registers.pc + 1) as usize];
            let a = registers.get_a();
            let result = a & value;
            registers.set_a(result);
            registers.update_flags(result == 0, false, true, false);
        }
        // OR r
        0xB0..=0xB7 => {
            let src = opcode & 0x07;
            let a = registers.get_a();
            let value = registers.get_by_index(src);
            let result = a | value;
            registers.set_a(result);
            registers.update_flags(result == 0, false, false, false);
        }
        // OR n
        0xF6 => {
            let value = mmu.memory[(registers.pc + 1) as usize];
            let a = registers.get_a();
            let result = a | value;
            registers.set_a(result);
            registers.update_flags(result == 0, false, false, false);
        }
        // XOR r
        0xA8..=0xAF => {
            let src = opcode & 0x07;
            let a = registers.get_a();
            let value = registers.get_by_index(src);
            let result = a ^ value;
            registers.set_a(result);
            registers.update_flags(result == 0, false, false, false);
        }
        // XOR n
        0xEE => {
            let value = mmu.memory[(registers.pc + 1) as usize];
            let a = registers.get_a();
            let result = a ^ value;
            registers.set_a(result);
            registers.update_flags(result == 0, false, false, false);
        }
        // CP r
        0xB8..=0xBF => {
            let src = opcode & 0x07;
            let a = registers.get_a();
            let value = registers.get_by_index(src);
            let (result, borrow) = a.overflowing_sub(value);
            let half_carry = (a & 0xF) < (value & 0xF);
            registers.update_flags(result == 0, true, half_carry, borrow);
        }
        // CP n
        0xFE => {
            let value = mmu.memory[(registers.pc + 1) as usize];
            let a = registers.get_a();
            let (result, borrow) = a.overflowing_sub(value);
            let half_carry = (a & 0xF) < (value & 0xF);
            registers.update_flags(result == 0, true, half_carry, borrow);
        }
        // 其他未實作指令 stub
        _ => {
            logger::log_to_file(&format!(
                "[UNIMPLEMENTED OPCODE] opcode=0x{:02X} PC=0x{:04X} A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} F={:02X} H={:02X} L={:02X} SP={:04X}",
                opcode,
                registers.pc,
                registers.a,
                registers.b,
                registers.c,
                registers.d,
                registers.e,
                registers.get_f(),
                registers.h,
                registers.l,
                registers.sp
            ));
        }
    }
}

// CB 指令輔助函式，可由 CPU::execute 呼叫
pub fn decode_and_execute_cb(registers: &mut Registers, mmu: &mut MMU, cb_opcode: u8) {
    let reg_idx = cb_opcode & 0x07;
    let bit_idx = (cb_opcode >> 3) & 0x07;
    // 修正 match arms 型別，所有分支都回傳 u8，或預設 0
    match cb_opcode {
        // RLC r
        0x00..=0x07 => {
            let val = registers.get_by_index(reg_idx);
            let carry = (val & 0x80) != 0;
            let res = (val << 1) | if carry { 1 } else { 0 };
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // RLC (HL)
        0x06 => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let carry = (val & 0x80) != 0;
            let res = (val << 1) | if carry { 1 } else { 0 };
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // RRC r
        0x08..=0x0F => {
            let val = registers.get_by_index(reg_idx);
            let carry = (val & 0x01) != 0;
            let res = (val >> 1) | if carry { 0x80 } else { 0 };
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // RRC (HL)
        0x0E => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let carry = (val & 0x01) != 0;
            let res = (val >> 1) | if carry { 0x80 } else { 0 };
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // RL r
        0x10..=0x17 => {
            let val = registers.get_by_index(reg_idx);
            let old_carry = registers.get_flag(crate::core::cpu::flags::Flag::C);
            let carry = (val & 0x80) != 0;
            let res = (val << 1) | if old_carry { 1 } else { 0 };
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // RL (HL)
        0x16 => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let old_carry = registers.get_flag(crate::core::cpu::flags::Flag::C);
            let carry = (val & 0x80) != 0;
            let res = (val << 1) | if old_carry { 1 } else { 0 };
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // RR r
        0x18..=0x1F => {
            let val = registers.get_by_index(reg_idx);
            let old_carry = registers.get_flag(crate::core::cpu::flags::Flag::C);
            let carry = (val & 0x01) != 0;
            let res = (val >> 1) | if old_carry { 0x80 } else { 0 };
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // RR (HL)
        0x1E => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let old_carry = registers.get_flag(crate::core::cpu::flags::Flag::C);
            let carry = (val & 0x01) != 0;
            let res = (val >> 1) | if old_carry { 0x80 } else { 0 };
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // SLA r
        0x20..=0x27 => {
            let val = registers.get_by_index(reg_idx);
            let carry = (val & 0x80) != 0;
            let res = val << 1;
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // SLA (HL)
        0x26 => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let carry = (val & 0x80) != 0;
            let res = val << 1;
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // SRA r
        0x28..=0x2F => {
            let val = registers.get_by_index(reg_idx);
            let carry = (val & 0x01) != 0;
            let res = (val >> 1) | (val & 0x80);
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // SRA (HL)
        0x2E => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let carry = (val & 0x01) != 0;
            let res = (val >> 1) | (val & 0x80);
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // SWAP r
        0x30..=0x37 => {
            let val = registers.get_by_index(reg_idx);
            let res = (val >> 4) | (val << 4);
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, false);
        }
        // SWAP (HL)
        0x36 => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let res = (val >> 4) | (val << 4);
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, false);
        }
        // SRL r
        0x38..=0x3F => {
            let val = registers.get_by_index(reg_idx);
            let carry = (val & 0x01) != 0;
            let res = val >> 1;
            registers.set_by_index(reg_idx, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // SRL (HL)
        0x3E => {
            let hl = registers.get_hl();
            let val = mmu.read_byte(hl).unwrap_or(0);
            let carry = (val & 0x01) != 0;
            let res = val >> 1;
            let _ = mmu.write_byte(hl, res);
            registers.update_flags(res == 0, false, false, carry);
        }
        // BIT b,r
        0x40..=0x7F => {
            let bit = (cb_opcode >> 3) & 0x07;
            let val = if reg_idx == 6 {
                let hl = registers.get_hl();
                mmu.read_byte(hl).unwrap_or(0)
            } else {
                registers.get_by_index(reg_idx)
            };
            let z = (val & (1 << bit)) == 0;
            registers.update_flags(
                z,
                false,
                true,
                registers.get_flag(crate::core::cpu::flags::Flag::C),
            );
        }
        // RES b,r
        0x80..=0xBF => {
            let bit = (cb_opcode >> 3) & 0x07;
            if reg_idx == 6 {
                let hl = registers.get_hl();
                let val = mmu.read_byte(hl).unwrap_or(0);
                let res = val & !(1 << bit);
                let _ = mmu.write_byte(hl, res);
            } else {
                let val = registers.get_by_index(reg_idx);
                let res = val & !(1 << bit);
                registers.set_by_index(reg_idx, res);
            }
        }
        // SET b,r
        0xC0..=0xFF => {
            let bit = (cb_opcode >> 3) & 0x07;
            if reg_idx == 6 {
                let hl = registers.get_hl();
                let val = mmu.read_byte(hl).unwrap_or(0);
                let res = val | (1 << bit);
                let _ = mmu.write_byte(hl, res);
            } else {
                let val = registers.get_by_index(reg_idx);
                let res = val | (1 << bit);
                registers.set_by_index(reg_idx, res);
            }
        }
        _ => {
            logger::log_to_file(&format!(
                "[UNIMPLEMENTED OPCODE] cb_opcode=0x{:02X} PC=0x{:04X} A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} F={:02X} H={:02X} L={:02X} SP={:04X}",
                cb_opcode,
                registers.pc,
                registers.a,
                registers.b,
                registers.c,
                registers.d,
                registers.e,
                registers.get_f(),
                registers.h,
                registers.l,
                registers.sp
            ));
        }
    }

    // 以下為輔助函式定義，全部移出主流程 match block之外
    fn vram_write_bc(registers: &Registers, mmu: &mut MMU) {
        let addr = registers.get_bc();
        let value = registers.a;
        let _ = mmu.write_byte(addr, value);
        if (0x8000..=0x9FFF).contains(&addr) {
            logger::log_to_file(&format!(
                "[CPU_DECODE][VRAM_WRITE] LD (BC={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            logger::log_to_file(&format!(
                "[CPU_DECODE][OAM_WRITE] LD (BC={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        }
        if addr == 0xFF46 {
            logger::log_to_file(&format!(
                "[CPU_OAM_DMA] LD (0xFF46),A={:#04X}, PC={:#06X}",
                value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            logger::log_to_file(&format!(
                "[CPU_OAM_WRITE] LD (OAM={:#06X}),A={:#04X}, PC={:#06X}",
                addr, value, registers.pc
            ));
        }
        if (0x8000..=0x9FFF).contains(&addr) {
            logger::log_to_file(&format!("[VRAM] LD (BC={:#06X}),A={:#04X}", addr, value));
        }
    }
    fn vram_write_de(registers: &Registers, mmu: &mut MMU) {
        let addr = registers.get_de();
        let value = registers.a;
        let _ = mmu.write_byte(addr, value);
        if (0x8000..=0x9FFF).contains(&addr) {
            logger::log_to_file(&format!(
                "[CPU_DECODE][VRAM_WRITE] LD (DE={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            logger::log_to_file(&format!(
                "[CPU_DECODE][OAM_WRITE] LD (DE={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        }
        if addr == 0xFF46 {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_DMA] LD (0xFF46),A={:#04X}, PC={:#06X}",
                value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_WRITE] LD (OAM={:#06X}),A={:#04X}, PC={:#06X}",
                addr, value, registers.pc
            ));
        }
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[VRAM] LD (DE={:#06X}),A={:#04X}",
                addr, value
            ));
        }
    }
    fn vram_write_hl_inc(registers: &mut Registers, mmu: &mut MMU) {
        let addr = registers.get_hl();
        let value = registers.a;
        let _ = mmu.write_byte(addr, value);
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][VRAM_WRITE] LD (HL+={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][OAM_WRITE] LD (HL+={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        }
        if addr == 0xFF46 {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_DMA] LD (0xFF46),A={:#04X}, PC={:#06X}",
                value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_WRITE] LD (OAM={:#06X}),A={:#04X}, PC={:#06X}",
                addr, value, registers.pc
            ));
        }
        registers.set_hl(addr.wrapping_add(1));
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[VRAM] LD (HL+={:#06X}),A={:#04X}",
                addr, value
            ));
        }
    }
    fn vram_write_hl_dec(registers: &mut Registers, mmu: &mut MMU) {
        let addr = registers.get_hl();
        let value = registers.a;
        let _ = mmu.write_byte(addr, value);
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][VRAM_WRITE] LD (HL-={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][OAM_WRITE] LD (HL-={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        }
        if addr == 0xFF46 {
            logger::log_to_file(&format!(
                "[CPU_OAM_DMA] LD (0xFF46),A={:#04X}, PC={:#06X}",
                value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            logger::log_to_file(&format!(
                "[CPU_OAM_WRITE] LD (OAM={:#06X}),A={:#04X}, PC={:#06X}",
                addr, value, registers.pc
            ));
        }
        registers.set_hl(addr.wrapping_sub(1));
        if (0x8000..=0x9FFF).contains(&addr) {
            logger::log_to_file(&format!("[VRAM] LD (HL-={:#06X}),A={:#04X}", addr, value));
        }
    }
    fn vram_write_hl_n(registers: &Registers, mmu: &mut MMU) {
        let addr = registers.get_hl();
        let n = mmu.memory[(registers.pc + 1) as usize];
        let _ = mmu.write_byte(addr, n);
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][VRAM_WRITE] LD (HL={:04X}),n={:02X} PC={:04X}",
                addr, n, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][OAM_WRITE] LD (HL={:04X}),n={:02X} PC={:04X}",
                addr, n, registers.pc
            ));
        }
        if addr == 0xFF46 {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_DMA] LD (0xFF46),n={:#04X}, PC={:#06X}",
                n, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_WRITE] LD (OAM={:#06X}),n={:#04X}, PC={:#06X}",
                addr, n, registers.pc
            ));
        }
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[VRAM] LD (HL={:#06X}),n={:#04X}",
                addr, n
            ));
        }
    }
    fn vram_write_nn_a(registers: &Registers, mmu: &mut MMU) {
        let lo = mmu.memory[(registers.pc + 1) as usize];
        let hi = mmu.memory[(registers.pc + 2) as usize];
        let addr = ((hi as u16) << 8) | (lo as u16);
        let value = registers.a;
        let _ = mmu.write_byte(addr, value);
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][VRAM_WRITE] LD (nn={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_DECODE][OAM_WRITE] LD (nn={:04X}),A={:02X} PC={:04X}",
                addr, value, registers.pc
            ));
        }
        if addr == 0xFF46 {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_DMA] LD (0xFF46),A={:#04X}, PC={:#06X}",
                value, registers.pc
            ));
        } else if (0xFE00..=0xFE9F).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[CPU_OAM_WRITE] LD (OAM={:#06X}),A={:#04X}, PC={:#06X}",
                addr, value, registers.pc
            ));
        }
        if (0x8000..=0x9FFF).contains(&addr) {
            crate::core::utils::logger::log_to_file(&format!(
                "[VRAM] LD (nn={:#06X}),A={:#04X}",
                addr, value
            ));
        }
    }
}
