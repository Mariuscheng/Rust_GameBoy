// ...existing code from backup/core/cpu/flags.rs...
#[derive(Debug, Clone, Copy)]
pub enum Flag {
    Z = 0x80, // Zero Flag (位元 7)
    N = 0x40, // Subtract Flag (位元 6)
    H = 0x20, // Half Carry Flag (位元 5)
    C = 0x10, // Carry Flag (位元 4)
}

#[derive(Debug, Clone, Copy)]
pub struct Flags(u8);

impl Default for Flags {
    fn default() -> Self {
        Flags(0)
    }
}

impl Flags {
    pub fn new(value: u8) -> Self {
        Flags(value)
    }

    pub fn value(&self) -> u8 {
        self.0
    }

    pub fn set_value(&mut self, value: u8) {
        self.0 = value & 0xF0; // 只保留高 4 位
    } // 基本標誌操作
    pub fn set(&mut self, flag: u8, value: bool) {
        if value {
            self.0 |= flag;
        } else {
            self.0 &= !flag;
        }
    }

    pub fn get(&self, flag: u8) -> bool {
        (self.0 & flag) != 0
    }

    // Zero Flag 操作 (Z)
    pub fn zero(&self) -> bool {
        self.get(Flag::Z as u8)
    }

    pub fn set_zero(&mut self, value: bool) {
        self.set(Flag::Z as u8, value)
    }

    // Subtract Flag 操作 (N)
    pub fn subtract(&self) -> bool {
        self.get(Flag::N as u8)
    }

    pub fn set_subtract(&mut self, value: bool) {
        self.set(Flag::N as u8, value)
    }

    // Half Carry Flag 操作 (H)
    pub fn half_carry(&self) -> bool {
        self.get(Flag::H as u8)
    }

    pub fn set_half_carry(&mut self, value: bool) {
        self.set(Flag::H as u8, value)
    }

    // Carry Flag 操作 (C)
    pub fn carry(&self) -> bool {
        self.get(Flag::C as u8)
    }

    pub fn set_carry(&mut self, value: bool) {
        self.set(Flag::C as u8, value)
    }

    // 組合標誌操作
    pub fn update_flags(&mut self, z: bool, n: bool, h: bool, c: bool) {
        self.set_zero(z);
        self.set_subtract(n);
        self.set_half_carry(h);
        self.set_carry(c);
    }

    pub fn update_zero_and_carry(&mut self, z: bool, c: bool) {
        let n = self.subtract();
        let h = self.half_carry();
        self.update_flags(z, n, h, c);
    }

    // 檢查條件標誌
    pub fn check_condition(&self, condition: u8) -> bool {
        match condition {
            0 => !self.zero(),  // NZ
            1 => self.zero(),   // Z
            2 => !self.carry(), // NC
            3 => self.carry(),  // C
            _ => unreachable!(),
        }
    }
}

// 輔助函數
pub fn check_half_carry_add(a: u8, b: u8) -> bool {
    (((a & 0x0F) + (b & 0x0F)) & 0x10) == 0x10
}

pub fn check_half_carry_sub(a: u8, b: u8) -> bool {
    (a & 0x0F) < (b & 0x0F)
}

pub fn check_half_carry_16_add(a: u16, b: u16) -> bool {
    (((a & 0x0FFF) + (b & 0x0FFF)) & 0x1000) == 0x1000
}

pub fn check_carry_add(a: u8, b: u8) -> bool {
    (a as u16 + b as u16) > 0xFF
}

pub fn check_carry_sub(a: u8, b: u8) -> bool {
    a < b
}

pub fn check_carry_16_add(a: u16, b: u16) -> bool {
    (a as u32 + b as u32) > 0xFFFF
}

pub fn get_flag(flags: u8, flag: Flag) -> bool {
    flags & (flag as u8) != 0
}

pub fn set_flag(flags: &mut u8, flag: Flag, value: bool) {
    if value {
        *flags |= flag as u8;
    } else {
        *flags &= !(flag as u8);
    }
}

pub fn set_zero_flag(flags: &mut u8, value: bool) {
    set_flag(flags, Flag::Z, value);
}

pub fn set_subtract_flag(flags: &mut u8, value: bool) {
    set_flag(flags, Flag::N, value);
}

pub fn set_half_carry_flag(flags: &mut u8, value: bool) {
    set_flag(flags, Flag::H, value);
}

pub fn set_carry_flag(flags: &mut u8, value: bool) {
    set_flag(flags, Flag::C, value);
}

pub fn get_zero_flag(flags: u8) -> bool {
    get_flag(flags, Flag::Z)
}

pub fn get_subtract_flag(flags: u8) -> bool {
    get_flag(flags, Flag::N)
}

pub fn get_half_carry_flag(flags: u8) -> bool {
    get_flag(flags, Flag::H)
}

pub fn get_carry_flag(flags: u8) -> bool {
    get_flag(flags, Flag::C)
}

// --- GameBoy CPU 旗標/特殊指令 stub function ---
pub fn di(flags: &mut Flags) {
    // 禁用中斷
    // flags.ime = false; // 若有 ime 欄位
}
pub fn ei(flags: &mut Flags) {
    // 啟用中斷
    // flags.ime = true; // 若有 ime 欄位
}
pub fn cpl(flags: &mut Flags) {
    // 取反 A 暫存器
    flags.set_subtract(true);
    flags.set_half_carry(true);
}
pub fn scf(flags: &mut Flags) {
    flags.set_carry(true);
    flags.set_subtract(false);
    flags.set_half_carry(false);
}
pub fn ccf(flags: &mut Flags) {
    let carry = flags.carry();
    flags.set_carry(!carry);
    flags.set_subtract(false);
    flags.set_half_carry(false);
}
pub fn daa(_flags: &mut Flags) {
    // DAA 實作略，僅 stub
}
