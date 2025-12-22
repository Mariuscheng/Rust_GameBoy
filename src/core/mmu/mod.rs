pub mod mbc5;
use crate::core::error::{Error, HardwareError, Result};
use crate::core::utils::logger::log_to_file;
use std::fs::OpenOptions;
use std::io::Write;

pub mod lcd_registers;
pub mod mbc;

pub mod mmu;

use lcd_registers::LCDRegisters;
