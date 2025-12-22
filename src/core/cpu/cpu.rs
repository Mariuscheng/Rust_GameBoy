use crate::core::cpu::instruction::{
    AluRegOp, CbInstruction, CbOp, IncDecInstruction, IncDecTarget, Reg16Target,
    SimpleAluInstruction, SixteenBitInstruction, SixteenBitOp,
};
pub fn execute_simple_alu(&mut self, instr: SimpleAluInstruction) {
    match instr {
        SimpleAluInstruction::RegOp(op, reg) => {
            let a = self.registers.a;
            let val = self.registers.get_by_index(reg as u8);
            match op {
                AluRegOp::Add => {
                    let (result, carry) = a.overflowing_add(val);
                    let half_carry = ((a & 0xF) + (val & 0xF)) > 0xF;
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, false, half_carry, carry);
                }
                AluRegOp::Sub => {
                    let (result, borrow) = a.overflowing_sub(val);
                    let half_carry = (a & 0xF) < (val & 0xF);
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, true, half_carry, borrow);
                }
                AluRegOp::And => {
                    let result = a & val;
                    self.registers.a = result;
                    self.registers.update_flags(result == 0, false, true, false);
                }
                AluRegOp::Or => {
                    let result = a | val;
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, false, false, false);
                }
                AluRegOp::Xor => {
                    let result = a ^ val;
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, false, false, false);
                }
                AluRegOp::Cp => {
                    let (result, borrow) = a.overflowing_sub(val);
                    let half_carry = (a & 0xF) < (val & 0xF);
                    self.registers
                        .update_flags(result == 0, true, half_carry, borrow);
                }
                AluRegOp::Adc => {
                    let carry = if self.registers.carry() { 1 } else { 0 };
                    let (result, new_carry) = a.overflowing_add(val);
                    let (result, final_carry) = result.overflowing_add(carry);
                    let half_carry = ((a & 0xF) + (val & 0xF) + carry) > 0xF;
                    self.registers.a = result;
                    self.registers.update_flags(
                        result == 0,
                        false,
                        half_carry,
                        final_carry || new_carry,
                    );
                }
                AluRegOp::Sbc => {
                    let carry = if self.registers.carry() { 1 } else { 0 };
                    let (result, new_borrow) = a.overflowing_sub(val);
                    let (result, final_borrow) = result.overflowing_sub(carry);
                    let half_carry = ((a & 0xF) < ((val & 0xF) + carry));
                    self.registers.a = result;
                    self.registers.update_flags(
                        result == 0,
                        true,
                        half_carry,
                        final_borrow || new_borrow,
                    );
                }
                AluRegOp::Inc => {
                    if reg == RegTarget::HL {
                        // INC (HL)
                        let addr = self.registers.get_hl();
                        let val = unsafe { &*self.mmu }.read_byte(addr).unwrap_or(0);
                        let result = val.wrapping_add(1);
                        unsafe { &mut *self.mmu }.write_byte(addr, result).ok();
                        self.registers
                            .update_flags(result == 0, false, (val & 0xF) == 0xF, false);
                    } else {
                        // INC r
                        let val = self.registers.get_by_index(reg as u8);
                        let result = val.wrapping_add(1);
                        self.registers.set_by_index(reg as u8, result);
                        self.registers
                            .update_flags(result == 0, false, (val & 0xF) == 0xF, false);
                    }
                }
                AluRegOp::Dec => {
                    if reg == RegTarget::HL {
                        // DEC (HL)
                        let addr = self.registers.get_hl();
                        let val = unsafe { &*self.mmu }.read_byte(addr).unwrap_or(0);
                        let result = val.wrapping_sub(1);
                        unsafe { &mut *self.mmu }.write_byte(addr, result).ok();
                        self.registers
                            .update_flags(result == 0, true, (val & 0xF) == 0, false);
                    } else {
                        // DEC r
                        let val = self.registers.get_by_index(reg as u8);
                        let result = val.wrapping_sub(1);
                        self.registers.set_by_index(reg as u8, result);
                        self.registers
                            .update_flags(result == 0, true, (val & 0xF) == 0, false);
                    }
                }
            }
            self.registers.pc = self.registers.pc.wrapping_add(1);
        }
        SimpleAluInstruction::ImmOp(op) => {
            let a = self.registers.a;
            let val = unsafe { &*self.mmu }.read_byte(self.registers.pc.wrapping_add(1)).unwrap_or(0);
            match op {
                AluImmOp::Add => {
                    let (result, carry) = a.overflowing_add(val);
                    let half_carry = ((a & 0xF) + (val & 0xF)) > 0xF;
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, false, half_carry, carry);
                }
                AluImmOp::Adc => {
                    let carry = if self.registers.carry() { 1 } else { 0 };
                    let (result, new_carry) = a.overflowing_add(val);
                    let (result, final_carry) = result.overflowing_add(carry);
                    let half_carry = ((a & 0xF) + (val & 0xF) + carry) > 0xF;
                    self.registers.a = result;
                    self.registers.update_flags(
                        result == 0,
                        false,
                        half_carry,
                        final_carry || new_carry,
                    );
                }
                AluImmOp::Sub => {
                    let (result, borrow) = a.overflowing_sub(val);
                    let half_carry = (a & 0xF) < (val & 0xF);
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, true, half_carry, borrow);
                }
                AluImmOp::Sbc => {
                    let carry = if self.registers.carry() { 1 } else { 0 };
                    let (result, new_borrow) = a.overflowing_sub(val);
                    let (result, final_borrow) = result.overflowing_sub(carry);
                    let half_carry = ((a & 0xF) < ((val & 0xF) + carry));
                    self.registers.a = result;
                    self.registers.update_flags(
                        result == 0,
                        true,
                        half_carry,
                        final_borrow || new_borrow,
                    );
                }
                AluImmOp::And => {
                    let result = a & val;
                    self.registers.a = result;
                    self.registers.update_flags(result == 0, false, true, false);
                }
                AluImmOp::Or => {
                    let result = a | val;
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, false, false, false);
                }
                AluImmOp::Xor => {
                    let result = a ^ val;
                    self.registers.a = result;
                    self.registers
                        .update_flags(result == 0, false, false, false);
                }
                AluImmOp::Cp => {
                    let (result, borrow) = a.overflowing_sub(val);
                    let half_carry = (a & 0xF) < (val & 0xF);
                    self.registers
                        .update_flags(result == 0, true, half_carry, borrow);
                }
            }
            self.registers.pc = self.registers.pc.wrapping_add(2);
        }
}
pub fn execute_interrupt(&mut self, instr: InterruptInstruction) {
    match instr {
        InterruptInstruction::Control(op) => match op {
            InterruptOp::Disable => {
                crate::core::utils::logger::log_to_file(&format!(
                    "[DI] pc=0x{:04X} ime={} -> ime=false, scheduled=false",
                    self.registers.pc, self.ime
                ));
                self.ime = false;
                self.ime_scheduled = false;
            }
            InterruptOp::Enable => {
                crate::core::utils::logger::log_to_file(&format!(
                    "[EI] pc=0x{:04X} ime={} -> scheduled=true",
                    self.registers.pc, self.ime
                ));
                self.ime_scheduled = true;
            }
        },
    }
    self.registers.pc = self.registers.pc.wrapping_add(1);
}
pub fn execute_interrupt_service(&mut self, service: InterruptService) {
    match service {
        InterruptService::Service(interrupt_type) => {
            let old_if = unsafe { &*self.mmu }.io[0x0F];
            let pc_before = self.registers.pc;
            crate::core::utils::logger::log_to_file(&format!(
                "[ISR] Servicing {:?} at pc=0x{:04X}, IF=0x{:02X} -> clearing bit 0x{:02X}",
                interrupt_type, pc_before, old_if, interrupt_type.bit_mask()
            ));
            let mmu = unsafe { &mut *self.mmu };
            // 清除 IF 對應 bit
            mmu.io[0x0F] &= !interrupt_type.bit_mask();
            self.ime = false; // 關閉中斷
            // Push PC 到 stack
            let pc = self.registers.pc;
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            mmu.write_byte(self.registers.sp, (pc & 0xFF) as u8).ok();
            mmu.write_byte(self.registers.sp + 1, (pc >> 8) as u8).ok();
            // 跳轉到中斷向量
            let new_pc = interrupt_type.vector_address();
            self.registers.pc = new_pc;
            crate::core::utils::logger::log_to_file(&format!(
                "[ISR] Pushed pc=0x{:04X} to sp=0x{:04X}, jumped to 0x{:04X}, ime=false",
                pc, self.registers.sp, new_pc
            ));
        }
    }
}
pub fn execute_control(&mut self, instr: ControlInstruction) {
    match instr {
        ControlInstruction::Halt => {
            let mmu = unsafe { &*self.mmu };
            let ie = mmu.ie;
            let iflags = mmu.io[0x0F];
            crate::core::utils::logger::log_to_file(&format!(
                "[HALT] Entered at pc=0x{:04X}, ime={}, IE=0x{:02X}, IF=0x{:02X}",
                self.registers.pc, self.ime, ie, iflags
            ));
            self.halted = true;
        }
        ControlInstruction::Stop => {
            // STOP 指令的實現（簡化版）
            // 在真實的 Game Boy 中，這會停止 CPU 直到按鈕被按下
            // 這裡我們只是簡單地繼續執行
        }
    }
    self.registers.pc = self.registers.pc.wrapping_add(1);
}
pub fn execute_simple_inc_dec(&mut self, instr: IncDecInstruction) {
    match instr {
        IncDecInstruction::Inc(target) => match target {
            IncDecTarget::Reg8(reg) => {
                if reg == RegTarget::HL {
                    // INC (HL)
                    let addr = self.registers.get_hl();
                    let val = self.memory.read_byte(addr).unwrap_or(0);
                    let result = val.wrapping_add(1);
                    self.memory.write_byte(addr, result).ok();
                    let half_carry = (val & 0xF) == 0xF;
                    self.registers.update_flags(
                        result == 0,
                        false,
                        half_carry,
                        self.registers.flags.get_carry(),
                    );
                } else {
                    // INC r
                    let val = self.registers.get_by_index(reg as u8);
                    let result = val.wrapping_add(1);
                    self.registers.set_by_index(reg as u8, result);
                    let half_carry = (val & 0xF) == 0xF;
                    self.registers.update_flags(
                        result == 0,
                        false,
                        half_carry,
                        self.registers.flags.get_carry(),
                    );
                }
            }
            IncDecTarget::Reg16(reg16) => {
                // INC rr - no flags affected
                match reg16 {
                    Reg16Target::BC => {
                        let val = self.registers.get_bc().wrapping_add(1);
                        self.registers.set_bc(val);
                    }
                    Reg16Target::DE => {
                        let val = self.registers.get_de().wrapping_add(1);
                        self.registers.set_de(val);
                    }
                    Reg16Target::HL => {
                        let val = self.registers.get_hl().wrapping_add(1);
                        self.registers.set_hl(val);
                    }
                    Reg16Target::SP => {
                        self.registers.sp = self.registers.sp.wrapping_add(1);
                    }
                    _ => {} // AF not used for INC/DEC
                }
            }
        },
        IncDecInstruction::Dec(target) => match target {
            IncDecTarget::Reg8(reg) => {
                if reg == RegTarget::HL {
                    // DEC (HL)
                    let addr = self.registers.get_hl();
                    let val = self.memory.read_byte(addr).unwrap_or(0);
                    let result = val.wrapping_sub(1);
                    self.memory.write_byte(addr, result).ok();
                    let half_carry = (val & 0xF) == 0x0;
                    self.registers.update_flags(
                        result == 0,
                        true,
                        half_carry,
                        self.registers.flags.get_carry(),
                    );
                } else {
                    // DEC r
                    let val = self.registers.get_by_index(reg as u8);
                    let result = val.wrapping_sub(1);
                    self.registers.set_by_index(reg as u8, result);
                    let half_carry = (val & 0xF) == 0x0;
                    self.registers.update_flags(
                        result == 0,
                        true,
                        half_carry,
                        self.registers.flags.get_carry(),
                    );
                }
            }
            IncDecTarget::Reg16(reg16) => {
                // DEC rr - no flags affected
                match reg16 {
                    Reg16Target::BC => {
                        let val = self.registers.get_bc().wrapping_sub(1);
                        self.registers.set_bc(val);
                    }
                    Reg16Target::DE => {
                        let val = self.registers.get_de().wrapping_sub(1);
                        self.registers.set_de(val);
                    }
                    Reg16Target::HL => {
                        let val = self.registers.get_hl().wrapping_sub(1);
                        self.registers.set_hl(val);
                    }
                    Reg16Target::SP => {
                        self.registers.sp = self.registers.sp.wrapping_sub(1);
                    }
                    _ => {} // AF not used for INC/DEC
                }
            }
        },
    }
    self.registers.pc = self.registers.pc.wrapping_add(1);
}
use crate::core::utils::logger;
// 輔助方法：取得對應暫存器值
impl CPU {
    fn get_reg(&self, idx: u8) -> u8 {
        match idx {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => 0, // (HL) 需經 MMU 讀取，這裡暫以 0 代表
            7 => self.registers.a,
            _ => 0,
        }
    }
}
// 指令解碼 trait 與 enum
pub trait InstructionDecoder {
    fn decode(&self, opcode: u8) -> Instruction;
    fn execute(&mut self, instr: Instruction);
}

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Nop,
    LdA(u8),
    // ... 其他指令 ...
    Unknown(u8),
}

