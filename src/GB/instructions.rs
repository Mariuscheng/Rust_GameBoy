// -------- Flag/CPU control: DAA, CPL, SCF, CCF, EI, DI, STOP --------
fn ei_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // EI enables IME after the NEXT instruction completes (delayed enable)
    cpu.ime_enable_armed = true;
    4
}

fn di_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    cpu.ime = false;
    // Cancel any pending EI delayed enable
    cpu.ime_enable_armed = false;
    cpu.ime_enable_pending = false;
    4
}

fn ccf_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let mut flags = cpu.registers.flags();
    // Preserve Z, clear N and H, toggle C
    let z = flags.contains(Flags::Z);
    let c = flags.contains(Flags::C);
    flags.remove(Flags::N | Flags::H | Flags::C);
    if z {
        flags.insert(Flags::Z);
    }
    if !c {
        flags.insert(Flags::C);
    }
    cpu.registers.set_flags(flags);
    4
}

fn scf_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let mut flags = cpu.registers.flags();
    let z = flags.contains(Flags::Z);
    flags.remove(Flags::N | Flags::H | Flags::C);
    if z {
        flags.insert(Flags::Z);
    }
    flags.insert(Flags::C);
    cpu.registers.set_flags(flags);
    4
}

fn cpl_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let a = cpu.registers.get_a();
    cpu.registers.set_a(!a);
    let mut flags = cpu.registers.flags();
    flags.insert(Flags::N | Flags::H); // set N and H; preserve Z and C implicitly
    cpu.registers.set_flags(flags);
    4
}

fn daa_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let mut a = cpu.registers.get_a() as u16;
    let mut flags = cpu.registers.flags();
    let n = flags.contains(Flags::N);
    let h = flags.contains(Flags::H);
    let c = flags.contains(Flags::C);

    let mut carry = c;
    if !n {
        if c || a > 0x99 {
            a = a.wrapping_add(0x60);
            carry = true;
        }
        if h || (a & 0x0F) > 0x09 {
            a = a.wrapping_add(0x06);
        }
    } else {
        if c {
            a = a.wrapping_sub(0x60);
        }
        if h {
            a = a.wrapping_sub(0x06);
        }
    }

    let a8 = (a & 0xFF) as u8;
    cpu.registers.set_a(a8);
    flags.set(Flags::Z, a8 == 0);
    flags.set(Flags::N, n); // preserve N
    flags.remove(Flags::H); // H cleared after DAA
    flags.set(Flags::C, carry);
    cpu.registers.set_flags(flags);
    4
}

fn stop_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // STOP is 2-byte instruction (0x10 0x00). Consume next byte and no-op.
    let _ = cpu.fetch_next();
    4
}

static EI: Instruction =
    Instruction { opcode: 0xFB, name: "EI", cycles: 4, size: 1, flags: &[], execute: ei_exec };
static DI: Instruction =
    Instruction { opcode: 0xF3, name: "DI", cycles: 4, size: 1, flags: &[], execute: di_exec };
static CCF: Instruction =
    Instruction { opcode: 0x3F, name: "CCF", cycles: 4, size: 1, flags: &[], execute: ccf_exec };
static SCF: Instruction =
    Instruction { opcode: 0x37, name: "SCF", cycles: 4, size: 1, flags: &[], execute: scf_exec };
static CPL: Instruction =
    Instruction { opcode: 0x2F, name: "CPL", cycles: 4, size: 1, flags: &[], execute: cpl_exec };
static DAAI: Instruction =
    Instruction { opcode: 0x27, name: "DAA", cycles: 4, size: 1, flags: &[], execute: daa_exec };
static STOP: Instruction =
    Instruction { opcode: 0x10, name: "STOP", cycles: 4, size: 2, flags: &[], execute: stop_exec };
// 指令結構
#[allow(dead_code)]
pub struct Instruction {
    pub opcode: u8,
    pub name: &'static str,
    pub cycles: u8,
    pub size: u8,
    pub flags: &'static [FlagBits],
    pub execute: fn(&Instruction, &mut crate::GB::CPU::CPU) -> u64,
}

// 旗標 enum
#[allow(dead_code)]
pub enum FlagBits {
    Z = 0b1000_0000,
    N = 0b0100_0000,
    H = 0b0010_0000,
    C = 0b0001_0000,
}

// -------- Helpers for register indexing (B,C,D,E,H,L,(HL),A) --------
#[inline]
fn read_r(cpu: &mut crate::GB::CPU::CPU, idx: u8) -> u8 {
    match idx & 0x07 {
        0 => cpu.registers.get_b(),
        1 => cpu.registers.get_c(),
        2 => cpu.registers.get_d(),
        3 => cpu.registers.get_e(),
        4 => cpu.registers.get_h(),
        5 => cpu.registers.get_l(),
        6 => {
            let hl = cpu.registers.get_hl();
            cpu.read8(hl)
        }
        7 => cpu.registers.get_a(),
        _ => unreachable!(),
    }
}

#[inline]
fn write_r(cpu: &mut crate::GB::CPU::CPU, idx: u8, val: u8) {
    match idx & 0x07 {
        0 => cpu.registers.set_b(val),
        1 => cpu.registers.set_c(val),
        2 => cpu.registers.set_d(val),
        3 => cpu.registers.set_e(val),
        4 => cpu.registers.set_h(val),
        5 => cpu.registers.set_l(val),
        6 => {
            let hl = cpu.registers.get_hl();
            cpu.write8(hl, val);
        }
        7 => cpu.registers.set_a(val),
        _ => unreachable!(),
    }
}

// 專用：在讀 r 時，若 r==(HL) 必須先步進 4 cycles 再讀記憶體，
// 以符合 mem_timing 對於記憶體取用時序的要求。
#[inline]
fn read_r_timed(cpu: &mut crate::GB::CPU::CPU, idx: u8) -> u8 {
    match idx & 0x07 {
        6 => {
            let hl = cpu.registers.get_hl();
            cpu.step_mem(4); // step BEFORE the memory read
            cpu.read8(hl)
        }
        _ => read_r(cpu, idx),
    }
}

// 指令執行範例
fn nop_exec(_instr: &Instruction, _cpu: &mut crate::GB::CPU::CPU) -> u64 {
    4
}

// Generic LD r,r' executor (for opcodes 0x40..=0x7F except 0x76)
fn ld_r_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let op = cpu.opcode;
    // Bits: dst = (op >> 3) & 0b111, src = op & 0b111; mapping 000=B,001=C,010=D,011=E,100=H,101=L,110=(HL),111=A
    let dst = (op >> 3) & 0x07;
    let src = op & 0x07;
    let val = read_r_timed(cpu, src);
    if dst == 6 {
        cpu.step_mem(4);
    } // step before memory write timing
    write_r(cpu, dst, val);
    if src == 6 || dst == 6 { 8 } else { 4 }
}

// LD A, n 指令執行範例
fn ld_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let val = cpu.read_u8_imm();
    cpu.registers.set_a(val);
    // Flags unaffected
    8
}

// INC A 指令執行範例
fn inc_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let a = cpu.registers.get_a();
    let res = a.wrapping_add(1);
    // Flags: Z set if zero, N=0, H set on half-carry, C unchanged
    use crate::GB::registers::Flags;
    let mut flags = cpu.registers.flags();
    flags.set(Flags::Z, res == 0);
    flags.remove(Flags::N);
    let h = (a & 0x0F) + 1 > 0x0F;
    flags.set(Flags::H, h);
    cpu.registers.set_a(res);
    cpu.registers.set_flags(flags);
    4
}

