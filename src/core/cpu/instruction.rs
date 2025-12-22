use crate::core::cpu::register_utils::RegTarget;
#[derive(Debug, Clone, Copy)]
pub enum AluRegOp {
    Add,
    Adc,
    Sub,
    Sbc,
    And,
    Or,
    Xor,
    Cp,
    Inc,
    Dec,
}

#[derive(Debug, Clone, Copy)]
pub enum SimpleAluInstruction {
    RegOp(AluRegOp, RegTarget),
    ImmOp(AluRegOp),
}

impl SimpleAluInstruction {
    pub fn decode(opcode: u8) -> Option<Self> {
        match opcode {
            0x80..=0x87 => Some(SimpleAluInstruction::RegOp(
                AluRegOp::Add,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0x88..=0x8F => Some(SimpleAluInstruction::RegOp(
                AluRegOp::Adc,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0x90..=0x97 => Some(SimpleAluInstruction::RegOp(
                AluRegOp::Sub,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0x98..=0x9F => Some(SimpleAluInstruction::RegOp(
                AluRegOp::Sbc,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0xA0..=0xA7 => Some(SimpleAluInstruction::RegOp(
                AluRegOp::And,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0xB0..=0xB7 => Some(SimpleAluInstruction::RegOp(
                AluRegOp::Or,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0xA8..=0xAF => Some(SimpleAluInstruction::RegOp(
                AluRegOp::Xor,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0xB8..=0xBF => Some(SimpleAluInstruction::RegOp(
                AluRegOp::Cp,
                RegTarget::from_bits(opcode & 0x07).ok()?,
            )),
            0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => {
                let reg = match opcode {
                    0x04 => RegTarget::B,
                    0x0C => RegTarget::C,
                    0x14 => RegTarget::D,
                    0x1C => RegTarget::E,
                    0x24 => RegTarget::H,
                    0x2C => RegTarget::L,
                    0x34 => RegTarget::HL,
                    0x3C => RegTarget::A,
                    _ => return None,
                };
                Some(SimpleAluInstruction::RegOp(AluRegOp::Inc, reg))
            }
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                let reg = match opcode {
                    0x05 => RegTarget::B,
                    0x0D => RegTarget::C,
                    0x15 => RegTarget::D,
                    0x1D => RegTarget::E,
                    0x25 => RegTarget::H,
                    0x2D => RegTarget::L,
                    0x35 => RegTarget::HL,
                    0x3D => RegTarget::A,
                    _ => return None,
                };
                Some(SimpleAluInstruction::RegOp(AluRegOp::Dec, reg))
            }
            0xC6 => Some(SimpleAluInstruction::ImmOp(AluRegOp::Add)),
            0xCE => Some(SimpleAluInstruction::ImmOp(AluRegOp::Adc)),
            0xD6 => Some(SimpleAluInstruction::ImmOp(AluRegOp::Sub)),
            0xDE => Some(SimpleAluInstruction::ImmOp(AluRegOp::Sbc)),
            0xE6 => Some(SimpleAluInstruction::ImmOp(AluRegOp::And)),
            0xF6 => Some(SimpleAluInstruction::ImmOp(AluRegOp::Or)),
            0xEE => Some(SimpleAluInstruction::ImmOp(AluRegOp::Xor)),
            0xFE => Some(SimpleAluInstruction::ImmOp(AluRegOp::Cp)),
            _ => None,
        }
    }
}
use crate::core::cpu::register_utils::RegTarget;

/// Game Boy CPU 指令分類
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    // 載入指令
    LD(LDTarget, LDSource),

    // 算術指令
    ADD(ArithTarget, ArithSource),
    ADC(ArithTarget, ArithSource),
    SUB(ArithSource),
    SBC(ArithSource),
    AND(ArithSource),
    OR(ArithSource),
    XOR(ArithSource),
    CP(ArithSource),

    // 增減指令
    INC(IncDecTarget),
    DEC(IncDecTarget),

    // 旋轉/移位指令
    RLCA,
    RRCA,
    RLA,
    RRA,
    RLC(RegTarget),
    RRC(RegTarget),
    RL(RegTarget),
    RR(RegTarget),
    SLA(RegTarget),
    SRA(RegTarget),
    SRL(RegTarget),

    // 位元操作指令
    BIT(u8, RegTarget),
    SET(u8, RegTarget),
    RES(u8, RegTarget),

    // 跳轉指令
    JP(JumpCondition, JumpTarget),
    JR(JumpCondition, i8),

    // 呼叫/返回指令
    CALL(JumpCondition, u16),
    RET(JumpCondition),
    RETI,
    RST(u8),

    // 堆疊指令
    PUSH(StackTarget),
    POP(StackTarget),

    // 其他指令
    CPL,
    CCF,
    SCF,
    DAA,
    HALT,
    STOP,
    DI,
    EI,
    NOP,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LDTarget {
    Reg(RegTarget),
    Addr16(u16),
    AddrReg(RegTarget), // (BC), (DE), (HL), (SP)
    AddrFF00(u8),       // (FF00+n)
    AddrFF00C,          // (FF00+C)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LDSource {
    Reg(RegTarget),
    Imm8(u8),
    Imm16(u16),
    Addr16(u16),
    AddrReg(RegTarget),
    AddrFF00(u8),
    AddrFF00C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithTarget {
    A,  // 只有 A 可以作為目標
    HL, // ADD HL,rr
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithSource {
    Reg(RegTarget),
    Imm8(u8),
    AddrHL, // (HL)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncDecTarget {
    Reg8(RegTarget),
    Reg16(Reg16Target),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reg16Target {
    BC,
    DE,
    HL,
    SP,
    AF,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpCondition {
    Unconditional,
    NZ,
    Z,
    NC,
    C,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JumpTarget {
    Imm16(u16),
    HL, // JP (HL)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StackTarget {
    BC,
    DE,
    HL,
    AF,
}

/// 從 opcode 解碼指令
pub fn decode_instruction(opcode: u8, immediate: &[u8]) -> Option<(Instruction, usize)> {
    match opcode {
        // NOP
        0x00 => Some((Instruction::NOP, 1)),

        // LD 指令族
        0x40..=0x7F => decode_ld_reg_reg(opcode),

        // 其他指令...
        _ => None,
    }
}

/// 解碼 LD r,r' 指令 (0x40-0x7F)
fn decode_ld_reg_reg(opcode: u8) -> Option<(Instruction, usize)> {
    if opcode == 0x76 {
        // HALT
        return Some((Instruction::HALT, 1));
    }

    let dst = (opcode >> 3) & 0x07;
    let src = opcode & 0x07;

    let dst_reg = RegTarget::from_bits(dst).ok()?;
    let src_reg = RegTarget::from_bits(src).ok()?;

    Some((
        Instruction::LD(LDTarget::Reg(dst_reg), LDSource::Reg(src_reg)),
        1,
    ))
}
#[derive(Debug, Clone, Copy)]
pub enum IncDecInstruction {
    Inc(IncDecTarget),
    Dec(IncDecTarget),
}

impl IncDecInstruction {
    pub fn decode(opcode: u8) -> Option<Self> {
        match opcode {
            // INC r
            0x04 => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::B))),
            0x0C => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::C))),
            0x14 => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::D))),
            0x1C => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::E))),
            0x24 => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::H))),
            0x2C => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::L))),
            0x34 => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::HL))), // (HL)
            0x3C => Some(IncDecInstruction::Inc(IncDecTarget::Reg8(RegTarget::A))),
            // DEC r
            0x05 => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::B))),
            0x0D => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::C))),
            0x15 => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::D))),
            0x1D => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::E))),
            0x25 => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::H))),
            0x2D => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::L))),
            0x35 => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::HL))), // (HL)
            0x3D => Some(IncDecInstruction::Dec(IncDecTarget::Reg8(RegTarget::A))),
            // INC rr
            0x03 => Some(IncDecInstruction::Inc(IncDecTarget::Reg16(Reg16Target::BC))),
            0x13 => Some(IncDecInstruction::Inc(IncDecTarget::Reg16(Reg16Target::DE))),
            0x23 => Some(IncDecInstruction::Inc(IncDecTarget::Reg16(Reg16Target::HL))),
            0x33 => Some(IncDecInstruction::Inc(IncDecTarget::Reg16(Reg16Target::SP))),
            // DEC rr
            0x0B => Some(IncDecInstruction::Dec(IncDecTarget::Reg16(Reg16Target::BC))),
            0x1B => Some(IncDecInstruction::Dec(IncDecTarget::Reg16(Reg16Target::DE))),
            0x2B => Some(IncDecInstruction::Dec(IncDecTarget::Reg16(Reg16Target::HL))),
            0x3B => Some(IncDecInstruction::Dec(IncDecTarget::Reg16(Reg16Target::SP))),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum SixteenBitOp {
    AddHl,
    Inc,
    Dec,
    AddSp,
}

