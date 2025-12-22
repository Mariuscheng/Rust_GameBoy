#![allow(unused_variables)]
#![allow(dead_code)]

use crate::core::cpu::cpu::CPU;
use crate::core::cpu::register_utils::FlagOperations;
use crate::core::cycles::{CYCLES_1, CYCLES_2, CYCLES_3, CyclesType};
use crate::core::error::{Error, InstructionError, Result};
use std::io::Write;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        // JP nn
        0xC3 => cpu.jp_nn(),

        // JP cc, nn
        0xC2 | 0xCA | 0xD2 | 0xDA => {
            let condition = (opcode >> 3) & 0x03;
            cpu.jp_cc_nn(condition)
        }

        // JP (HL)
        0xE9 => cpu.jp_hl(),

        // JR n
        0x18 => cpu.jr_n(),

        // JR cc, n
        0x20 | 0x28 | 0x30 | 0x38 => {
            let condition = (opcode >> 3) & 0x03;
            cpu.jr_cc_n(condition)
        }

        // CALL nn
        0xCD => cpu.call_nn(),

        // CALL cc,nn
        0xC4 | 0xCC | 0xD4 | 0xDC => {
            let condition = (opcode >> 3) & 0x03;
            cpu.call_cc_nn(condition)
        }

        // RET
        0xC9 => cpu.return_no_condition(),

        // RET cc
        0xC0 | 0xC8 | 0xD0 | 0xD8 => {
            let condition = (opcode >> 3) & 0x03;
            cpu.return_if_condition(condition)
        }

        // RETI
        0xD9 => cpu.return_and_enable_interrupts(),

        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

impl CPU {
    fn log_instruction(&mut self, instruction_name: &str, details: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/cpu_exec.log")
        {
            writeln!(
                file,
                "PC={:04X} | {} | {} | AF={:04X} BC={:04X} DE={:04X} HL={:04X}",
                self.registers.get_pc(),
                instruction_name,
                details,
                self.registers.get_af(),
                self.registers.get_bc(),
                self.registers.get_de(),
                self.registers.get_hl()
            )
            .ok();
        }
    }
}