// CPU 實作 InstructionDecoder trait
impl InstructionDecoder for CPU {
    fn decode(&self, opcode: u8) -> Instruction {
        match opcode {
            0x00 => Instruction::Nop,
            0x3E => Instruction::LdA(0), // 0x3E 為 LD A, n，暫以 0 代表，實際需 fetch
            _ => Instruction::Unknown(opcode),
        }
    }
    fn execute(&mut self, instr: Instruction) {
        match instr {
            Instruction::Nop => {
                // NOP 不做事
            }
            Instruction::LdA(val) => {
                self.registers.a = val;
            }
            Instruction::Unknown(op) => {
                logger::log_to_file(&format!("[WARN] Unknown opcode: 0x{:02X}", op));
            }
        }
    }
}
use crate::core::cpu::registers::Registers;
use crate::core::mmu::mmu::MMU;

impl CPU {
    // CPU::new 增加 log，記錄 PC、SP、A、ROM buffer 狀態（如可取得）
    pub fn new_with_log(mmu: &mut crate::core::mmu::mmu::MMU) -> Self {
        let registers = Registers::new();
        let cpu = CPU {
            ime: false,
            halted: false,
            stopped: false,
            registers,
            mmu: mmu as *mut crate::core::mmu::mmu::MMU,
        };
        log::info!(
            "[CPU INIT] (DMG boot) PC={:04X} SP={:04X} A={:02X} F={:02X} B={:02X} C={:02X} D={:02X} E={:02X} H={:02X} L={:02X}",
            cpu.registers.pc,
            cpu.registers.sp,
            cpu.registers.a,
            cpu.registers.flags.value(),
            cpu.registers.b,
            cpu.registers.c,
            cpu.registers.d,
            cpu.registers.e,
            cpu.registers.h,
            cpu.registers.l
        );
        // 若能取得 ROM buffer，可額外 log ROM[0..16]
        let rom_buf = unsafe { (*cpu.mmu).get_rom_buffer() };
        if let Some(rom) = rom_buf.get(0..16) {
            logger::log_to_file(&format!("[CPU_INIT_ROM] ROM[0..16]={:02X?}", rom));
        }
        cpu
    }
    // --- jump.rs 需要的 method stub ---
    pub fn jp_nn(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("jp_nn not implemented");
    }
    pub fn jp_cc_nn(
        &mut self,
        _condition: u8,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("jp_cc_nn not implemented");
    }
    pub fn jp_hl(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("jp_hl not implemented");
    }
    pub fn jr_n(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("jr_n not implemented");
    }
    pub fn jr_cc_n(
        &mut self,
        _condition: u8,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("jr_cc_n not implemented");
    }
    pub fn call_nn(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("call_nn not implemented");
    }
    pub fn call_cc_nn(
        &mut self,
        _condition: u8,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("call_cc_nn not implemented");
    }
    pub fn return_no_condition(
        &mut self,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("return_no_condition not implemented");
    }
    pub fn return_if_condition(
        &mut self,
        _condition: u8,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("return_if_condition not implemented");
    }
    pub fn return_and_enable_interrupts(
        &mut self,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        unimplemented!("return_and_enable_interrupts not implemented");
    }
    /// 推進 CPU 一步（stub，可串接 decode_and_execute）
    pub fn step(&mut self) {
        // HALT 處理：若處於 halted，僅在中斷發生時喚醒或服務中斷
        let mmu = unsafe { &mut *self.mmu };
        if self.halted {
            let ie = mmu.ie;
            let iflags = mmu.io[0x0F];
            if (ie & iflags) != 0 {
                // 如果有 pending 中斷，喚醒 CPU
                self.halted = false;
                // 若 IME 為真且有可服務的中斷，則服務第一個可用中斷
                if self.ime && (ie & iflags) != 0 {
                    let interrupt_types = [
                        InterruptType::VBlank,
                        InterruptType::LcdStat,
                        InterruptType::Timer,
                        InterruptType::Serial,
                        InterruptType::Joypad,
                    ];
                    for interrupt_type in interrupt_types.iter() {
                        if (ie & iflags) & interrupt_type.bit_mask() != 0 {
                            crate::core::utils::logger::log_to_file(&format!(
                                "[HALT] Wake -> service {:?} (ime=true, IE=0x{:02X}, IF=0x{:02X})",
                                interrupt_type, ie, iflags
                            ));
                            self.execute_interrupt_service(InterruptService::Service(*interrupt_type));
                            break;
                        }
                    }
                } else {
                    // 只喚醒但不服務（IME = 0 的情況）
                    crate::core::utils::logger::log_to_file(&format!(
                        "[HALT] Wake (ime=false, IE=0x{:02X}, IF=0x{:02X}) - CPU resumes without servicing",
                        ie, iflags
                    ));
                }
            } else {
                // 仍然 halted，消耗一些週期（簡化：直接返回）
                // 在真實硬體上需要推進定時器；此處簡化處理
                return;
            }
        }

        // 取出 PC 指向的 opcode（直接從 MMU 讀，不在這裡自動遞增 PC）
        let opcode = mmu.read_byte(self.registers.pc).unwrap_or(0);
        if let Some(instr) = SimpleAluInstruction::decode(opcode) {
            self.execute_simple_alu(instr);
        } else if let Some(instr) = InterruptInstruction::decode(opcode) {
            self.execute_interrupt(instr);
        } else if let Some(instr) = ControlInstruction::decode(opcode) {
            self.execute_control(instr);
        } else if let Some(instr) = SixteenBitInstruction::decode(opcode) {
            self.execute_sixteen_bit(instr);
        } else if opcode == 0xE8 {
            // ADD SP, n
            let imm = self.fetch_byte().unwrap_or(0) as i8;
            self.execute_sixteen_bit(SixteenBitInstruction::AddSpImm(imm));
        } else if let Some(instr) = IncDecInstruction::decode(opcode) {
            self.execute_simple_inc_dec(instr);
        } else if opcode == 0xCB {
            let cb_opcode = self.fetch_byte().unwrap_or(0);
            if let Some(instr) = CbInstruction::decode(cb_opcode) {
                self.execute_cb(instr);
            }
        } else {
            // Try arithmetic dispatch first
            if let Ok(cycles) = crate::core::cpu::arithmetic::dispatch(self, opcode) {
                // Arithmetic instruction handled
                unsafe {
                    (*self.mmu).timer_tick(cycles as u32);
                }
            } else {
                // Fall back to decode_and_execute for other instructions
                crate::core::cpu::decode_and_execute::decode_and_execute(
                    self,
                    unsafe { &mut *self.mmu },
                    opcode,
                );
            }
        }

        // --- EI 延遲生效邏輯 ---
        if self.ime_scheduled {
            crate::core::utils::logger::log_to_file(&format!(
                "[IME] scheduled=true -> ime=true at pc=0x{:04X}",
                self.registers.pc
            ));
            self.ime = true;
            self.ime_scheduled = false;
        }

        // --- 中斷處理（在指令執行後進行）---
        // 取得 IE (0xFFFF) 與 IF (0xFF0F)
        let mmu = unsafe { &mut *self.mmu };
        let ie = mmu.ie;
        let iflags = mmu.io[0x0F];
        if self.ime && (ie & iflags) != 0 {
            // 依 Game Boy 優先序處理中斷
            let interrupt_types = [
                InterruptType::VBlank,
                InterruptType::LcdStat,
                InterruptType::Timer,
                InterruptType::Serial,
                InterruptType::Joypad,
            ];
            for interrupt_type in interrupt_types.iter() {
                if (ie & iflags) & interrupt_type.bit_mask() != 0 {
                    // 服務中斷
                    self.execute_interrupt_service(InterruptService::Service(*interrupt_type));
                    break;
                }
            }
        }
        // 執行完指令後推進計時器
        unsafe {
            (*self.mmu).timer_tick(4);
        } // 假設每指令 4 cycles，可依實際指令 cycles 調整
    }
    /// 根據 opcode 回傳指令長度（Game Boy 指令集，暫時只處理常見指令）
    pub fn opcode_len(opcode: u8) -> usize {
        match opcode {
            // 1-byte 指令
            0x00 | 0x07 | 0x0F | 0x17 | 0x1F | 0x27 | 0x2F | 0x37 | 0x3F | 0x76 | 0xC9 | 0xD9 => 1,
            // 2-byte指令（如 LD r, n; JP nn; CALL nn; LD (nn), A）
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E | 0xC3 | 0xCD | 0xEA | 0xFA => 3,
            // 3-byte指令（如 LD HL, SP+r8）
            0xF8 => 2,
            // 其他常見指令
            0x21 | 0x31 | 0x01 | 0x11 => 3,
            // 2-byte指令（如 JR r8, LD (a16), SP）
            0x18 | 0x20 | 0x28 | 0x30 | 0xE0 | 0xE2 | 0xF0 | 0xF2 => 2,
            // CB 前綴指令
            0xCB => 2,
            // 預設 1-byte
            _ => 1,
        }
    }
    /// 根據 opcode 回傳指令名稱
    pub fn decode_opcode(opcode: u8) -> &'static str {
        match opcode {
            0x00 => "NOP",
            0x01 => "LD BC,nn",
            0x02 => "LD (BC),A",
            0x03 => "INC BC",
            0x04 => "INC B",
            0x05 => "DEC B",
            0x06 => "LD B,n",
            0x07 => "RLCA",
            0x08 => "LD (nn),SP",
            0x09 => "ADD HL,BC",
            0x0A => "LD A,(BC)",
            0x0B => "DEC BC",
            0x0C => "INC C",
            0x0D => "DEC C",
            0x0E => "LD C,n",
            0x0F => "RRCA",
            0x10 => "STOP",
            0x11 => "LD DE,nn",
            0x12 => "LD (DE),A",
            0x13 => "INC DE",
            0x14 => "INC D",
            0x15 => "DEC D",
            0x16 => "LD D,n",
            0x17 => "RLA",
            0x18 => "JR n",
            0x19 => "ADD HL,DE",
            0x1A => "LD A,(DE)",
            0x1B => "DEC DE",
            0x1C => "INC E",
            0x1D => "DEC E",
            0x1E => "LD E,n",
            0x1F => "RRA",
            0x20 => "JR NZ,n",
            0x21 => "LD HL,nn",
            0x22 => "LD (HL+),A",
            0x23 => "INC HL",
            0x24 => "INC H",
            0x25 => "DEC H",
            0x26 => "LD H,n",
            0x27 => "DAA",
            0x28 => "JR Z,n",
            0x29 => "ADD HL,HL",
            0x2A => "LD A,(HL+)",
            0x2B => "DEC HL",
            0x2C => "INC L",
            0x2D => "DEC L",
            0x2E => "LD L,n",
            0x2F => "CPL",
            0x30 => "JR NC,n",
            0x31 => "LD SP,nn",
            0x32 => "LD (HL-),A",
            0x33 => "INC SP",
            0x34 => "INC (HL)",
            0x35 => "DEC (HL)",
            0x36 => "LD (HL),n",
            0x37 => "SCF",
            0x38 => "JR C,n",
            0x39 => "ADD HL,SP",
            0x3A => "LD A,(HL-)",
            0x3B => "DEC SP",
            0x3C => "INC A",
            0x3D => "DEC A",
            0x3E => "LD A,n",
            0x3F => "CCF",
            0x76 => "HALT",
            0xC3 => "JP nn",
            0xC9 => "RET",
            0xCD => "CALL nn",
            0xD9 => "RETI",
            0xE9 => "JP (HL)",
            0xF3 => "DI",
            0xFB => "EI",
            0xAF => "XOR A",
            0xA7 => "AND A",
            0xB7 => "OR A",
            0x97 => "SUB A",
            0x87 => "ADD A,A",
            0xC5 => "PUSH BC",
            0xD5 => "PUSH DE",
            0xE5 => "PUSH HL",
            0xF5 => "PUSH AF",
            0xC1 => "POP BC",
            0xD1 => "POP DE",
            0xE1 => "POP HL",
            0xF1 => "POP AF",
            _ => "未知指令",
        }
    }
}