#[derive(Debug, Clone, Copy)]
pub enum SixteenBitInstruction {
    Op(SixteenBitOp, Reg16Target),
    AddSpImm(i8),
}

impl SixteenBitInstruction {
    pub fn decode(opcode: u8) -> Option<Self> {
        match opcode {
            0x09 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::AddHl,
                Reg16Target::BC,
            )),
            0x19 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::AddHl,
                Reg16Target::DE,
            )),
            0x29 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::AddHl,
                Reg16Target::HL,
            )),
            0x39 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::AddHl,
                Reg16Target::SP,
            )),
            0x03 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Inc,
                Reg16Target::BC,
            )),
            0x13 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Inc,
                Reg16Target::DE,
            )),
            0x23 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Inc,
                Reg16Target::HL,
            )),
            0x33 => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Inc,
                Reg16Target::SP,
            )),
            0x0B => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Dec,
                Reg16Target::BC,
            )),
            0x1B => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Dec,
                Reg16Target::DE,
            )),
            0x2B => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Dec,
                Reg16Target::HL,
            )),
            0x3B => Some(SixteenBitInstruction::Op(
                SixteenBitOp::Dec,
                Reg16Target::SP,
            )),
            _ => None,
        }
    }
}
#[derive(Debug, Clone, Copy)]
pub enum CbOp {
    Rlc,
    Rrc,
    Rl,
    Rr,
    Sla,
    Sra,
    Swap,
    Srl,
    Bit(u8),
    Res(u8),
    Set(u8),
}

