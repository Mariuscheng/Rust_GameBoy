#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RegKind {
    Scx,
    Wx,
    Bgp,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct RegEvent {
    pub x: u16, // pixel column 0..159 at which the new value takes effect
    pub kind: RegKind,
    pub val: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MbcType {
    None,
    Mbc1,
    Mbc3,
    Mbc5,
}
