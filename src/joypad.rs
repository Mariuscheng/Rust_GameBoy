// Joypad (按鍵輸入) - 處理玩家輸入

pub struct Joypad {
    // 按鍵狀態 (0 代表按下，1 代表放開)
    // 位元: 0=A/右, 1=B/左, 2=Select/上, 3=Start/下
    pub action_keys: u8,
    pub direction_keys: u8,

    // 選取位元 (Bit 4: 方向鍵, Bit 5: 功能鍵)
    pub select: u8,
}

impl Joypad {
    pub fn new() -> Self {
        Joypad {
            action_keys: 0x0F,    // 預設為全放開 (1)
            direction_keys: 0x0F, // 預設為全放開 (1)
            select: 0x30,         // 預設為不選取 (11)
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
        let old_res = self.read_register();

        match key {
            JoypadKey::A => {
                if pressed {
                    self.action_keys &= !0x01
                } else {
                    self.action_keys |= 0x01
                }
            }
            JoypadKey::B => {
                if pressed {
                    self.action_keys &= !0x02
                } else {
                    self.action_keys |= 0x02
                }
            }
            JoypadKey::Select => {
                if pressed {
                    self.action_keys &= !0x04
                } else {
                    self.action_keys |= 0x04
                }
            }
            JoypadKey::Start => {
                if pressed {
                    self.action_keys &= !0x08
                } else {
                    self.action_keys |= 0x08
                }
            }
            JoypadKey::Right => {
                if pressed {
                    self.direction_keys &= !0x01
                } else {
                    self.direction_keys |= 0x01
                }
            }
            JoypadKey::Left => {
                if pressed {
                    self.direction_keys &= !0x02
                } else {
                    self.direction_keys |= 0x02
                }
            }
            JoypadKey::Up => {
                if pressed {
                    self.direction_keys &= !0x04
                } else {
                    self.direction_keys |= 0x04
                }
            }
            JoypadKey::Down => {
                if pressed {
                    self.direction_keys &= !0x08
                } else {
                    self.direction_keys |= 0x08
                }
            }
        }

        let new_res = self.read_register();

        // 如果任何位元從 1 變為 0 (Falling Edge)，觸發 Joypad 中斷 (Bit 4)
        (old_res & !new_res & 0x0F) != 0
    }
}

#[derive(Debug)]
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