#[derive(Debug, Clone, Copy)]
pub enum CbInstruction {
    Op(CbOp, RegTarget),
}

impl CbInstruction {
    pub fn decode(cb_opcode: u8) -> Option<Self> {
        let reg = RegTarget::from_bits(cb_opcode & 0x07).ok()?;
        let op_high = cb_opcode >> 3;
        let op = match op_high {
            0x00 => CbOp::Rlc,
            0x01 => CbOp::Rrc,
            0x02 => CbOp::Rl,
            0x03 => CbOp::Rr,
            0x04 => CbOp::Sla,
            0x05 => CbOp::Sra,
            0x06 => CbOp::Swap,
            0x07 => CbOp::Srl,
            0x08..=0x0F => CbOp::Bit(op_high - 0x08),
            0x10..=0x17 => CbOp::Res(op_high - 0x10),
            0x18..=0x1F => CbOp::Set(op_high - 0x18),
            _ => return None,
        };
        Some(CbInstruction::Op(op, reg))
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptOp {
    Enable,  // EI
    Disable, // DI
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptInstruction {
    Control(InterruptOp),
}

impl InterruptInstruction {
    pub fn decode(opcode: u8) -> Option<Self> {
        match opcode {
            0xF3 => Some(InterruptInstruction::Control(InterruptOp::Disable)),
            0xFB => Some(InterruptInstruction::Control(InterruptOp::Enable)),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptType {
    VBlank = 0,
    LcdStat = 1,
    Timer = 2,
    Serial = 3,
    Joypad = 4,
}

impl InterruptType {
    pub fn vector_address(&self) -> u16 {
        match self {
            InterruptType::VBlank => 0x40,
            InterruptType::LcdStat => 0x48,
            InterruptType::Timer => 0x50,
            InterruptType::Serial => 0x58,
            InterruptType::Joypad => 0x60,
        }
    }

    pub fn bit_mask(&self) -> u8 {
        1 << (*self as u8)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum InterruptService {
    Service(InterruptType),
}
#[derive(Debug, Clone, Copy)]
pub enum ControlInstruction {
    Halt,
    Stop,
}

impl ControlInstruction {
    pub fn decode(opcode: u8) -> Option<Self> {
        match opcode {
            0x76 => Some(ControlInstruction::Halt),
            0x10 => Some(ControlInstruction::Stop),
            _ => None,
        }
    }
}
