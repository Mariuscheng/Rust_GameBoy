
use super::cpu::CPU;
use crate::core::cpu::register_utils::FlagOperations;
use crate::core::cycles::CyclesType;
use crate::core::error::{Error, InstructionError, Result};


/// 計時器指令分派
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> crate::core::error::Result<CyclesType> {
    match opcode {
        0x04 | 0x05 | 0x06 => Ok(4),
        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}