// XOR A 指令執行範例
fn xor_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let res = 0; // A XOR A = 0
    cpu.registers.set_a(res);
    // Z set, N H C cleared
    use crate::GB::registers::Flags;
    cpu.registers.set_flags(Flags::Z);
    4
}

// DEC A 指令執行範例
fn dec_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let a = cpu.registers.get_a();
    let res = a.wrapping_sub(1);
    use crate::GB::registers::Flags;
    let mut flags = cpu.registers.flags();
    flags.set(Flags::Z, res == 0);
    flags.insert(Flags::N);
    // H set if borrow from bit 4 (i.e., low nibble underflow)
    flags.set(Flags::H, (a & 0x0F) == 0);
    cpu.registers.set_a(res);
    cpu.registers.set_flags(flags);
    4
}

// HALT 指令執行範例
fn halt_exec(_instr: &Instruction, _cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // HALT behavior:
    // - If IME=1: CPU halts until an interrupt occurs (then wakes and services it).
    // - If IME=0 and (IE & IF) == 0: CPU halts until an interrupt becomes pending.
    // - If IME=0 and (IE & IF) != 0: HALT bug — do not halt, and next opcode fetch uses same PC twice.
    let ime = _cpu.ime;
    // Only low 5 bits correspond to real interrupt sources
    let pending = (_cpu.read8(0xFFFF) & _cpu.read8(0xFF0F)) & 0x1F;
    if !ime && pending != 0 {
        // HALT bug: set the flag so fetch_next doesn't advance PC once
        _cpu.halt_bug = true;
    } else {
        _cpu.halted = true;
    }
    4
}

fn or_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let val = cpu.read_u8_imm();
    let res = cpu.registers.get_a() | val;
    cpu.registers.set_a(res);
    // Z set if zero, N H C cleared
    use crate::GB::registers::Flags;
    cpu.registers.set_flags(if res == 0 { Flags::Z } else { Flags::empty() });
    8
}

// RET 指令執行範例
fn ret_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // Without a real stack, emulate pop PC from memory at SP
    let sp = cpu.registers.get_sp();
    cpu.step_mem(4);
    let lo = cpu.read8(sp) as u16;
    cpu.step_mem(4);
    let hi = cpu.read8(sp.wrapping_add(1)) as u16;
    cpu.registers.set_sp(sp.wrapping_add(2));
    let addr = (hi << 8) | lo;
    cpu.registers.set_pc(addr);
    16
}

// ------- Rotates on A (non-CB): RLCA(0x07), RRCA(0x0F), RLA(0x17), RRA(0x1F)
fn rlca_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let a = cpu.registers.get_a();
    let c = (a >> 7) & 1; // old bit7
    let res = (a << 1) | c;
    cpu.registers.set_a(res);
    let mut f = Flags::empty();
    // On GB (not Z80), RLCA clears Z, N, H, sets C to old bit7
    if c != 0 {
        f.insert(Flags::C);
    }
    cpu.registers.set_flags(f);
    4
}

fn rrca_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let a = cpu.registers.get_a();
    let c = a & 1; // old bit0
    let res = (a >> 1) | (c << 7);
    cpu.registers.set_a(res);
    let mut f = Flags::empty();
    if c != 0 {
        f.insert(Flags::C);
    }
    cpu.registers.set_flags(f);
    4
}

fn rla_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let a = cpu.registers.get_a();
    let old_c = if cpu.registers.flags().contains(Flags::C) { 1 } else { 0 };
    let new_c = (a >> 7) & 1;
    let res = (a << 1) | old_c;
    cpu.registers.set_a(res);
    let mut f = Flags::empty();
    if new_c != 0 {
        f.insert(Flags::C);
    }
    cpu.registers.set_flags(f);
    4
}

fn rra_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let a = cpu.registers.get_a();
    let old_c = if cpu.registers.flags().contains(Flags::C) { 1 } else { 0 };
    let new_c = a & 1;
    let res = (a >> 1) | (old_c << 7);
    cpu.registers.set_a(res);
    let mut f = Flags::empty();
    if new_c != 0 {
        f.insert(Flags::C);
    }
    cpu.registers.set_flags(f);
    4
}

// JP nn 指令執行範例
fn jp_nn_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.read_u16_imm();
    cpu.registers.set_pc(addr);
    16
}

// 靜態指令定義
static NOP: Instruction =
    Instruction { opcode: 0x00, name: "NOP", cycles: 4, size: 1, flags: &[], execute: nop_exec };

static LD_A_N: Instruction = Instruction {
    opcode: 0x3E,
    name: "LD A, n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: ld_a_n_exec,
};

static INC_A: Instruction = Instruction {
    opcode: 0x3C,
    name: "INC A",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: inc_a_exec,
};

static XOR_A: Instruction = Instruction {
    opcode: 0xAF,
    name: "XOR A",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: xor_a_exec,
};

static DEC_A: Instruction = Instruction {
    opcode: 0x3D,
    name: "DEC A",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: dec_a_exec,
};

static HALT: Instruction =
    Instruction { opcode: 0x76, name: "HALT", cycles: 4, size: 1, flags: &[], execute: halt_exec };

static JP_NN: Instruction = Instruction {
    opcode: 0xC3,
    name: "JP nn",
    cycles: 16,
    size: 3,
    flags: &[],
    execute: jp_nn_exec,
};

static OR_A_N: Instruction = Instruction {
    opcode: 0xF6,
    name: "OR A, n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: or_a_n_exec,
};

static RET: Instruction =
    Instruction { opcode: 0xC9, name: "RET", cycles: 16, size: 1, flags: &[], execute: ret_exec };

static RLCA: Instruction =
    Instruction { opcode: 0x07, name: "RLCA", cycles: 4, size: 1, flags: &[], execute: rlca_exec };

static RRCA: Instruction =
    Instruction { opcode: 0x0F, name: "RRCA", cycles: 4, size: 1, flags: &[], execute: rrca_exec };

static RLA: Instruction =
    Instruction { opcode: 0x17, name: "RLA", cycles: 4, size: 1, flags: &[], execute: rla_exec };

static RRA: Instruction =
    Instruction { opcode: 0x1F, name: "RRA", cycles: 4, size: 1, flags: &[], execute: rra_exec };

// Generic definitions
pub static GENERIC_LD_R_R: Instruction = Instruction {
    opcode: 0x40, // placeholder; executor uses cpu.opcode
    name: "LD r,r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: ld_r_r_exec,
};

// Generic LD r,n (00 rrr 110)
fn ld_r_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let op = cpu.opcode;
    let dst = (op >> 3) & 0x07;
    let n = cpu.read_u8_imm();
    match dst {
        0 => cpu.registers.set_b(n),
        1 => cpu.registers.set_c(n),
        2 => cpu.registers.set_d(n),
        3 => cpu.registers.set_e(n),
        4 => cpu.registers.set_h(n),
        5 => cpu.registers.set_l(n),
        6 => {
            let hl = cpu.registers.get_hl();
            cpu.write8(hl, n);
            cpu.step_mem(4);
        }
        7 => cpu.registers.set_a(n),
        _ => unreachable!(),
    }
    if dst == 6 { 12 } else { 8 }
}

