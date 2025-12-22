//! PPU VRAM 操作

pub struct VRAM {
    pub data: [u8; 0x2000], // 8KB
}

impl VRAM {
    /// 建立新的 VRAM 實例（不再複製 ROM buffer，VRAM 一律初始化為全 0）
    pub fn new_with_buffer(buffer: Option<&[u8]>) -> Self {
        let mut data = [0u8; 0x2000];
        if let Some(buf) = buffer {
            let len = buf.len().min(0x2000);
            data[..len].copy_from_slice(&buf[..len]);
        }
        VRAM { data }
    }
    /// 寫入 VRAM 指定位址
    pub fn write_byte(&mut self, addr: usize, value: u8) {
        if addr < 0x2000 {
            use crate::core::utils::logger::log_to_file;
            log_to_file(&format!(
                "[VRAM_WRITE] addr={:04X} value={:02X}",
                addr, value
            ));
            self.data[addr] = value;
        } else {
            // ...existing code...
        }
    }
    /// 讀取 VRAM 指定位址
    pub fn read_byte(&self, addr: usize) -> u8 {
        if addr < 0x2000 {
            let value = self.data[addr];

            value
        } else {
            // ...existing code...
            0
        }
    }
    /// 清空 VRAM
    pub fn clear(&mut self) {
        self.data.fill(0);
    }
}

pub fn clear_vram(vram: &mut [u8]) {
    vram.fill(0);
}
