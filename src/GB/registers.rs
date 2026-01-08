use bitflags::bitflags;

bitflags! {
    #[derive(Default, Copy, Clone)]
    pub struct Flags: u8 {
        const Z = 0x80; // Zero
        const N = 0x40; // Subtract
        const H = 0x20; // Half-carry
        const C = 0x10; // Carry
    }
}
macro_rules! get_set {
    ($reg:ident, $get_name:ident, $set_name:ident, $size:ty) => {
        pub fn $get_name(&self) -> $size {
            self.$reg
        }

        pub fn $set_name(&mut self, val: $size) {
            self.$reg = val;
        }
    };
}

macro_rules! get_set_dual {
    ($reg1:ident, $reg2:ident, $get_name:ident, $set_name:ident) => {
        pub fn $get_name(&self) -> u16 {
            (self.$reg1 as u16) << 8 | self.$reg2 as u16
        }

        pub fn $set_name(&mut self, val: u16) {
            self.$reg1 = (val >> 8) as u8;
            self.$reg2 = val as u8;
        }
    };
}

pub struct Registers {
    a: u8,
    b: u8,
    c: u8,
    d: u8,
    e: u8,
    f: Flags,
    h: u8,
    l: u8,
    sp: u16,
    pc: u16,
}

impl Registers {
    pub fn new() -> Registers {
        Registers {
            a: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            f: Flags::empty(),
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
        }
    }

    get_set!(a, get_a, set_a, u8);
    get_set!(b, get_b, set_b, u8);
    get_set!(c, get_c, set_c, u8);
    get_set!(d, get_d, set_d, u8);
    get_set!(e, get_e, set_e, u8);
    get_set!(h, get_h, set_h, u8);
    get_set!(l, get_l, set_l, u8);
    get_set!(sp, get_sp, set_sp, u16);
    get_set!(pc, get_pc, set_pc, u16);
    get_set_dual!(b, c, get_bc, set_bc);
    get_set_dual!(d, e, get_de, set_de);
    get_set_dual!(h, l, get_hl, set_hl);

    // Backward-compatible accessors with u8 masks
    pub fn get_f(&self) -> u8 {
        self.f.bits()
    }
    pub fn set_f(&mut self, val: u8) {
        self.f = Flags::from_bits_truncate(val & 0xF0);
    }

    // Preferred typed flag accessors
    pub fn flags(&self) -> Flags {
        self.f
    }
    pub fn set_flags(&mut self, flags: Flags) {
        self.f = flags;
    }
    #[allow(dead_code)]
    pub fn set_flag(&mut self, flag: Flags, on: bool) {
        if on {
            self.f.insert(flag);
        } else {
            self.f.remove(flag);
        }
    }

    pub fn get_af(&self) -> u16 {
        (self.a as u16) << 8 | self.f.bits() as u16
    }
    pub fn set_af(&mut self, val: u16) {
        self.a = (val >> 8) as u8;
        self.f = Flags::from_bits_truncate((val & 0x00F0) as u8);
    }
}