pub struct CPU {
    pub ime: bool,
    pub ime_scheduled: bool, // EI 延遲啟用
    pub halted: bool,
    pub stopped: bool,
    pub registers: Registers,
    pub mmu: *mut MMU, // 直接持有裸指標
}

impl CPU {
    // LD r, n : 將立即值 n 寫入指定暫存器
    pub fn ld_r_n(
        &mut self,
        target: crate::core::cpu::register_utils::RegTarget,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        let value = self.fetch_byte()?;
        self.set_reg_value(target, value);
        logger::log_to_file(&format!(
            "[CPU] LD r, n: target={:?}, value={:#04X}",
            target, value
        ));
        Ok(crate::core::cycles::CYCLES_2)
    }

    // LD (HL+), A : 將 A 寫入 HL 指向記憶體，HL++
    pub fn ld_hli_a(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        let hl = self.registers.get_hl();
        let value = self.registers.a;
        self.write_byte(hl, value)?;
        self.registers.set_hl(hl.wrapping_add(1));
        logger::log_to_file(&format!(
            "[CPU] LD (HL+), A: HL={:#06X}, value={:#04X}",
            hl, value
        ));
        Ok(crate::core::cycles::CYCLES_2)
    }

    // LD HL, SP+r8 : HL = SP + signed r8
    pub fn ld_hl_sp_r8(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        let sp = self.registers.sp;
        let r8 = self.fetch_byte()? as i8 as i16;
        let hl = (sp as i16).wrapping_add(r8) as u16;
        self.registers.set_hl(hl);
        logger::log_to_file(&format!(
            "[CPU] LD HL, SP+r8: SP={:#06X}, r8={:+}, HL={:#06X}",
            sp, r8, hl
        ));
        Ok(crate::core::cycles::CYCLES_3)
    }