pub static GENERIC_LD_R_N: Instruction = Instruction {
    opcode: 0x06,
    name: "LD r,n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: ld_r_n_exec,
};

// JR r8 (relative jump)
fn jr_r8_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let off = cpu.read_u8_imm() as i8 as i16 as u16; // sign-extend (fetch already stepped)
    let pc = cpu.registers.get_pc();
    cpu.registers.set_pc(pc.wrapping_add(off));
    12
}

static JR_R8: Instruction = Instruction {
    opcode: 0x18,
    name: "JR r8",
    cycles: 12,
    size: 2,
    flags: &[],
    execute: jr_r8_exec,
};

// -------- Generic INC r / DEC r (00 rrr 100 / 00 rrr 101) --------
fn inc_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let r = (cpu.opcode >> 3) & 0x07;
    let val = read_r_timed(cpu, r);
    let res = val.wrapping_add(1);
    let mut flags = cpu.registers.flags();
    flags.set(Flags::Z, res == 0);
    flags.remove(Flags::N);
    flags.set(Flags::H, (val & 0x0F) + 1 > 0x0F);
    if r == 6 {
        // For (HL), the read occurs on cycle 2 and write on cycle 3 of the instruction.
        // Step before the write-back so the bus write lands on the last cycle.
        cpu.step_mem(4);
    }
    write_r(cpu, r, res);
    cpu.registers.set_flags(flags);
    if r == 6 { 12 } else { 4 }
}

fn dec_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let r = (cpu.opcode >> 3) & 0x07;
    let val = read_r_timed(cpu, r);
    let res = val.wrapping_sub(1);
    let mut flags = cpu.registers.flags();
    flags.set(Flags::Z, res == 0);
    flags.insert(Flags::N);
    flags.set(Flags::H, (val & 0x0F) == 0);
    if r == 6 {
        // For (HL), step before the write-back so the bus write lands on the last cycle.
        cpu.step_mem(4);
    }
    write_r(cpu, r, res);
    cpu.registers.set_flags(flags);
    if r == 6 { 12 } else { 4 }
}

pub static GENERIC_INC_R: Instruction = Instruction {
    opcode: 0x04,
    name: "INC r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: inc_r_exec,
};

pub static GENERIC_DEC_R: Instruction = Instruction {
    opcode: 0x05,
    name: "DEC r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: dec_r_exec,
};

// -------- Remaining flow: JP (HL), RST t, RETI --------
fn jp_hl_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.registers.get_hl();
    cpu.registers.set_pc(addr);
    4
}

fn rst_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // Vector encoded by opcode bucket from 0xC7..0xFF step 8
    let vec = (cpu.opcode.wrapping_sub(0xC7) & 0x38) as u16; // 0x00,0x08,...,0x38
    let ret = cpu.registers.get_pc();
    // push high then low, SP decreases
    let sp1 = cpu.registers.get_sp().wrapping_sub(1);
    cpu.write8(sp1, (ret >> 8) as u8);
    cpu.step_mem(4);
    let sp2 = sp1.wrapping_sub(1);
    cpu.write8(sp2, (ret & 0x00FF) as u8);
    cpu.step_mem(4);
    cpu.registers.set_sp(sp2);
    cpu.registers.set_pc(vec);
    16
}

fn reti_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // same as RET, then set IME
    let sp = cpu.registers.get_sp();
    let lo = cpu.read8(sp) as u16;
    cpu.step_mem(4);
    let hi = cpu.read8(sp.wrapping_add(1)) as u16;
    cpu.step_mem(4);
    cpu.registers.set_sp(sp.wrapping_add(2));
    cpu.registers.set_pc((hi << 8) | lo);
    cpu.ime = true;
    16
}

static JP_HL: Instruction = Instruction {
    opcode: 0xE9,
    name: "JP (HL)",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: jp_hl_exec,
};
static RST_T: Instruction =
    Instruction { opcode: 0xC7, name: "RST t", cycles: 16, size: 1, flags: &[], execute: rst_exec };
static RETI: Instruction =
    Instruction { opcode: 0xD9, name: "RETI", cycles: 16, size: 1, flags: &[], execute: reti_exec };

// Unconditional CALL nn
fn call_nn_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let lo = cpu.fetch_next() as u16;
    let hi = cpu.fetch_next() as u16;
    let addr = (hi << 8) | lo;
    let ret = cpu.registers.get_pc();
    let sp1 = cpu.registers.get_sp().wrapping_sub(1);
    cpu.write8(sp1, (ret >> 8) as u8);
    cpu.step_mem(4);
    let sp2 = sp1.wrapping_sub(1);
    cpu.write8(sp2, (ret & 0x00FF) as u8);
    cpu.step_mem(4);
    cpu.registers.set_sp(sp2);
    cpu.registers.set_pc(addr);
    24
}

static CALL_NN: Instruction = Instruction {
    opcode: 0xCD,
    name: "CALL nn",
    cycles: 24,
    size: 3,
    flags: &[],
    execute: call_nn_exec,
};

// -------- Stack operations: PUSH/POP rr/AF --------
#[inline]
fn get_rr_af(cpu: &crate::GB::CPU::CPU, idx: u8) -> u16 {
    match idx & 0x03 {
        0 => cpu.registers.get_bc(),
        1 => cpu.registers.get_de(),
        2 => cpu.registers.get_hl(),
        3 => cpu.registers.get_af(),
        _ => unreachable!(),
    }
}

#[inline]
fn set_rr_af(cpu: &mut crate::GB::CPU::CPU, idx: u8, val: u16) {
    match idx & 0x03 {
        0 => cpu.registers.set_bc(val),
        1 => cpu.registers.set_de(val),
        2 => cpu.registers.set_hl(val),
        3 => cpu.registers.set_af(val), // set_af 會自動遮掉 F 的低 4 bits
        _ => unreachable!(),
    }
}

fn push_rr_af_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // 11 rr 0101 => rr at bits 5:4; order high then low; SP decrements
    let rr = (cpu.opcode >> 4) & 0x03;
    let val = get_rr_af(cpu, rr);
    let sp1 = cpu.registers.get_sp().wrapping_sub(1);
    cpu.write8(sp1, (val >> 8) as u8);
    cpu.step_mem(4);
    let sp2 = sp1.wrapping_sub(1);
    cpu.write8(sp2, (val & 0x00FF) as u8);
    cpu.step_mem(4);
    cpu.registers.set_sp(sp2);
    16
}

fn pop_rr_af_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // 11 rr 0001 => rr at bits 5:4; pop low then high
    let rr = (cpu.opcode >> 4) & 0x03;
    let sp = cpu.registers.get_sp();
    let lo = cpu.read8(sp) as u16;
    cpu.step_mem(4);
    let hi = cpu.read8(sp.wrapping_add(1)) as u16;
    cpu.step_mem(4);
    cpu.registers.set_sp(sp.wrapping_add(2));
    let val = (hi << 8) | lo;
    set_rr_af(cpu, rr, val);
    12
}

static GENERIC_PUSH_RR_AF: Instruction = Instruction {
    opcode: 0xC5,
    name: "PUSH rr/AF",
    cycles: 16,
    size: 1,
    flags: &[],
    execute: push_rr_af_exec,
};

static GENERIC_POP_RR_AF: Instruction = Instruction {
    opcode: 0xC1,
    name: "POP rr/AF",
    cycles: 12,
    size: 1,
    flags: &[],
    execute: pop_rr_af_exec,
};

