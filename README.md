# Game Boy 模擬器 (AI 建立)

這是一個使用 **Rust** 和 **SDL3** 開發的 Game Boy 模擬器，完全由 AI 協助建立。

## ⚠️ 目前狀態

| 測試 | 結果 |
|------|------|
| Wario Land 3 | ✅ |
| Hyper Lode Runner | ✅ |
| Gremlins 2 | ✅ |
| Dr.Mario | ❌ |
| Tetris | ❌ |

## 測試結果

### cpu_instrs (Blargg's CPU 指令測試)
| 測試 | 結果 |
|------|------|
| 01-special | ✅ Passed |
| 02-interrupts | ✅ Passed |
| 03-op sp,hl | ✅ Passed |
| 04-op r,imm | ✅ Passed |
| 05-op rp | ✅ Passed |
| 06-ld r,r | ✅ Passed |
| 07-jr,jp,call,ret,rst | ✅ Passed |
| 08-misc instrs | ✅ Passed |
| 09-op r,r | ✅ Passed |
| 10-bit ops | ✅ Passed |
| 11-op a,(hl) | ✅ Passed |

**總計: 11/11 通過**

### dmg_sound (Blargg's APU 音訊測試)
| 測試 | 結果 |
|------|------|
| 01-registers | ✅ Passed |
| 02-len ctr | ✅ Passed |
| 03-trigger | ✅ Passed |
| 04-sweep | ✅ Passed |
| 05-sweep details | ✅ Passed |
| 06-overflow on trigger | ✅ Passed |
| 07-len sweep period sync | ✅ Passed |
| 08-len ctr during power | ✅ Passed |
| 09-wave read while on | ✅ Passed |
| 10-wave trigger while on | ❌ Failed |
| 11-regs after power | ✅ Passed |
| 12-wave write while on | ✅ Passed |

**總計: 11/12 通過**

### 其他測試
| 測試 ROM | 結果 |
|----------|------|
| dmg-acid2.gb | ✅ Passed |
| instr_timing.gb | ✅ Passed |
| mem_timing_1.gb | ✅ Passed |

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

## 環境需求

### Windows

1. **安裝 Rust**
   ```bash
   # 從 https://rustup.rs/ 下載並安裝
   rustup default stable
   ```

2. **安裝 vcpkg** (用於管理 SDL3)
   ```bash
   git clone https://github.com/microsoft/vcpkg.git
   cd vcpkg
   .\bootstrap-vcpkg.bat
   ```

3. **安裝 SDL3**
   ```bash
   .\vcpkg install sdl3:x64-windows
   ```

4. **設置環境變數**
   ```bash
   # 設定 VCPKG_ROOT 環境變數指向 vcpkg 安裝目錄
   set VCPKG_ROOT=C:\path\to\vcpkg
   ```

### 替代方案：手動放置 DLL

如果不想使用 vcpkg，可以：
1. 從 [SDL3 Releases](https://github.com/libsdl-org/SDL/releases) 下載預編譯的 SDL3
2. 將 `SDL3.dll` 放到專案根目錄的 `SDL3/` 資料夾
3. 將 `SDL3.lib` 放到同一資料夾
4. 執行時確保 `SDL3.dll` 在執行檔同目錄或系統 PATH 中

## 運行

1. 先建立roms資料夾，並放入 Game Boy ROM 檔案

```bash
cargo run --release -- roms/<your_game>.gb
```
2. 或不想建立資料夾，直接放在專案根目錄也可以：

```bash
cargo run --release -- <your_game>.gb
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





