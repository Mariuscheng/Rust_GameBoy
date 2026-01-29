use sdl3::keyboard::Scancode;

/// 輸入管理器
pub struct InputManager;

impl InputManager {
    pub fn new() -> Self {
        Self
    }

    /// 處理除錯鍵盤事件（每幀檢查）
    pub fn handle_debug_keys(display: &mut crate::interface::sdl3_display::SdlDisplay) {
        for sc in [
            Scancode::Right,
            Scancode::Left,
            Scancode::Up,
            Scancode::Down,
            Scancode::Z,
            Scancode::X,
            Scancode::RShift,
            Scancode::Return,
        ] {
            let _ = display.take_keydown(sc);
        }
    }
}