// -------- ALU r family (0x80..=0xBF) --------
#[inline]
fn set_flags_zhnc(cpu: &mut crate::GB::CPU::CPU, z: bool, n: bool, h: bool, c: bool) {
    use crate::GB::registers::Flags;
    let mut flags = Flags::empty();
    flags.set(Flags::Z, z);
    flags.set(Flags::N, n);
    flags.set(Flags::H, h);
    flags.set(Flags::C, c);
    cpu.registers.set_flags(flags);
}

fn alu_add_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let a = cpu.registers.get_a();
    let b = read_r_timed(cpu, src);
    let res = a.wrapping_add(b);
    let z = res == 0;
    let n = false;
    let h = (a & 0x0F) + (b & 0x0F) > 0x0F;
    let c = (a as u16 + b as u16) > 0xFF;
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, n, h, c);
    if src == 6 { 8 } else { 4 }
}

fn alu_adc_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let a = cpu.registers.get_a();
    let b = read_r_timed(cpu, src);
    let carry = cpu.registers.flags().contains(crate::GB::registers::Flags::C);
    let cval = if carry { 1 } else { 0 };
    let res = a.wrapping_add(b).wrapping_add(cval);
    let z = res == 0;
    let n = false;
    let h = ((a & 0x0F) + (b & 0x0F) + cval) > 0x0F;
    let c = (a as u16 + b as u16 + cval as u16) > 0xFF;
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, n, h, c);
    if src == 6 { 8 } else { 4 }
}

fn alu_sub_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let a = cpu.registers.get_a();
    let b = read_r_timed(cpu, src);
    let res = a.wrapping_sub(b);
    let z = res == 0;
    let n = true;
    let h = (a & 0x0F) < (b & 0x0F);
    let c = (a as u16) < (b as u16);
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, n, h, c);
    if src == 6 { 8 } else { 4 }
}

fn alu_sbc_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let a = cpu.registers.get_a();
    let b = read_r_timed(cpu, src);
    let carry = cpu.registers.flags().contains(crate::GB::registers::Flags::C);
    let cval = if carry { 1 } else { 0 };
    let res = a.wrapping_sub(b).wrapping_sub(cval);
    let z = res == 0;
    let n = true;
    let h = (a & 0x0F) < ((b & 0x0F) + cval);
    let c = (a as i16) < (b as i16 + cval as i16);
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, n, h, c);
    if src == 6 { 8 } else { 4 }
}

fn alu_and_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let res = cpu.registers.get_a() & read_r_timed(cpu, src);
    cpu.registers.set_a(res);
    // Z set, N=0, H=1, C=0
    set_flags_zhnc(cpu, res == 0, false, true, false);
    if src == 6 { 8 } else { 4 }
}

fn alu_xor_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let res = cpu.registers.get_a() ^ read_r_timed(cpu, src);
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, res == 0, false, false, false);
    if src == 6 { 8 } else { 4 }
}

fn alu_or_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let res = cpu.registers.get_a() | read_r_timed(cpu, src);
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, res == 0, false, false, false);
    if src == 6 { 8 } else { 4 }
}

fn alu_cp_a_r_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let src = cpu.opcode & 0x07;
    let a = cpu.registers.get_a();
    let b = read_r_timed(cpu, src);
    let res = a.wrapping_sub(b);
    let z = res == 0;
    let n = true;
    let h = (a & 0x0F) < (b & 0x0F);
    let c = (a as u16) < (b as u16);
    set_flags_zhnc(cpu, z, n, h, c);
    if src == 6 { 8 } else { 4 }
}

pub static GENERIC_ALU_ADD: Instruction = Instruction {
    opcode: 0x80,
    name: "ADD A,r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_add_a_r_exec,
};
pub static GENERIC_ALU_ADC: Instruction = Instruction {
    opcode: 0x88,
    name: "ADC A,r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_adc_a_r_exec,
};
pub static GENERIC_ALU_SUB: Instruction = Instruction {
    opcode: 0x90,
    name: "SUB r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_sub_a_r_exec,
};
pub static GENERIC_ALU_SBC: Instruction = Instruction {
    opcode: 0x98,
    name: "SBC A,r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_sbc_a_r_exec,
};
pub static GENERIC_ALU_AND: Instruction = Instruction {
    opcode: 0xA0,
    name: "AND r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_and_a_r_exec,
};
pub static GENERIC_ALU_XOR: Instruction = Instruction {
    opcode: 0xA8,
    name: "XOR r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_xor_a_r_exec,
};
pub static GENERIC_ALU_OR: Instruction = Instruction {
    opcode: 0xB0,
    name: "OR r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_or_a_r_exec,
};
pub static GENERIC_ALU_CP: Instruction = Instruction {
    opcode: 0xB8,
    name: "CP r",
    cycles: 4,
    size: 1,
    flags: &[],
    execute: alu_cp_a_r_exec,
};

// -------- ALU n (immediate) family --------
fn alu_add_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let a = cpu.registers.get_a();
    let n = cpu.read_u8_imm();
    let res = a.wrapping_add(n);
    let z = res == 0;
    let nflag = false;
    let h = (a & 0x0F) + (n & 0x0F) > 0x0F;
    let c = (a as u16 + n as u16) > 0xFF;
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, nflag, h, c);
    8
}

fn alu_adc_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let a = cpu.registers.get_a();
    let n = cpu.read_u8_imm();
    let carry = cpu.registers.flags().contains(crate::GB::registers::Flags::C);
    let cval = if carry { 1 } else { 0 };
    let res = a.wrapping_add(n).wrapping_add(cval);
    let z = res == 0;
    let nflag = false;
    let h = ((a & 0x0F) + (n & 0x0F) + cval) > 0x0F;
    let c = (a as u16 + n as u16 + cval as u16) > 0xFF;
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, nflag, h, c);
    8
}

fn alu_sub_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let a = cpu.registers.get_a();
    let n = cpu.read_u8_imm();
    let res = a.wrapping_sub(n);
    let z = res == 0;
    let nflag = true;
    let h = (a & 0x0F) < (n & 0x0F);
    let c = (a as u16) < (n as u16);
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, nflag, h, c);
    8
}

fn alu_sbc_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let a = cpu.registers.get_a();
    let n = cpu.read_u8_imm();
    let carry = cpu.registers.flags().contains(crate::GB::registers::Flags::C);
    let cval = if carry { 1 } else { 0 };
    let res = a.wrapping_sub(n).wrapping_sub(cval);
    let z = res == 0;
    let nflag = true;
    let h = (a & 0x0F) < ((n & 0x0F) + cval);
    let c = (a as i16) < (n as i16 + cval as i16);
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, z, nflag, h, c);
    8
}

fn alu_and_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let n = cpu.read_u8_imm();
    let res = cpu.registers.get_a() & n;
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, res == 0, false, true, false);
    8
}

fn alu_xor_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let n = cpu.read_u8_imm();
    let res = cpu.registers.get_a() ^ n;
    cpu.registers.set_a(res);
    set_flags_zhnc(cpu, res == 0, false, false, false);
    8
}

fn alu_cp_a_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let a = cpu.registers.get_a();
    let n = cpu.read_u8_imm();
    let res = a.wrapping_sub(n);
    let z = res == 0;
    let nflag = true;
    let h = (a & 0x0F) < (n & 0x0F);
    let c = (a as u16) < (n as u16);
    set_flags_zhnc(cpu, z, nflag, h, c);
    8
}

