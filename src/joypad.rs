// Joypad (按鍵輸入) - 處理玩家輸入

use crate::gameboy::{InterruptHandler, InterruptType};
use std::time::{Duration, Instant};

/// Game Boy button types for input mapping
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum GameBoyButton {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
}

/// Input event types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum InputEventType {
    ButtonPress,
    ButtonRelease,
}

/// Input event structure
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct InputEvent {
    pub event_type: InputEventType,
    pub button: GameBoyButton,
    pub timestamp: Instant,
    pub sdl_timestamp: u32,
    pub processing_latency: Option<Duration>,
}

pub struct Joypad {
    // 按鍵狀態 (0 代表按下，1 代表放開)
    // 位元: 0=A/右, 1=B/左, 2=Select/上, 3=Start/下
    pub action_keys: u8,
    pub direction_keys: u8,

    // 選取位元 (Bit 4: 方向鍵, Bit 5: 功能鍵)
    pub select: u8,

    // 精確狀態追蹤
    pub key_states: [KeyState; 8], // 8個按鍵的狀態
    pub debounce_filter: DebounceFilter,
    pub interrupt_handler: Option<*mut InterruptHandler>,
}

#[derive(Debug, Clone)]
pub struct KeyState {
    #[allow(dead_code)]
    pub key: JoypadKey,
    pub pressed: bool,
    pub last_change: Instant,
    pub press_duration: Duration,
    pub release_duration: Duration,
    pub bounce_count: u32, // 去抖動計數
}

