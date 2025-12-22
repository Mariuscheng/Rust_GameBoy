use crate::core::cpu::cpu::CPU;
use crate::core::cpu::register_utils::RegTarget;
use crate::core::cycles::CyclesType;
use crate::core::error::{Error, InstructionError, Result};
use std::io::Write;

/// 處理 LD 指令族
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        0x01 => return cpu.ld_bc_nn(),
        0x11 => return cpu.ld_de_nn(),
        0x21 => return cpu.ld_hl_nn(),
        0x31 => return cpu.ld_sp_nn(),
        0x70..=0x77 => {
            let src = opcode & 0x07;
            match RegTarget::from_bits(src) {
                Ok(source) => return cpu.ld_hl_r(source),
                Err(_e) => {
                    return Ok(4);
                }
            }
        }
        0x40..=0x7F => {
            let dst = ((opcode >> 3) & 0x07) as u8;
            let src = (opcode & 0x07) as u8;
            let target = match RegTarget::from_bits(dst) {
                Ok(t) => t,
                Err(_e) => {
                    return Ok(4);
                }
            };
            let source = match RegTarget::from_bits(src) {
                Ok(s) => s,
                Err(_e) => {
                    return Ok(4);
                }
            };
            return cpu.ld_r_r(target, source);
        }
        0x06 => return cpu.ld_r_n(RegTarget::B),
        0x0E => return cpu.ld_r_n(RegTarget::C),
        0x16 => return cpu.ld_r_n(RegTarget::D),
        0x1E => return cpu.ld_r_n(RegTarget::E),
        0x26 => return cpu.ld_r_n(RegTarget::H),
        0x2E => return cpu.ld_r_n(RegTarget::L),
        0x3E => return cpu.ld_r_n(RegTarget::A),
        0x0A => return cpu.ld_a_bc(),
        0x1A => return cpu.ld_a_de(),
        0xFA => return cpu.ld_a_nn(),
        0x02 => return cpu.ld_bc_a(),
        0x12 => return cpu.ld_de_a(),
        0xEA => return cpu.ld_nn_a(),
        0xF2 => return cpu.ld_a_c(),
        0xE2 => return cpu.ld_c_a(),
        0xE0 => return cpu.ldh_n_a(),
        0xF0 => return cpu.ldh_a_n(),
        0x22 => return cpu.ld_hli_a(),
        0x2A => return cpu.ld_a_hli(),
        0x32 => return cpu.ld_hld_a(),
        0x3A => return cpu.ld_a_hld(),
        0x36 => return cpu.ld_hl_n(),
        0xF9 => return cpu.ld_sp_hl(),
        0xF8 => return cpu.ld_hl_sp_r8(),
        0x08 => return cpu.ld_nn_sp(),
        _ => return Ok(4),
    }

    /// 實作 LD 指令相關方法
    impl CPU {
        pub fn ld_r_r(&mut self, target: RegTarget, source: RegTarget) -> Result<CyclesType> {
            let value = match source {
                RegTarget::A => self.registers.a,
                RegTarget::B => self.registers.b,
                RegTarget::C => self.registers.c,
                RegTarget::D => self.registers.d,
                RegTarget::E => self.registers.e,
                RegTarget::H => self.registers.h,
                RegTarget::L => self.registers.l,
                RegTarget::HL => {
                    let addr = self.registers.get_hl();
                    self.read_byte(addr).unwrap_or(0)
                }
                reg => {
                    // ...existing code...
                    return Ok(4);
                }
            };

            match target {
                RegTarget::A => self.registers.a = value,
                RegTarget::B => self.registers.b = value,
                RegTarget::C => self.registers.c = value,
                RegTarget::D => self.registers.d = value,
                RegTarget::E => self.registers.e = value,
                RegTarget::H => self.registers.h = value,
                RegTarget::L => self.registers.l = value,
                RegTarget::HL => {
                    let addr = self.registers.get_hl();
                    self.write_byte(addr, value)?;
                    self.log_vram_write(addr, value, &format!("ld_r_r {:?}", source))?;
                }
                reg => {
                    // ...existing code...
                }
            }
            Ok(4)
        }
    }

    impl CPU {
        pub fn ld_a_hli(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_hld_a(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_a_hld(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_a_bc(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_bc_a(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_a_de(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_de_a(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_a_nn(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_nn_a(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_a_c(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_c_a(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ldh_n_a(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ldh_a_n(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_bc_nn(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_de_nn(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_hl_nn(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_sp_nn(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
        pub fn ld_sp_hl(&mut self) -> Result<CyclesType> {
            Ok(4)
        }
    }
}

impl CPU {
    pub fn log_vram_write(&self, addr: u16, value: u8, source: &str) -> Result<()> {
        // ...existing code...
        if addr >= 0x8000 && addr <= 0x9FFF {
            let mut log_msg = format!(
                "VRAM Write: addr=0x{:04X}, value=0x{:02X}, src={}, PC=0x{:04X}, ",
                addr, value, source, self.registers.pc
            );

            // 如果在 tile data 區域
            if addr >= 0x8000 && addr <= 0x97FF {
                let tile_number = (addr - 0x8000) / 16;
                let row = ((addr - 0x8000) % 16) / 2;
                let is_high_bits = (addr - 0x8000) % 2 == 1;
                log_msg.push_str(&format!(
                    "Tile Data: tile={}, row={}, {}",
                    tile_number,
                    row,
                    if is_high_bits { "high" } else { "low" }
                ));
            }
            // 如果在 tile map 區域
            else if addr >= 0x9800 && addr <= 0x9FFF {
                let map_number = if addr >= 0x9C00 { 1 } else { 0 };
                let tile_pos = addr - if map_number == 0 { 0x9800 } else { 0x9C00 };
                log_msg.push_str(&format!("Tile Map {}: pos={}", map_number, tile_pos));
            }

            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/vram_write.log")
            {
                writeln!(&mut file, "{}", log_msg)?;
            }
        }
        Ok(())
    }
}

// --- LD/搬移指令 stub function ---
pub fn ld_a_b(cpu: &mut CPU) -> CyclesType {
    cpu.registers.a = cpu.registers.b;
    1
}

pub fn ld_b_c(cpu: &mut CPU) -> CyclesType {
    cpu.registers.b = cpu.registers.c;
    1
}

pub fn ld_c_d(cpu: &mut CPU) -> CyclesType {
    cpu.registers.c = cpu.registers.d;
    1
}

pub fn ld_d_e(cpu: &mut CPU) -> CyclesType {
    cpu.registers.d = cpu.registers.e;
    1
}

pub fn ld_e_h(cpu: &mut CPU) -> CyclesType {
    cpu.registers.e = cpu.registers.h;
    1
}

pub fn ld_h_l(cpu: &mut CPU) -> CyclesType {
    cpu.registers.h = cpu.registers.l;
    1
}

pub fn ld_l_a(cpu: &mut CPU) -> CyclesType {
    cpu.registers.l = cpu.registers.a;
    1
}

pub fn ld_a_a(cpu: &mut CPU) -> CyclesType {
    cpu.registers.a = cpu.registers.a;
    1
}

// LD r,r 指令 (40~7F)