static ADD_A_N: Instruction = Instruction {
    opcode: 0xC6,
    name: "ADD A,n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: alu_add_a_n_exec,
};
static ADC_A_N: Instruction = Instruction {
    opcode: 0xCE,
    name: "ADC A,n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: alu_adc_a_n_exec,
};
static SUB_N: Instruction = Instruction {
    opcode: 0xD6,
    name: "SUB n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: alu_sub_a_n_exec,
};
static SBC_A_N: Instruction = Instruction {
    opcode: 0xDE,
    name: "SBC A,n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: alu_sbc_a_n_exec,
};
static AND_N: Instruction = Instruction {
    opcode: 0xE6,
    name: "AND n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: alu_and_a_n_exec,
};
static XOR_N: Instruction = Instruction {
    opcode: 0xEE,
    name: "XOR n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: alu_xor_a_n_exec,
};
static CP_N: Instruction = Instruction {
    opcode: 0xFE,
    name: "CP n",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: alu_cp_a_n_exec,
};

// -------- HL auto inc/dec loads --------
fn ld_hl_inc_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let hl = cpu.registers.get_hl();
    let a = cpu.registers.get_a();
    cpu.step_mem(4);
    cpu.write8(hl, a);
    cpu.registers.set_hl(hl.wrapping_add(1));
    8
}

fn ld_a_hl_inc_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let hl = cpu.registers.get_hl();
    cpu.step_mem(4);
    let val = cpu.read8(hl);
    cpu.registers.set_a(val);
    cpu.registers.set_hl(hl.wrapping_add(1));
    8
}

fn ld_hl_dec_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let hl = cpu.registers.get_hl();
    let a = cpu.registers.get_a();
    cpu.step_mem(4);
    cpu.write8(hl, a);
    cpu.registers.set_hl(hl.wrapping_sub(1));
    8
}

fn ld_a_hl_dec_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let hl = cpu.registers.get_hl();
    cpu.step_mem(4);
    let val = cpu.read8(hl);
    cpu.registers.set_a(val);
    cpu.registers.set_hl(hl.wrapping_sub(1));
    8
}

static LD_HL_INC_A: Instruction = Instruction {
    opcode: 0x22,
    name: "LD (HL+),A",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_hl_inc_a_exec,
};
static LD_A_HL_INC: Instruction = Instruction {
    opcode: 0x2A,
    name: "LD A,(HL+)",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_a_hl_inc_exec,
};
static LD_HL_DEC_A: Instruction = Instruction {
    opcode: 0x32,
    name: "LD (HL-),A",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_hl_dec_a_exec,
};
static LD_A_HL_DEC: Instruction = Instruction {
    opcode: 0x3A,
    name: "LD A,(HL-)",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_a_hl_dec_exec,
};

// -------- Simple indirect loads via BC/DE and (HL),n --------
fn ld_a_bc_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.registers.get_bc();
    cpu.step_mem(4);
    let val = cpu.read8(addr);
    cpu.registers.set_a(val);
    8
}

fn ld_a_de_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.registers.get_de();
    cpu.step_mem(4);
    let val = cpu.read8(addr);
    cpu.registers.set_a(val);
    8
}

fn ld_bc_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.registers.get_bc();
    let a = cpu.registers.get_a();
    cpu.step_mem(4);
    cpu.write8(addr, a);
    8
}

fn ld_de_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.registers.get_de();
    let a = cpu.registers.get_a();
    cpu.step_mem(4);
    cpu.write8(addr, a);
    8
}

fn ld_hl_n_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let n = cpu.read_u8_imm();
    let hl = cpu.registers.get_hl();
    cpu.step_mem(4);
    cpu.write8(hl, n);
    12
}

static LD_A_BC: Instruction = Instruction {
    opcode: 0x0A,
    name: "LD A,(BC)",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_a_bc_exec,
};
static LD_A_DE: Instruction = Instruction {
    opcode: 0x1A,
    name: "LD A,(DE)",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_a_de_exec,
};
static LD_BC_A: Instruction = Instruction {
    opcode: 0x02,
    name: "LD (BC),A",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_bc_a_exec,
};
static LD_DE_A: Instruction = Instruction {
    opcode: 0x12,
    name: "LD (DE),A",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_de_a_exec,
};
static LD_HL_N: Instruction = Instruction {
    opcode: 0x36,
    name: "LD (HL),n",
    cycles: 12,
    size: 2,
    flags: &[],
    execute: ld_hl_n_exec,
};

// -------- SP/offset and absolute address moves --------
fn add_sp_e8_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let e = cpu.read_u8_imm();
    let e_signed = e as i8 as i16 as u16;
    let sp = cpu.registers.get_sp();
    let res = sp.wrapping_add(e_signed);
    // Flags: Z=0, N=0, H and C from low-byte addition
    let low_sp = (sp & 0x00FF) as u16;
    let low_e = (e as u16) & 0x00FF;
    let h = ((sp & 0x000F) + (low_e & 0x000F)) > 0x000F;
    let c = (low_sp + low_e) > 0x00FF;
    let mut f = Flags::empty();
    f.set(Flags::H, h);
    f.set(Flags::C, c);
    // Z and N cleared
    cpu.registers.set_flags(f);
    cpu.registers.set_sp(res);
    16
}

fn ld_hl_sp_e8_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    use crate::GB::registers::Flags;
    let e = cpu.read_u8_imm();
    let e_signed = e as i8 as i16 as u16;
    let sp = cpu.registers.get_sp();
    let res = sp.wrapping_add(e_signed);
    let low_sp = (sp & 0x00FF) as u16;
    let low_e = (e as u16) & 0x00FF;
    let h = ((sp & 0x000F) + (low_e & 0x000F)) > 0x000F;
    let c = (low_sp + low_e) > 0x00FF;
    let mut f = Flags::empty();
    f.set(Flags::H, h);
    f.set(Flags::C, c);
    cpu.registers.set_flags(f); // Z=0, N=0
    cpu.registers.set_hl(res);
    12
}

fn ld_sp_hl_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // Flags unaffected
    let hl = cpu.registers.get_hl();
    cpu.registers.set_sp(hl);
    8
}

fn ld_a16_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.read_u16_imm();
    let a = cpu.registers.get_a();
    cpu.step_mem(4);
    cpu.write8(addr, a);
    16
}

fn ld_a_a16_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.read_u16_imm();
    cpu.step_mem(4);
    let val = cpu.read8(addr);
    cpu.registers.set_a(val);
    16
}

fn ld_a16_sp_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let addr = cpu.read_u16_imm();
    let sp = cpu.registers.get_sp();
    // write low then high
    cpu.step_mem(4);
    cpu.write8(addr, (sp & 0x00FF) as u8);
    cpu.step_mem(4);
    cpu.write8(addr.wrapping_add(1), (sp >> 8) as u8);
    20
}

