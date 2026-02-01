# Game Boy 模擬器 (AI 建立)

這是一個使用 **Rust** 和 **SDL3** 開發的 Game Boy 模擬器，完全由 AI 協助建立。

## ⚠️ 目前狀態

- ✅ **Hyper_Lode_Runner.gb** - 已測試，可正常遊玩
- ⚠️ 其他遊戲可能有問題，日後會持續優化

## 功能

- CPU 模擬 (Sharp LR35902)
- PPU 圖形渲染 (背景、視窗、精靈)
- APU 音訊處理
- Joypad 輸入處理
- MBC1 卡帶支援
- 外部 RAM 存檔

## 操作按鍵

| Game Boy | 鍵盤 |
|----------|------|
| 方向鍵 | ↑ ↓ ← → |
| A | Z |
| B | X |
| Start | Enter / Space |
| Select | Right Shift |
| 退出 | Escape |

## 運行

```bash
cargo run --release -- roms/Hyper_Lode_Runner.gb
```

## 文件結構

- `src/cpu.rs` - CPU 模擬
- `src/ppu.rs` - PPU 圖形處理
- `src/apu.rs` - APU 音訊處理
- `src/mmu.rs` - 記憶體管理
- `src/joypad.rs` - 輸入處理
- `src/sdl3.rs` - SDL3 視窗與渲染

## 未來優化

- 修復其他遊戲的相容性問題
- 改善 Joypad 輸入處理
- 通過更多測試 ROM
- 支援更多 MBC 類型