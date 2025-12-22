#![allow(dead_code)]

impl RegTarget {
    pub fn from_bits(bits: u8) -> Result<Self> {
        match bits {
            0 => Ok(RegTarget::A),
            1 => Ok(RegTarget::B),
            2 => Ok(RegTarget::C),
            3 => Ok(RegTarget::D),
            4 => Ok(RegTarget::E),
            5 => Ok(RegTarget::H),
            6 => Ok(RegTarget::L),
            7 => Ok(RegTarget::F),
            _ => Err(Error::Instruction(InstructionError::InvalidRegister(bits))),
        }
    }
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub enum Error {
    // TODO: 根據實際需求擴充錯誤型別
    Unknown,
    Instruction(InstructionError),
    Hardware(HardwareError),
    Io(std::io::Error),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::Io(err)
    }
}
#[derive(Debug)]
pub enum HardwareError {
    // TODO: 擴充硬體錯誤型別
    Unknown,
    MemoryWrite(u16),
    PPU(String),
}

#[derive(Debug)]
pub enum InstructionError {
    // TODO: 擴充指令錯誤型別
    Unknown,
    InvalidOpcode(u8),
    InvalidRegister(u8),
    InvalidCondition(u8),
    Custom(String),
}

#[derive(Debug)]
pub enum RegTarget {
    // TODO: 擴充暫存器目標型別
    A,
    B,
    C,
    D,
    E,
    H,
    L,
    F,
    HL,
    BC,
    DE,
    SP,
    PC,
}