static ADD_SP_E8: Instruction = Instruction {
    opcode: 0xE8,
    name: "ADD SP,e8",
    cycles: 16,
    size: 2,
    flags: &[],
    execute: add_sp_e8_exec,
};
static LD_HL_SP_E8: Instruction = Instruction {
    opcode: 0xF8,
    name: "LD HL,SP+e8",
    cycles: 12,
    size: 2,
    flags: &[],
    execute: ld_hl_sp_e8_exec,
};
static LD_SP_HL: Instruction = Instruction {
    opcode: 0xF9,
    name: "LD SP,HL",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_sp_hl_exec,
};
static LD_A16_A: Instruction = Instruction {
    opcode: 0xEA,
    name: "LD (a16),A",
    cycles: 16,
    size: 3,
    flags: &[],
    execute: ld_a16_a_exec,
};
static LD_A_A16: Instruction = Instruction {
    opcode: 0xFA,
    name: "LD A,(a16)",
    cycles: 16,
    size: 3,
    flags: &[],
    execute: ld_a_a16_exec,
};
static LD_A16_SP: Instruction = Instruction {
    opcode: 0x08,
    name: "LD (a16),SP",
    cycles: 20,
    size: 3,
    flags: &[],
    execute: ld_a16_sp_exec,
};

// -------- LDH high-page I/O moves --------
fn ldh_a8_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // LD (0xFF00 + a8),A
    let off = cpu.read_u8_imm() as u16;
    let addr = 0xFF00u16.wrapping_add(off);
    let a = cpu.registers.get_a();
    cpu.step_mem(4);
    cpu.write8(addr, a);
    12
}

fn ldh_a_a8_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // LD A,(0xFF00 + a8)
    let off = cpu.read_u8_imm() as u16;
    let addr = 0xFF00u16.wrapping_add(off);
    cpu.step_mem(4);
    let v = cpu.read8(addr);
    cpu.registers.set_a(v);
    12
}

fn ld_c_a_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // LD (0xFF00 + C),A
    let addr = 0xFF00u16.wrapping_add(cpu.registers.get_c() as u16);
    let a = cpu.registers.get_a();
    cpu.step_mem(4);
    cpu.write8(addr, a);
    8
}

fn ld_a_c_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // LD A,(0xFF00 + C)
    let addr = 0xFF00u16.wrapping_add(cpu.registers.get_c() as u16);
    cpu.step_mem(4);
    let v = cpu.read8(addr);
    cpu.registers.set_a(v);
    8
}

static LDH_A8_A: Instruction = Instruction {
    opcode: 0xE0,
    name: "LDH (a8),A",
    cycles: 12,
    size: 2,
    flags: &[],
    execute: ldh_a8_a_exec,
};
static LDH_A_A8: Instruction = Instruction {
    opcode: 0xF0,
    name: "LDH A,(a8)",
    cycles: 12,
    size: 2,
    flags: &[],
    execute: ldh_a_a8_exec,
};
static LD_C_A: Instruction = Instruction {
    opcode: 0xE2,
    name: "LD (C),A",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_c_a_exec,
};
static LD_A_C: Instruction = Instruction {
    opcode: 0xF2,
    name: "LD A,(C)",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: ld_a_c_exec,
};
// -------- CB prefix generic executor --------
fn cb_generic_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let op = cpu.opcode; // second byte after 0xCB
    let x = op >> 6; // 0..3
    let y = (op >> 3) & 0x07; // bit/opcode within group
    let z = op & 0x07; // register index

    // Helpers
    // For (HL) (z==6), memory is accessed; step timing accordingly before the read.
    if z == 6 {
        cpu.step_mem(4);
    }
    let mut val = read_r(cpu, z);
    let cycles = match (x, z == 6) {
        (0, true) => 16, // rotates/shifts on (HL)
        (0, false) => 8,
        (1, true) => 12, // BIT on (HL)
        (1, false) => 8,
        (2, true) | (3, true) => 16, // RES/SET on (HL)
        (2, false) | (3, false) => 8,
        _ => 8,
    };

    match x {
        0 => {
            // Rot/Shift group by y
            use crate::GB::registers::Flags;
            let mut f = Flags::empty(); // we'll set Z and C as needed; N=H=0
            match y {
                0 => {
                    // RLC
                    let carry = (val & 0x80) != 0;
                    val = (val << 1) | (if carry { 1 } else { 0 });
                    f.set(Flags::Z, val == 0);
                    f.set(Flags::C, carry);
                }
                1 => {
                    // RRC
                    let carry = (val & 0x01) != 0;
                    val = (val >> 1) | (if carry { 0x80 } else { 0 });
                    f.set(Flags::Z, val == 0);
                    f.set(Flags::C, carry);
                }
                2 => {
                    // RL
                    let old_c = cpu.registers.flags().contains(Flags::C);
                    let carry = (val & 0x80) != 0;
                    val = (val << 1) | (if old_c { 1 } else { 0 });
                    f.set(Flags::Z, val == 0);
                    f.set(Flags::C, carry);
                }
                3 => {
                    // RR
                    let old_c = cpu.registers.flags().contains(Flags::C);
                    let carry = (val & 0x01) != 0;
                    val = (if old_c { 0x80 } else { 0 }) | (val >> 1);
                    f.set(Flags::Z, val == 0);
                    f.set(Flags::C, carry);
                }
                4 => {
                    // SLA
                    let carry = (val & 0x80) != 0;
                    val = val << 1;
                    f.set(Flags::Z, val == 0);
                    f.set(Flags::C, carry);
                }
                5 => {
                    // SRA
                    let carry = (val & 0x01) != 0;
                    let msb = val & 0x80;
                    val = (val >> 1) | msb;
                    f.set(Flags::Z, val == 0);
                    f.set(Flags::C, carry);
                }
                6 => {
                    // SWAP
                    val = (val << 4) | (val >> 4);
                    f.set(Flags::Z, val == 0);
                    // C=0
                }
                7 => {
                    // SRL
                    let carry = (val & 0x01) != 0;
                    val = val >> 1;
                    f.set(Flags::Z, val == 0);
                    f.set(Flags::C, carry);
                }
                _ => {}
            }
            // N=H=0 are already zeroed
            cpu.registers.set_flags(f);
            if z == 6 {
                cpu.step_mem(4);
            } // write-back to (HL)
            write_r(cpu, z, val);
        }
        1 => {
            // BIT y, r[z]
            let bit = (val >> y) & 1;
            use crate::GB::registers::Flags;
            let mut f = cpu.registers.flags();
            f.set(Flags::Z, bit == 0);
            f.remove(Flags::N);
            f.insert(Flags::H);
            // C unchanged
            cpu.registers.set_flags(f);
        }
        2 => {
            // RES y, r[z]
            val &= !(1u8 << y);
            if z == 6 {
                cpu.step_mem(4);
            }
            write_r(cpu, z, val);
        }
        3 => {
            // SET y, r[z]
            val |= 1u8 << y;
            if z == 6 {
                cpu.step_mem(4);
            }
            write_r(cpu, z, val);
        }
        _ => {}
    }

    cycles
}

static CB_GENERIC: Instruction = Instruction {
    opcode: 0xCB,
    name: "CB *",
    cycles: 8,
    size: 2,
    flags: &[],
    execute: cb_generic_exec,
};

// -------- 16-bit instruction families --------
#[inline]
fn get_rr(cpu: &crate::GB::CPU::CPU, idx: u8) -> u16 {
    match idx & 0x03 {
        0 => cpu.registers.get_bc(),
        1 => cpu.registers.get_de(),
        2 => cpu.registers.get_hl(),
        3 => cpu.registers.get_sp(),
        _ => unreachable!(),
    }
}

