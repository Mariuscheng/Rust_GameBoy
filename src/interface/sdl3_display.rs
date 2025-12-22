use sdl3::event::Event;
use sdl3::keyboard::Scancode;
use sdl3::pixels::PixelFormat;
use sdl3::rect::Rect;
use sdl3::video::Window;
use sdl3::Sdl;
use std::collections::HashSet;

/// SDL3 顯示與輸入後端
pub struct SdlDisplay {
    pub _sdl: Sdl,
    pub _window: Window,
    pub canvas: sdl3::render::Canvas<Window>,
    pub event_pump: sdl3::EventPump,
    pub _scale: u32,
    recent_keydowns: HashSet<Scancode>,
}

impl SdlDisplay {
    pub fn new(title: &str, scale: u32) -> Result<Self, String> {
        let sdl = sdl3::init().map_err(|e| format!("SDL init error: {:?}", e))?;
        let video = sdl
            .video()
            .map_err(|e| format!("SDL video error: {:?}", e))?;
        let width = 160 * scale;
        let height = 144 * scale;
        let window = video
            .window(title, width, height)
            .position_centered()
            .resizable()
            .build()
            .map_err(|e| format!("SDL build window error: {:?}", e))?;
        let canvas = window.into_canvas();
        let event_pump = sdl
            .event_pump()
            .map_err(|e| format!("SDL event pump error: {:?}", e))?;
        Ok(Self {
            _sdl: sdl,
            _window: canvas.window().clone(),
            canvas,
            event_pump,
            _scale: scale,
            recent_keydowns: HashSet::new(),
        })
    }

    /// 將 0..3 的灰階 framebuffer 轉成 ARGB8888 並更新 texture
    pub fn blit_framebuffer(&mut self, shades: &[u8]) -> Result<(), String> {
        if shades.len() != 160 * 144 {
            return Err("framebuffer size mismatch".into());
        }
        // No automatic smoothing by default: keep the framebuffer pixel-perfect
        // for compatibility with graphical test ROMs such as dmg-acid2.
        let fixed: Vec<u8> = shades.to_vec();
        // DMG palette (ARGB) switched to pure grayscale, white background
        let palette: [u32; 4] = [
            0xFFFFFFFF, // 白 (最亮)
            0xFFBFBFBF, // 淺灰
            0xFF7F7F7F, // 深灰
            0xFF000000, // 黑
        ];
        let texture_creator = self.canvas.texture_creator();
        let mut texture = texture_creator
            .create_texture_streaming(PixelFormat::ARGB8888, 160, 144)
            .map_err(|e| format!("SDL texture error: {:?}", e))?;
        texture
            .with_lock(None, |buf: &mut [u8], pitch: usize| {
                for y in 0..144usize {
                    let row = &fixed[y * 160..(y + 1) * 160];
                    let dst = &mut buf[y * pitch..y * pitch + 160 * 4];
                    for (x, &s) in row.iter().enumerate() {
                        let px = palette[s as usize];
                        let o = x * 4;
                        dst[o + 0] = (px & 0xFF) as u8; // B
                        dst[o + 1] = ((px >> 8) & 0xFF) as u8; // G
                        dst[o + 2] = ((px >> 16) & 0xFF) as u8; // R
                        dst[o + 3] = ((px >> 24) & 0xFF) as u8; // A
                    }
                }
            })
            .map_err(|e| format!("lock texture error: {:?}", e))?;
        self.canvas.clear();
        // 整數縮放：依視窗大小計算最接近的整數倍數並置中，避免非整數縮放造成條紋
        let (ww, wh) = self._window.size();
        let sx = (ww / 160).max(1);
        let sy = (wh / 144).max(1);
        let scale = sx.min(sy);
        let dst_w = 160 * scale;
        let dst_h = 144 * scale;
        let dst_x = ((ww - dst_w) / 2) as i32;
        let dst_y = ((wh - dst_h) / 2) as i32;
        let dst = Rect::new(dst_x, dst_y, dst_w, dst_h);
        self.canvas
            .copy(&texture, None, dst)
            .map_err(|e| format!("copy texture error: {:?}", e))?;
        self.canvas.present();
        Ok(())
    }

    /// 處理事件，回傳是否應該結束
    pub fn pump_events_and_update_joypad<F: FnMut(u8, u8)>(&mut self, mut set_p1: F) -> bool {
        // 清理本幀 KeyDown 邊緣
        self.recent_keydowns.clear();
        // GB P1: bit4=方向選擇, bit5=按鍵選擇。被清為 0 表示選中該群。
        // 正確行為：CPU 會寫入 P1 來選群，我們應該回讀時以最近一次寫入的 P1 的 bit4/5 決定哪一群回報。
        // 對應鍵位（方向/按鍵群各自獨立）：
        // 方向：Right, Left, Up, Down -> Arrow keys
        // 按鍵：A, B, Select, Start -> Z, X, RightShift, Enter
        // 只輸出兩組低四位資料（active-low）
        for event in self.event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => return true,
                Event::KeyDown {
                    keycode,
                    scancode,
                    repeat,
                    ..
                } => {
                    if !repeat {
                        if let Some(sc) = scancode {
                            self.recent_keydowns.insert(sc);
                        }
                    }
                    if let Some(sdl3::keyboard::Keycode::Escape) = keycode {
                        return true;
                    }
                }
                _ => {}
            }
        }
        let keyboard = self.event_pump.keyboard_state();
        // 方向
        let right = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::Right);
        let left = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::Left);
        let up = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::Up);
        let down = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::Down);
        // 按鍵
        let a = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::Z);
        let b = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::X);
        let select = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::RShift);
        let start = keyboard.is_scancode_pressed(sdl3::keyboard::Scancode::Return);

        let mut dpad = 0x0Fu8;
        let mut btns = 0x0Fu8;
        if right {
            dpad &= !0x01;
        }
        if left {
            dpad &= !0x02;
        }
        if up {
            dpad &= !0x04;
        }
        if down {
            dpad &= !0x08;
        }
        if a {
            btns &= !0x01;
        }
        if b {
            btns &= !0x02;
        }
        if select {
            btns &= !0x04;
        }
        if start {
            btns &= !0x08;
        }

        set_p1(dpad, btns);
        false
    }

    /// 檢查某個實體鍵是否按下（以 Scancode 判斷）
    #[allow(dead_code)]
    pub fn is_scancode_down(&mut self, sc: Scancode) -> bool {
        let kb = self.event_pump.keyboard_state();
        kb.is_scancode_pressed(sc)
    }

    /// 回傳本幀是否接收到此 scancode 的 KeyDown 事件（單次邊緣觸發）。
    /// 讀取會同時清除此記錄，避免多次觸發。
    pub fn take_keydown(&mut self, sc: Scancode) -> bool {
        if self.recent_keydowns.contains(&sc) {
            self.recent_keydowns.remove(&sc);
            true
        } else {
            false
        }
    }
}
