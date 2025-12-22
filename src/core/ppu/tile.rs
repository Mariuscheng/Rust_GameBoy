//! PPU Tile 處理

pub fn decode_tile(vram: &[u8], tile_index: usize) -> [u8; 16] {
    // TODO: 解析 VRAM 中的 tile 資料
    let mut tile = [0u8; 16];
    tile.copy_from_slice(&vram[tile_index*16..tile_index*16+16]);
    tile
}
