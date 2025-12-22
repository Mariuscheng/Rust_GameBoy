//! 精靈渲染與屬性解析

#[derive(Debug, Clone, Copy)]
pub struct SpriteFlags(u8);

impl SpriteFlags {
    pub fn new(value: u8) -> Self {
        SpriteFlags(value)
    }
    pub fn priority(&self) -> bool {
        (self.0 & 0x80) != 0
    }
    pub fn y_flip(&self) -> bool {
        (self.0 & 0x40) != 0
    }
    pub fn x_flip(&self) -> bool {
        (self.0 & 0x20) != 0
    }
    pub fn palette(&self) -> bool {
        (self.0 & 0x10) != 0
    }
}

#[derive(Debug, Clone)]
pub struct Sprite {
    pub y: u8,
    pub x: u8,
    pub tile: u8,
    pub flags: SpriteFlags,
}

impl Sprite {
    pub fn new(y: u8, x: u8, tile: u8, flags: u8) -> Self {
        Sprite {
            y,
            x,
            tile,
            flags: SpriteFlags::new(flags),
        }
    }
}

pub fn render_sprites(ppu: &mut super::ppu::PPU) {
    // TODO: 根據 OAM, LCDC, OBP0, OBP1 等渲染精靈
}
