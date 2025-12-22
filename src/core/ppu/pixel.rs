//! PPU 像素渲染

pub fn set_pixel(framebuffer: &mut [u8], x: usize, y: usize, color: u8) {
    // TODO: 設定畫面緩衝區的像素顏色
    let idx = y * 160 + x;
    if idx < framebuffer.len() {
        framebuffer[idx] = color;
    }
}
