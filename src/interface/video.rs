//! VideoInterface stub for PPU display

pub trait VideoInterface {
    /// 渲染畫面
    fn render(&mut self) -> Result<(), String>;
    /// 取得畫面緩衝區
    fn get_framebuffer(&self) -> &[u32];
}

/// 測試用虛擬顯示介面
pub struct DummyVideoInterface {
    framebuffer: Vec<u32>,
}

impl DummyVideoInterface {
    pub fn new() -> Self {
        Self {
            framebuffer: vec![0xFFFFFFFF; 160 * 144],
        }
    }
}

impl VideoInterface for DummyVideoInterface {
    fn render(&mut self) -> Result<(), String> {
        // TODO: 實際顯示畫面
        Ok(())
    }
    fn get_framebuffer(&self) -> &[u32] {
        &self.framebuffer
    }
}
