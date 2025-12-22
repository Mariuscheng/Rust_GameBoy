use sdl3::pixels::Color;

/// DMG (Game Boy) palette: index 0~3 對應 4 階灰階
pub fn indices_to_sdl_colors(indices: &[u8]) -> Vec<Color> {
    // Game Boy DMG 預設 4 階灰階
    let palette = [
        Color::RGB(224, 248, 208), // 白
        Color::RGB(136, 192, 112), // 淺灰
        Color::RGB(52, 104, 86),   // 深灰
        Color::RGB(8, 24, 32),     // 黑
    ];
    indices
        .iter()
        .map(|&idx| palette.get(idx as usize).copied().unwrap_or(palette[0]))
        .collect()
}