    // LD (nn), SP : 將 SP 寫入絕對位址 nn
    pub fn ld_nn_sp(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        let addr = self.fetch_word()?;
        let sp = self.registers.sp;
        self.write_byte(addr, (sp & 0xFF) as u8)?;
        self.write_byte(addr + 1, (sp >> 8) as u8)?;
        logger::log_to_file(&format!(
            "[CPU] LD (nn), SP: addr={:#06X}, SP={:#06X}",
            addr, sp
        ));
        Ok(crate::core::cycles::CYCLES_4)
    }
    // LD (HL), n : 將立即值 n 寫入 HL 指向的記憶體 (通常 VRAM)
    pub fn ld_hl_n(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        let hl = self.registers.get_hl();
        let n = self.fetch_byte()?;
        self.write_byte(hl, n)?;
        logger::log_to_file(&format!("[CPU] LD (HL), n: HL={:#06X}, n={:#04X}", hl, n));
        logger::log_to_file(&format!(
            "[TRACE] LD (HL), n executed: PC={:#06X}, HL={:#06X}, n={:#04X}",
            self.registers.pc, hl, n
        ));
        Ok(crate::core::cycles::CYCLES_3)
    }

    // LD (HL), r : 將暫存器 r 的值寫入 HL 指向的記憶體 (通常 VRAM)
    pub fn ld_hl_r(
        &mut self,
        r: crate::core::cpu::register_utils::RegTarget,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        let hl = self.registers.get_hl();
        let value = self.registers.get_reg(r);
        self.write_byte(hl, value)?;
        logger::log_to_file(&format!(
            "[CPU] LD (HL), r: HL={:#06X}, r={:?}, value={:#04X}",
            hl, r, value
        ));
        Ok(crate::core::cycles::CYCLES_2)
    }