#[inline]
fn set_rr(cpu: &mut crate::GB::CPU::CPU, idx: u8, val: u16) {
    match idx & 0x03 {
        0 => cpu.registers.set_bc(val),
        1 => cpu.registers.set_de(val),
        2 => cpu.registers.set_hl(val),
        3 => cpu.registers.set_sp(val),
        _ => unreachable!(),
    }
}

fn ld_rr_nn_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // 00 rr 0001 => rr at bits 5:4
    let rr = (cpu.opcode >> 4) & 0x03;
    let imm = cpu.read_u16_imm();
    set_rr(cpu, rr, imm);
    12
}

fn inc_rr_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // 00 rr 0011
    let rr = (cpu.opcode >> 4) & 0x03;
    let val = get_rr(cpu, rr).wrapping_add(1);
    set_rr(cpu, rr, val);
    8
}

fn dec_rr_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // 00 rr 1011
    let rr = (cpu.opcode >> 4) & 0x03;
    let val = get_rr(cpu, rr).wrapping_sub(1);
    set_rr(cpu, rr, val);
    8
}

fn add_hl_rr_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    // 00 rr 1001
    let rr = (cpu.opcode >> 4) & 0x03;
    let hl = cpu.registers.get_hl();
    let op = get_rr(cpu, rr);
    let res = hl.wrapping_add(op);
    // Flags: Z unaffected, N=0, H carry from bit 11, C carry
    use crate::GB::registers::Flags;
    let mut flags = cpu.registers.flags();
    // Preserve Z, clear N
    let z = flags.contains(Flags::Z);
    flags = Flags::empty();
    flags.set(Flags::Z, z);
    // Half-carry: carry from bit 11
    let h = ((hl & 0x0FFF) + (op & 0x0FFF)) > 0x0FFF;
    flags.set(Flags::H, h);
    let c = (hl as u32 + op as u32) > 0xFFFF;
    flags.set(Flags::C, c);
    // N stays cleared
    cpu.registers.set_flags(flags);
    cpu.registers.set_hl(res);
    8
}

pub static GENERIC_LD_RR_NN: Instruction = Instruction {
    opcode: 0x01,
    name: "LD rr,nn",
    cycles: 12,
    size: 3,
    flags: &[],
    execute: ld_rr_nn_exec,
};
pub static GENERIC_INC_RR: Instruction = Instruction {
    opcode: 0x03,
    name: "INC rr",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: inc_rr_exec,
};
pub static GENERIC_DEC_RR: Instruction = Instruction {
    opcode: 0x0B,
    name: "DEC rr",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: dec_rr_exec,
};
pub static GENERIC_ADD_HL_RR: Instruction = Instruction {
    opcode: 0x09,
    name: "ADD HL,rr",
    cycles: 8,
    size: 1,
    flags: &[],
    execute: add_hl_rr_exec,
};

// -------- Conditional control flow (JR/JP/CALL/RET with cc) --------
#[inline]
fn cc_is_true(flags_u8: u8, cc: u8) -> bool {
    use crate::GB::registers::Flags;
    let flags = Flags::from_bits_truncate(flags_u8);
    match cc & 0x03 {
        0 => !flags.contains(Flags::Z), // NZ
        1 => flags.contains(Flags::Z),  // Z
        2 => !flags.contains(Flags::C), // NC
        3 => flags.contains(Flags::C),  // C
        _ => false,
    }
}

fn jr_cc_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let cc = (cpu.opcode >> 3) & 0x03;
    let off = cpu.read_u8_imm() as i8 as i16 as u16;
    if cc_is_true(cpu.registers.get_f(), cc) {
        let pc = cpu.registers.get_pc();
        cpu.registers.set_pc(pc.wrapping_add(off));
        12
    } else {
        8
    }
}

fn jp_cc_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let cc = (cpu.opcode >> 3) & 0x03;
    let addr = cpu.read_u16_imm();
    if cc_is_true(cpu.registers.get_f(), cc) {
        cpu.registers.set_pc(addr);
        16
    } else {
        12
    }
}

fn call_cc_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let cc = (cpu.opcode >> 3) & 0x03;
    let addr = cpu.read_u16_imm();
    if cc_is_true(cpu.registers.get_f(), cc) {
        let ret = cpu.registers.get_pc();
        // push return address: write high then low, SP decreases
        let sp1 = cpu.registers.get_sp().wrapping_sub(1);
        cpu.write8(sp1, (ret >> 8) as u8);
        cpu.step_mem(4);
        let sp2 = sp1.wrapping_sub(1);
        cpu.write8(sp2, (ret & 0x00FF) as u8);
        cpu.step_mem(4);
        cpu.registers.set_sp(sp2);
        cpu.registers.set_pc(addr);
        24
    } else {
        12
    }
}

fn ret_cc_exec(_instr: &Instruction, cpu: &mut crate::GB::CPU::CPU) -> u64 {
    let cc = (cpu.opcode >> 3) & 0x03;
    if cc_is_true(cpu.registers.get_f(), cc) {
        let sp = cpu.registers.get_sp();
        cpu.step_mem(4);
        let lo = cpu.read8(sp) as u16;
        cpu.step_mem(4);
        let hi = cpu.read8(sp.wrapping_add(1)) as u16;
        cpu.registers.set_sp(sp.wrapping_add(2));
        cpu.registers.set_pc((hi << 8) | lo);
        20
    } else {
        8
    }
}

static JR_CC: Instruction = Instruction {
    opcode: 0x20,
    name: "JR cc,r8",
    cycles: 12,
    size: 2,
    flags: &[],
    execute: jr_cc_exec,
};
static JP_CC: Instruction = Instruction {
    opcode: 0xC2,
    name: "JP cc,nn",
    cycles: 16,
    size: 3,
    flags: &[],
    execute: jp_cc_exec,
};
static CALL_CC: Instruction = Instruction {
    opcode: 0xC4,
    name: "CALL cc,nn",
    cycles: 24,
    size: 3,
    flags: &[],
    execute: call_cc_exec,
};
static RET_CC: Instruction = Instruction {
    opcode: 0xC0,
    name: "RET cc",
    cycles: 20,
    size: 1,
    flags: &[],
    execute: ret_cc_exec,
};

