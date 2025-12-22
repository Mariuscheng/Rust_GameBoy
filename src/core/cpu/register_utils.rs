#[derive(Clone, Copy, PartialEq, Eq)]
pub enum RegTarget {
    B,
    C,
    D,
    E,
    H,
    L,
    HL,
    A,
    Immediate,
}

impl std::fmt::Debug for RegTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            RegTarget::B => "B",
            RegTarget::C => "C",
            RegTarget::D => "D",
            RegTarget::E => "E",
            RegTarget::H => "H",
            RegTarget::L => "L",
            RegTarget::HL => "HL",
            RegTarget::A => "A",
            RegTarget::Immediate => "Imm",
        };
        write!(f, "{}", s)
    }
}

impl RegTarget {
    pub fn from_bits(bits: u8) -> Result<Self, crate::core::error::Error> {
        match bits {
            0 => Ok(RegTarget::B),
            1 => Ok(RegTarget::C),
            2 => Ok(RegTarget::D),
            3 => Ok(RegTarget::E),
            4 => Ok(RegTarget::H),
            5 => Ok(RegTarget::L),
            6 => Ok(RegTarget::HL),
            7 => Ok(RegTarget::A),
            _ => Err(crate::core::error::Error::Instruction(
                crate::core::error::InstructionError::InvalidRegister(bits),
            )),
        }
    }
}

// Convert register pair bits to register pair targets
pub fn get_reg_pair(bits: u8) -> Option<(RegTarget, RegTarget)> {
    match bits & 0x0F {
        0x00 => Some((RegTarget::B, RegTarget::C)),
        0x01 => Some((RegTarget::D, RegTarget::E)),
        0x02 => Some((RegTarget::H, RegTarget::L)),
        // 0x03 => Some((RegTarget::SP, RegTarget::SP)), // SP 不是 RegTarget，這行應移除或改用其他型別
        _ => None,
    }
}

// Convert bit pattern to single register target
pub fn get_reg_target(bits: u8) -> Option<RegTarget> {
    match bits & 0x07 {
        0x00 => Some(RegTarget::B),
        0x01 => Some(RegTarget::C),
        0x02 => Some(RegTarget::D),
        0x03 => Some(RegTarget::E),
        0x04 => Some(RegTarget::H),
        0x05 => Some(RegTarget::L),
        0x06 => Some(RegTarget::HL),
        0x07 => Some(RegTarget::A),
        _ => None,
    }
}

// 標誌位操作 trait
pub trait FlagOperations {
    fn set_zero(&mut self, value: bool);
    fn set_subtract(&mut self, value: bool);
    fn set_half_carry(&mut self, value: bool);
    fn set_carry(&mut self, value: bool);
    fn get_zero(&self) -> bool;
    fn get_subtract(&self) -> bool;
    fn get_half_carry(&self) -> bool;
    fn get_carry(&self) -> bool;
}

// 計算 16 位元加法的進位
pub fn calc_16bit_carry(a: u16, b: u16, c: bool) -> bool {
    let c_in = if c { 1 } else { 0 };
    let result = (a as u32) + (b as u32) + (c_in as u32);
    result > 0xFFFF
}

// 計算 8 位元加法的半進位
pub fn calc_half_carry(a: u8, b: u8, c: bool) -> bool {
    let c_in = if c { 1 } else { 0 };
    let result = (a & 0x0F) + (b & 0x0F) + c_in;
    result > 0x0F
}

// 計算 16 位元加法的半進位
pub fn calc_16bit_half_carry(a: u16, b: u16) -> bool {
    ((a & 0x0FFF) + (b & 0x0FFF)) > 0x0FFF
}