    // LD (a16), A : 將 A 暫存器的值寫入 16-bit 絕對位址 (通常 VRAM)
    pub fn ld_a16_a(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        let addr = self.fetch_word()?;
        let value = self.registers.a;
        self.write_byte(addr, value)?;
        logger::log_to_file(&format!(
            "[CPU] LD (a16), A: addr={:#06X}, value={:#04X}",
            addr, value
        ));
        logger::log_to_file(&format!(
            "[TRACE] LD (a16), A executed: PC={:#06X}, a16={:#06X}, value={:#04X}",
            self.registers.pc, addr, value
        ));
        Ok(crate::core::cycles::CYCLES_4)
    }

    pub fn new(mmu: &mut MMU) -> Self {
        CPU {
            ime: false,
            ime_scheduled: false,
            halted: false,
            stopped: false,
            registers: Registers::default(),
            mmu: mmu as *mut MMU,
        }
    }
    /// 設定暫存器值（供 bit 指令族使用）
    pub fn set_reg_value(&mut self, reg: super::register_utils::RegTarget, value: u8) {
        match reg {
            super::register_utils::RegTarget::A => self.registers.a = value,
            super::register_utils::RegTarget::B => self.registers.b = value,
            super::register_utils::RegTarget::C => self.registers.c = value,
            super::register_utils::RegTarget::D => self.registers.d = value,
            super::register_utils::RegTarget::E => self.registers.e = value,
            super::register_utils::RegTarget::H => self.registers.h = value,
            super::register_utils::RegTarget::L => self.registers.l = value,
            super::register_utils::RegTarget::HL => {
                let addr = self.registers.get_hl();
                let _ = self.write_byte(addr, value);
            }
            _ => {}
        }
    }
    #[allow(dead_code)]
    pub fn rlc_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn rrc_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn rl_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn rr_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn sla_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn sra_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn swap_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn srl_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn bit_b_r(
        &mut self,
        _bit: u8,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn res_b_r(
        &mut self,
        _bit: u8,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    #[allow(dead_code)]
    pub fn set_b_r(
        &mut self,
        _bit: u8,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        8
    }
    pub fn pop_word(&mut self) -> crate::core::error::Result<u16> {
        // TODO: 實作堆疊彈出
        Ok(0)
    }
    pub fn push_word(&mut self, value: u16) -> crate::core::error::Result<()> {
        // TODO: 實作堆疊推入
        let _ = value; // 避免 unused variable 編譯錯誤
        Ok(())
    }
    pub fn pop_bc(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    pub fn pop_de(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    pub fn pop_hl(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    pub fn pop_af(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    pub fn push_bc(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    pub fn push_de(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    pub fn push_hl(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    pub fn push_af(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        Ok(crate::core::cycles::CYCLES_1)
    }
    /// 取得可變暫存器引用
    pub fn registers_mut(&mut self) -> &mut Registers {
        &mut self.registers
    }

    /// 取得不可變暫存器引用
    pub fn registers(&self) -> &Registers {
        &self.registers
    }

    // Rc/RefCell 已移除，若需要可改為：
    pub fn mmu_ptr(&self) -> *mut MMU {
        self.mmu
    }

    // ...existing code...

    pub fn fetch_word(&mut self) -> crate::core::error::Result<u16> {
        // 以 PC 為位址連續讀取兩個 byte（小端序），並自動遞增 PC
        let low = self.fetch_byte()? as u16;
        let high = self.fetch_byte()? as u16;
        Ok((high << 8) | low)
    }

    pub fn read_byte(&mut self, addr: u16) -> crate::core::error::Result<u8> {
        unsafe { &*self.mmu }.read_byte(addr)
    }

    /// 寫入一個 byte 到記憶體
    pub fn write_byte(&mut self, addr: u16, value: u8) -> crate::core::error::Result<()> {
        unsafe { &mut *self.mmu }.write_byte(addr, value)
    }

    pub fn and_a_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        // TODO: 實作 AND A,r
        Ok(4)
    }
    pub fn or_a_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        // TODO: 實作 OR A,r
        Ok(4)
    }
    pub fn cpl(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        // TODO: 實作 CPL
        Ok(4)
    }
    pub fn scf(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        // TODO: 實作 SCF
        Ok(4)
    }
    pub fn ccf(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        // TODO: 實作 CCF
        Ok(4)
    }
    pub fn daa(&mut self) -> crate::core::error::Result<crate::core::cycles::CyclesType> {
        // TODO: 實作 DAA
        Ok(4)
    }

    // 補齊 fetch_byte stub function
    pub fn fetch_byte(&mut self) -> Result<u8, crate::core::error::Error> {
        // 真正從 MMU 讀取 PC 指向的 byte，並自動遞增 PC
        let pc_before = self.registers.pc;
        let byte = unsafe { &*self.mmu }.read_byte(self.registers.pc)?;
        self.registers.pc = self.registers.pc.wrapping_add(1);
        logger::log_to_file(&format!(
            "[fetch_byte] PC={:04X} -> {:04X}, opcode={:02X}",
            pc_before, self.registers.pc, byte
        ));
        Ok(byte)
    }

    // arithmetic.rs 相關方法：直接呼叫 arithmetic.rs 實作
    pub fn dec_r(
        &mut self,
        target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        crate::core::cpu::arithmetic::dec_r(self, target)
    }
    pub fn add_a_r(
        &mut self,
        target: super::register_utils::RegTarget,
        use_carry: bool,
    ) -> crate::core::cycles::CyclesType {
        crate::core::cpu::arithmetic::add_a_r(self, target, use_carry)
    }
    pub fn sub_a_r(
        &mut self,
        target: super::register_utils::RegTarget,
        use_carry: bool,
    ) -> crate::core::cycles::CyclesType {
        crate::core::cpu::arithmetic::sub_a_r(self, target, use_carry)
    }
    pub fn add_a_n(&mut self, use_carry: bool) -> crate::core::cycles::CyclesType {
        crate::core::cpu::arithmetic::add_a_n(self, use_carry)
    }
    pub fn sub_a_n(&mut self, use_carry: bool) -> crate::core::cycles::CyclesType {
        crate::core::cpu::arithmetic::sub_a_n(self, use_carry)
    }

    // cb.rs 相關 stub
    pub fn cb_misc(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        0
    }

    // control.rs 相關 stub

    // io.rs 相關方法：直接呼叫 io.rs 實作
    pub fn read_joypad(&mut self) -> crate::core::cycles::CyclesType {
        crate::core::cpu::io::read_joypad(self)
    }
    pub fn write_joypad(&mut self) -> crate::core::cycles::CyclesType {
        crate::core::cpu::io::write_joypad(self)
    }
    pub fn read_serial(&mut self) -> crate::core::cycles::CyclesType {
        crate::core::cpu::io::read_serial(self)
    }
    pub fn write_serial(&mut self) -> crate::core::cycles::CyclesType {
        crate::core::cpu::io::write_serial(self)
    }

    // stack.rs 相關 stub
    pub fn push_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        0
    }
    pub fn pop_r(
        &mut self,
        _target: super::register_utils::RegTarget,
    ) -> crate::core::cycles::CyclesType {
        0
    }
}
pub fn execute_sixteen_bit(&mut self, instr: SixteenBitInstruction) {
    match instr {
        SixteenBitInstruction::Op(op, reg) => {
            match op {
                SixteenBitOp::AddHl => {
                    let hl = self.registers.get_hl();
                    let val = self.registers.get_reg16(reg);
                    let (result, carry) = hl.overflowing_add(val);
                    let half_carry = ((hl & 0xFFF) + (val & 0xFFF)) > 0xFFF;
                    self.registers.set_hl(result);
                    self.registers
                        .update_flags(self.registers.zero(), false, half_carry, carry);
                }
                SixteenBitOp::Inc => {
                    let val = self.registers.get_reg16(reg);
                    let result = val.wrapping_add(1);
                    self.registers.set_reg16(reg, result);
                }
                SixteenBitOp::Dec => {
                    let val = self.registers.get_reg16(reg);
                    let result = val.wrapping_sub(1);
                    self.registers.set_reg16(reg, result);
                }
                SixteenBitOp::AddSp => {
                    // This should not happen here, handled separately
                }
            }
            self.registers.pc = self.registers.pc.wrapping_add(1);
        }
        SixteenBitInstruction::AddSpImm(imm) => {
            let sp = self.registers.sp;
            let imm_u16 = imm as u16;
            let result = sp.wrapping_add(imm_u16);
            let half_carry = ((sp & 0xF) + (imm_u16 & 0xF)) > 0xF;
            let carry = ((sp & 0xFF) + (imm_u16 & 0xFF)) > 0xFF;
            self.registers.sp = result;
            self.registers.update_flags(false, false, half_carry, carry);
            self.registers.pc = self.registers.pc.wrapping_add(2);
        }
    }
}
pub fn execute_cb(&mut self, instr: CbInstruction) {
    match instr {
        CbInstruction::Op(op, reg) => {
            let mut val = if reg == RegTarget::HL {
                unsafe { &*self.mmu }
                    .read_byte(self.registers.get_hl())
                    .unwrap_or(0)
            } else {
                self.registers.get_by_index(reg as u8)
            };

            let result = match op {
                CbOp::Rlc => {
                    let carry = (val & 0x80) != 0;
                    val = (val << 1) | if carry { 1 } else { 0 };
                    self.registers.update_flags(val == 0, false, false, carry);
                    val
                }
                CbOp::Rrc => {
                    let carry = (val & 0x01) != 0;
                    val = (val >> 1) | if carry { 0x80 } else { 0 };
                    self.registers.update_flags(val == 0, false, false, carry);
                    val
                }
                CbOp::Rl => {
                    let old_carry = self.registers.carry();
                    let carry = (val & 0x80) != 0;
                    val = (val << 1) | if old_carry { 1 } else { 0 };
                    self.registers.update_flags(val == 0, false, false, carry);
                    val
                }
                CbOp::Rr => {
                    let old_carry = self.registers.carry();
                    let carry = (val & 0x01) != 0;
                    val = (val >> 1) | if old_carry { 0x80 } else { 0 };
                    self.registers.update_flags(val == 0, false, false, carry);
                    val
                }
                CbOp::Sla => {
                    let carry = (val & 0x80) != 0;
                    val <<= 1;
                    self.registers.update_flags(val == 0, false, false, carry);
                    val
                }
                CbOp::Sra => {
                    let carry = (val & 0x01) != 0;
                    val = (val >> 1) | (val & 0x80);
                    self.registers.update_flags(val == 0, false, false, carry);
                    val
                }
                CbOp::Swap => {
                    val = ((val & 0x0F) << 4) | ((val & 0xF0) >> 4);
                    self.registers.update_flags(val == 0, false, false, false);
                    val
                }
                CbOp::Srl => {
                    let carry = (val & 0x01) != 0;
                    val >>= 1;
                    self.registers.update_flags(val == 0, false, false, carry);
                    val
                }
                CbOp::Bit(bit) => {
                    let bit_set = (val & (1 << bit)) != 0;
                    self.registers
                        .update_flags(!bit_set, false, true, self.registers.carry());
                    val
                }
                CbOp::Res(bit) => val & !(1 << bit),
                CbOp::Set(bit) => val | (1 << bit),
            };

            if reg == RegTarget::HL {
                unsafe { &mut *self.mmu }
                    .write_byte(self.registers.get_hl(), result)
                    .ok();
            } else {
                self.registers.set_by_index(reg as u8, result);
            }

            self.registers.pc = self.registers.pc.wrapping_add(2);
        }
    }
}