// macro 產生陣列內容
macro_rules! def_instr_array {
    ( $( $opcode:expr => $instr:expr ),* $(,)? ) => {{
        let mut arr: [Option<&'static Instruction>; 256] = [None; 256];
        $(
            arr[($opcode) as usize] = Some(&$instr);
        )*
        arr
    }};
}

// 指令表常數
pub static OPCODES: [Option<&'static Instruction>; 256] = def_instr_array! {
    // Rotates on A
    0x07 => RLCA,
    0x0F => RRCA,
    0x17 => RLA,
    0x1F => RRA,
    // Simple indirect loads
    0x0A => LD_A_BC,
    0x1A => LD_A_DE,
    0x02 => LD_BC_A,
    0x12 => LD_DE_A,
    0x36 => LD_HL_N,
    0x00 => NOP,
    0x18 => JR_R8,
    // JR cc
    0x20 => JR_CC, 0x28 => JR_CC, 0x30 => JR_CC, 0x38 => JR_CC,
    0x3E => LD_A_N,
    0x3C => INC_A,
    0x3D => DEC_A,
    0x76 => HALT,
    0xAF => XOR_A,
    // HL auto inc/dec
    0x22 => LD_HL_INC_A,
    0x2A => LD_A_HL_INC,
    0x32 => LD_HL_DEC_A,
    0x3A => LD_A_HL_DEC,
    // PUSH/POP rr/AF
    0xC5 => GENERIC_PUSH_RR_AF, 0xD5 => GENERIC_PUSH_RR_AF, 0xE5 => GENERIC_PUSH_RR_AF, 0xF5 => GENERIC_PUSH_RR_AF,
    0xC1 => GENERIC_POP_RR_AF,  0xD1 => GENERIC_POP_RR_AF,  0xE1 => GENERIC_POP_RR_AF,  0xF1 => GENERIC_POP_RR_AF,
    // INC r
    0x04 => GENERIC_INC_R, 0x0C => GENERIC_INC_R, 0x14 => GENERIC_INC_R, 0x1C => GENERIC_INC_R,
    0x24 => GENERIC_INC_R, 0x2C => GENERIC_INC_R, 0x34 => GENERIC_INC_R,
    // DEC r
    0x05 => GENERIC_DEC_R, 0x0D => GENERIC_DEC_R, 0x15 => GENERIC_DEC_R, 0x1D => GENERIC_DEC_R,
    0x25 => GENERIC_DEC_R, 0x2D => GENERIC_DEC_R, 0x35 => GENERIC_DEC_R,
    // ALU immediate family
    0xC6 => ADD_A_N, 0xCE => ADC_A_N, 0xD6 => SUB_N, 0xDE => SBC_A_N,
    0xE6 => AND_N, 0xEE => XOR_N, 0xF6 => OR_A_N, 0xFE => CP_N,
    // JP (HL), RETI
    0xE9 => JP_HL,
    0xD9 => RETI,
    // JP cc
    0xC2 => JP_CC, 0xCA => JP_CC, 0xD2 => JP_CC, 0xDA => JP_CC,
    0xC3 => JP_NN,
    0xCD => CALL_NN,
    // CALL cc
    0xC4 => CALL_CC, 0xCC => CALL_CC, 0xD4 => CALL_CC, 0xDC => CALL_CC,
    0xF6 => OR_A_N,
    // RET cc
    0xC0 => RET_CC, 0xC8 => RET_CC, 0xD0 => RET_CC, 0xD8 => RET_CC,
    0xC9 => RET,
    // RST t
    0xC7 => RST_T, 0xCF => RST_T, 0xD7 => RST_T, 0xDF => RST_T,
    0xE7 => RST_T, 0xEF => RST_T, 0xF7 => RST_T, 0xFF => RST_T,
    // CPU/flags
    0xFB => EI,
    0xF3 => DI,
    0x3F => CCF,
    0x37 => SCF,
    0x2F => CPL,
    0x27 => DAAI,
    0x10 => STOP,
    // SP/offset and absolute address moves
    0xE8 => ADD_SP_E8,
    0xF8 => LD_HL_SP_E8,
    0xF9 => LD_SP_HL,
    0xEA => LD_A16_A,
    0xFA => LD_A_A16,
    0x08 => LD_A16_SP,
    // 16-bit families
    0x01 => GENERIC_LD_RR_NN,
    0x11 => GENERIC_LD_RR_NN,
    0x21 => GENERIC_LD_RR_NN,
    0x31 => GENERIC_LD_RR_NN,
    0x03 => GENERIC_INC_RR,
    0x13 => GENERIC_INC_RR,
    0x23 => GENERIC_INC_RR,
    0x33 => GENERIC_INC_RR,
    0x0B => GENERIC_DEC_RR,
    0x1B => GENERIC_DEC_RR,
    0x2B => GENERIC_DEC_RR,
    0x3B => GENERIC_DEC_RR,
    0x09 => GENERIC_ADD_HL_RR,
    0x19 => GENERIC_ADD_HL_RR,
    0x29 => GENERIC_ADD_HL_RR,
    0x39 => GENERIC_ADD_HL_RR,
    // LDH/IO high-page
    0xE0 => LDH_A8_A,
    0xF0 => LDH_A_A8,
    0xE2 => LD_C_A,
    0xF2 => LD_A_C,
    // ALU r groups (0x80..=0xBF)
    0x80 => GENERIC_ALU_ADD, 0x81 => GENERIC_ALU_ADD, 0x82 => GENERIC_ALU_ADD, 0x83 => GENERIC_ALU_ADD,
    0x84 => GENERIC_ALU_ADD, 0x85 => GENERIC_ALU_ADD, 0x86 => GENERIC_ALU_ADD, 0x87 => GENERIC_ALU_ADD,
    0x88 => GENERIC_ALU_ADC, 0x89 => GENERIC_ALU_ADC, 0x8A => GENERIC_ALU_ADC, 0x8B => GENERIC_ALU_ADC,
    0x8C => GENERIC_ALU_ADC, 0x8D => GENERIC_ALU_ADC, 0x8E => GENERIC_ALU_ADC, 0x8F => GENERIC_ALU_ADC,
    0x90 => GENERIC_ALU_SUB, 0x91 => GENERIC_ALU_SUB, 0x92 => GENERIC_ALU_SUB, 0x93 => GENERIC_ALU_SUB,
    0x94 => GENERIC_ALU_SUB, 0x95 => GENERIC_ALU_SUB, 0x96 => GENERIC_ALU_SUB, 0x97 => GENERIC_ALU_SUB,
    0x98 => GENERIC_ALU_SBC, 0x99 => GENERIC_ALU_SBC, 0x9A => GENERIC_ALU_SBC, 0x9B => GENERIC_ALU_SBC,
    0x9C => GENERIC_ALU_SBC, 0x9D => GENERIC_ALU_SBC, 0x9E => GENERIC_ALU_SBC, 0x9F => GENERIC_ALU_SBC,
    0xA0 => GENERIC_ALU_AND, 0xA1 => GENERIC_ALU_AND, 0xA2 => GENERIC_ALU_AND, 0xA3 => GENERIC_ALU_AND,
    0xA4 => GENERIC_ALU_AND, 0xA5 => GENERIC_ALU_AND, 0xA6 => GENERIC_ALU_AND, 0xA7 => GENERIC_ALU_AND,
    0xA8 => GENERIC_ALU_XOR, 0xA9 => GENERIC_ALU_XOR, 0xAA => GENERIC_ALU_XOR, 0xAB => GENERIC_ALU_XOR,
    0xAC => GENERIC_ALU_XOR, 0xAD => GENERIC_ALU_XOR, 0xAE => GENERIC_ALU_XOR, 0xAF => GENERIC_ALU_XOR,
    0xB0 => GENERIC_ALU_OR,  0xB1 => GENERIC_ALU_OR,  0xB2 => GENERIC_ALU_OR,  0xB3 => GENERIC_ALU_OR,
    0xB4 => GENERIC_ALU_OR,  0xB5 => GENERIC_ALU_OR,  0xB6 => GENERIC_ALU_OR,  0xB7 => GENERIC_ALU_OR,
    0xB8 => GENERIC_ALU_CP,  0xB9 => GENERIC_ALU_CP,  0xBA => GENERIC_ALU_CP,  0xBB => GENERIC_ALU_CP,
    0xBC => GENERIC_ALU_CP,  0xBD => GENERIC_ALU_CP,  0xBE => GENERIC_ALU_CP,  0xBF => GENERIC_ALU_CP,
    // 其他指令...
};

// Fill CB table with single generic executor (exec uses cpu.opcode to disambiguate)
pub static OPCODES_CB: [Option<&'static Instruction>; 256] = [Some(&CB_GENERIC); 256];