impl Default for KeyState {
    fn default() -> Self {
        Self {
            key: JoypadKey::A,
            pressed: false,
            last_change: Instant::now(),
            press_duration: Duration::ZERO,
            release_duration: Duration::ZERO,
            bounce_count: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DebounceFilter {
    pub debounce_threshold: Duration, // 去抖動閾值 (通常 5-10ms)
    #[allow(dead_code)]
    pub last_bounce_check: Instant,
    pub bounce_events_filtered: u64,
}

impl Default for DebounceFilter {
    fn default() -> Self {
        Self {
            debounce_threshold: Duration::from_millis(5), // 5ms 去抖動
            last_bounce_check: Instant::now(),
            bounce_events_filtered: 0,
        }
    }
}

impl Joypad {
    pub fn new() -> Self {
        let mut key_states = [
            KeyState {
                key: JoypadKey::A,
                ..Default::default()
            },
            KeyState {
                key: JoypadKey::B,
                ..Default::default()
            },
            KeyState {
                key: JoypadKey::Select,
                ..Default::default()
            },
            KeyState {
                key: JoypadKey::Start,
                ..Default::default()
            },
            KeyState {
                key: JoypadKey::Right,
                ..Default::default()
            },
            KeyState {
                key: JoypadKey::Left,
                ..Default::default()
            },
            KeyState {
                key: JoypadKey::Up,
                ..Default::default()
            },
            KeyState {
                key: JoypadKey::Down,
                ..Default::default()
            },
        ];

        // 初始化所有按鍵為放開狀態
        for state in &mut key_states {
            state.pressed = false;
            state.last_change = Instant::now() - Duration::from_secs(1); // 設置為過去的時間，以便第一次按鍵被接受
        }

        Joypad {
            action_keys: 0x0F,    // 預設為全放開 (1)
            direction_keys: 0x0F, // 預設為全放開 (1)
            select: 0x30,         // 預設為不選取 (11)
            key_states,
            debounce_filter: DebounceFilter::default(),
            interrupt_handler: None,
        }
    }

    /// 設置中斷處理器以進行優化的中斷處理
    pub fn set_interrupt_handler(&mut self, handler: *mut InterruptHandler) {
        self.interrupt_handler = Some(handler);
    }

    /// 輔助函數：更新位元狀態
    fn update_key_bit(target: &mut u8, mask: u8, pressed: bool) {
        if pressed {
            *target &= !mask;
        } else {
            *target |= mask;
        }
    }

    pub fn read_register(&self) -> u8 {
        // 高位元(6-7)讀取時通常為 1，位元 4-5 是 select bits
        let upper = 0xC0 | self.select;

        // 低 4 位預設為 1（沒有按鍵按下）
        let mut keys = 0x0F;

        if (self.select & 0x10) == 0 {
            // 已選取方向鍵 (Bit 4 = 0)
            keys &= self.direction_keys;
        }

        if (self.select & 0x20) == 0 {
            // 已選取功能鍵 (Bit 5 = 0)
            keys &= self.action_keys;
        }

        upper | keys
    }

    pub fn write_register(&mut self, value: u8) {
        // 只允許寫入位元 4 和 5
        self.select = value & 0x30;
    }

    // 更新按鍵狀態 (由外部轉送，如 SDL3)
    // 按下時 bit 設為 0，放開時設為 1，返回是否觸發中斷
    pub fn set_key(&mut self, key: JoypadKey, pressed: bool) -> bool {
        let now = Instant::now();
        let key_index = key.as_index();

        // 應用去抖動過濾
        if !self.should_process_key_change(key_index, pressed, now) {
            return false; // 過濾掉抖動事件
        }

        let old_res = self.read_register();
        let key_state = &mut self.key_states[key_index];

        // 更新狀態追蹤
        let previous_pressed = key_state.pressed;
        let time_since_last_change = now.duration_since(key_state.last_change);

        if pressed != previous_pressed {
            // 狀態改變
            if pressed {
                key_state.press_duration = time_since_last_change;
            } else {
                key_state.release_duration = time_since_last_change;
            }
            key_state.last_change = now;
            key_state.pressed = pressed;
        } else {
            // 相同狀態，可能是抖動
            key_state.bounce_count += 1;
            self.debounce_filter.bounce_events_filtered += 1;
        }

        // 更新舊的位元狀態以保持兼容性
        match key {
            JoypadKey::A => Self::update_key_bit(&mut self.action_keys, 0x01, pressed),
            JoypadKey::B => Self::update_key_bit(&mut self.action_keys, 0x02, pressed),
            JoypadKey::Select => Self::update_key_bit(&mut self.action_keys, 0x04, pressed),
            JoypadKey::Start => Self::update_key_bit(&mut self.action_keys, 0x08, pressed),
            JoypadKey::Right => Self::update_key_bit(&mut self.direction_keys, 0x01, pressed),
            JoypadKey::Left => Self::update_key_bit(&mut self.direction_keys, 0x02, pressed),
            JoypadKey::Up => Self::update_key_bit(&mut self.direction_keys, 0x04, pressed),
            JoypadKey::Down => Self::update_key_bit(&mut self.direction_keys, 0x08, pressed),
        }

        let new_res = self.read_register();

        // 如果任何位元從 1 變為 0 (Falling Edge) 且有中斷處理器，觸發優化的 Joypad 中斷
        let should_trigger_interrupt = (old_res & !new_res & 0x0F) != 0;
        if should_trigger_interrupt && let Some(handler_ptr) = self.interrupt_handler {
            unsafe {
                (*handler_ptr).trigger_interrupt(InterruptType::Joypad);
            }
        }

        should_trigger_interrupt
    }

    /// 檢查是否應該處理按鍵變化（去抖動過濾）
    fn should_process_key_change(
        &mut self,
        key_index: usize,
        _new_pressed: bool,
        now: Instant,
    ) -> bool {
        let key_state = &self.key_states[key_index];
        let time_since_last_change = now.duration_since(key_state.last_change);

        // 如果時間間隔小於去抖動閾值，過濾掉這個事件
        if time_since_last_change < self.debounce_filter.debounce_threshold {
            self.debounce_filter.bounce_events_filtered += 1;
            return false;
        }

        true
    }

    /// 獲取按鍵狀態統計信息
    #[allow(dead_code)]
    pub fn get_key_stats(&self, key: JoypadKey) -> &KeyState {
        &self.key_states[key.as_index()]
    }

    /// 獲取去抖動統計信息
    #[allow(dead_code)]
    pub fn get_debounce_stats(&self) -> &DebounceFilter {
        &self.debounce_filter
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Copy)]
pub enum JoypadKey {
    A,
    B,
    Select,
    Start,
    Right,
    Left,
    Up,
    Down,
}

impl JoypadKey {
    /// 將 JoypadKey 轉換為數組索引
    pub fn as_index(self) -> usize {
        match self {
            JoypadKey::A => 0,
            JoypadKey::B => 1,
            JoypadKey::Select => 2,
            JoypadKey::Start => 3,
            JoypadKey::Right => 4,
            JoypadKey::Left => 5,
            JoypadKey::Up => 6,
            JoypadKey::Down => 7,
        }
    }

    /// 獲取鍵盤映射
    pub fn get_keyboard_mapping() -> std::collections::HashMap<sdl3::keyboard::Scancode, JoypadKey>
    {
        let mut mapping = std::collections::HashMap::new();
        use sdl3::keyboard::Scancode;

        // 基礎映射 - 適用於所有遊戲
        mapping.insert(Scancode::Up, JoypadKey::Up);
        mapping.insert(Scancode::Down, JoypadKey::Down);
        mapping.insert(Scancode::Left, JoypadKey::Left);
        mapping.insert(Scancode::Right, JoypadKey::Right);
        mapping.insert(Scancode::Return, JoypadKey::Start);
        mapping.insert(Scancode::RShift, JoypadKey::Select);

        // 統一映射 - 包含 Z, X, 和 Space
        mapping.insert(Scancode::Z, JoypadKey::A);
        mapping.insert(Scancode::X, JoypadKey::B);
        mapping.insert(Scancode::Space, JoypadKey::Start); // 額外的 Start 按鍵

        mapping
    }
}
