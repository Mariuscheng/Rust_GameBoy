use crate::core::cpu::register_utils::FlagOperations;
use crate::core::utils::logger;
// Registers 實作 FlagOperations trait
impl FlagOperations for Registers {
    fn set_zero(&mut self, value: bool) {
        self.set_flag(Flag::Z, value);
    }
    fn set_subtract(&mut self, value: bool) {
        self.set_flag(Flag::N, value);
    }
    fn set_half_carry(&mut self, value: bool) {
        self.set_flag(Flag::H, value);
    }
    fn set_carry(&mut self, value: bool) {
        self.set_flag(Flag::C, value);
    }
    fn get_zero(&self) -> bool {
        self.get_flag(Flag::Z)
    }
    fn get_subtract(&self) -> bool {
        self.get_flag(Flag::N)
    }
    fn get_half_carry(&self) -> bool {
        self.get_flag(Flag::H)
    }
    fn get_carry(&self) -> bool {
        self.get_flag(Flag::C)
    }
}
use crate::core::cpu::flags::{Flag, Flags};

#[derive(Debug, Default)]
pub struct Registers {
    pub a: u8,        // 累加器 A
    pub flags: Flags, // 標誌寄存器 F
    pub b: u8,        // B 寄存器
    pub c: u8,        // C 寄存器
    pub d: u8,        // D 寄存器
    pub e: u8,        // E 寄存器
    pub h: u8,        // H 寄存器
    pub l: u8,        // L 寄存器
    pub sp: u16,      // 堆疊指針
    pub pc: u16,      // 程式計數器
}

impl Registers {
    /// 依 index 取得暫存器值 (0:A, 1:B, 2:C, 3:D, 4:E, 5:H, 6:L, 7:flags)
    /// 依 index 取得暫存器值 (0:A, 1:B, 2:C, 3:D, 4:E, 5:H, 6:L, 7:flags)
    pub fn get_by_index(&self, idx: u8) -> u8 {
        match idx {
            0 => self.get_a(),
            1 => self.get_b(),
            2 => self.get_c(),
            3 => self.get_d(),
            4 => self.get_e(),
            5 => self.get_h(),
            6 => self.get_l(),
            7 => self.get_f(),
            _ => 0,
        }
    }
    /// 依 index 設定暫存器值 (0:A, 1:B, 2:C, 3:D, 4:E, 5:H, 6:L, 7:flags)
    pub fn set_by_index(&mut self, idx: u8, value: u8) {
        match idx {
            0 => self.set_a(value),
            1 => self.set_b(value),
            2 => self.set_c(value),
            3 => self.set_d(value),
            4 => self.set_e(value),
            5 => self.set_h(value),
            6 => self.set_l(value),
            7 => self.flags = Flags::new(value & 0xF0),
            _ => {}
        }
    }
    pub fn new() -> Self {
        let reg = Self {
            a: 0x01,
            flags: Flags::new(0xB0),
            b: 0x00,
            c: 0x13,
            d: 0x00,
            e: 0xD8,
            h: 0x01,
            l: 0x4D,
            sp: 0xFFFE,
            pc: 0x0100,
        };
        logger::log_to_file(&format!(
            "[REGISTERS_INIT] PC={:#06X} SP={:#06X} A={:02X} B={:02X} C={:02X} D={:02X} E={:02X} H={:02X} L={:02X}",
            reg.pc, reg.sp, reg.a, reg.b, reg.c, reg.d, reg.e, reg.h, reg.l
        ));
        reg
    }
    pub fn set_flag(&mut self, flag: Flag, value: bool) {
        self.flags.set(flag as u8, value);
    }
    pub fn get_flag(&self, flag: Flag) -> bool {
        self.flags.get(flag as u8)
    }
    pub fn get_af(&self) -> u16 {
        ((self.a as u16) << 8) | (self.flags.value() as u16)
    }
    pub fn set_af(&mut self, value: u16) {
        self.a = (value >> 8) as u8;
        self.flags = Flags::new(value as u8 & 0xF0);
    }
    pub fn get_bc(&self) -> u16 {
        ((self.b as u16) << 8) | (self.c as u16)
    }
    pub fn set_bc(&mut self, value: u16) {
        self.b = (value >> 8) as u8;
        self.c = value as u8;
    }
    pub fn get_de(&self) -> u16 {
        ((self.d as u16) << 8) | (self.e as u16)
    }
    pub fn set_de(&mut self, value: u16) {
        self.d = (value >> 8) as u8;
        self.e = value as u8;
    }
    pub fn get_hl(&self) -> u16 {
        ((self.h as u16) << 8) | (self.l as u16)
    }
    pub fn set_hl(&mut self, value: u16) {
        self.h = (value >> 8) as u8;
        self.l = value as u8;
    }
    pub fn get_pc(&self) -> u16 {
        self.pc
    }
    pub fn set_pc(&mut self, value: u16) {
        self.pc = value;
    }
    pub fn set_sp(&mut self, value: u16) {
        self.sp = value;
    }
    // 8位 getter/setter
    pub fn set_a(&mut self, value: u8) {
        self.a = value;
    }
    pub fn get_a(&self) -> u8 {
        self.a
    }
    pub fn set_b(&mut self, value: u8) {
        self.b = value;
    }
    pub fn get_b(&self) -> u8 {
        self.b
    }
    pub fn set_c(&mut self, value: u8) {
        self.c = value;
    }
    pub fn get_c(&self) -> u8 {
        self.c
    }
    pub fn set_d(&mut self, value: u8) {
        self.d = value;
    }
    pub fn get_d(&self) -> u8 {
        self.d
    }
    pub fn set_e(&mut self, value: u8) {
        self.e = value;
    }
    pub fn get_e(&self) -> u8 {
        self.e
    }
    pub fn set_h(&mut self, value: u8) {
        self.h = value;
    }
    pub fn get_h(&self) -> u8 {
        self.h
    }
    pub fn set_l(&mut self, value: u8) {
        self.l = value;
    }
    pub fn get_l(&self) -> u8 {
        self.l
    }
    // 組合標誌操作
    pub fn update_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.flags.set_zero(z);
        self.flags.set_subtract(n);
        self.flags.set_half_carry(h);
        self.flags.set_carry(c);
    }
    pub fn update_zero_and_carry(&mut self, z: bool, c: bool) {
        let n = self.get_flag(Flag::N);
        let h = self.get_flag(Flag::H);
        self.update_flags(z, n, h, c);
    }
    pub fn get_f(&self) -> u8 {
        self.flags.value()
    }
    pub fn get_reg(&self, reg: crate::core::cpu::register_utils::RegTarget) -> u8 {
        match reg {
            crate::core::cpu::register_utils::RegTarget::A => self.a,
            crate::core::cpu::register_utils::RegTarget::B => self.b,
            crate::core::cpu::register_utils::RegTarget::C => self.c,
            crate::core::cpu::register_utils::RegTarget::D => self.d,
            crate::core::cpu::register_utils::RegTarget::E => self.e,
            crate::core::cpu::register_utils::RegTarget::H => self.h,
            crate::core::cpu::register_utils::RegTarget::L => self.l,
            _ => 0,
        }
    }
}
