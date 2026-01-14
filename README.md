# Rust Game Boy 模擬器 (未完成)

一個以 Rust 編寫、搭配 SDL3 前端的 Game Boy 模擬器專案。目標是能夠運行經典 ROM、顯示像素、接收輸入並輸出聲音，同時維持簡潔、可讀的程式碼。

- CPU 與匯流排/MMU（支援 MBC1/3/5）
- PPU（背景/視窗/精靈）與整數縮放輸出
- 手把映射鍵盤
- 極簡 APU（方波通道 CH1/CH2）+ SDL 音訊
- 主控台效能遙測（速度/FPS/週期）

> 重要：請使用您合法取得的 ROM。本專案不提供任何 ROM。

## 快速開始

1) 將您的 ROM 放到 `roms/` 資料夾。預設首先尋找 `roms/rom.gb`。
   另外也會依序嘗試：

   - `pokemon_yellow.gb`
   - `Tetris (Japan) (En).gb`
   - `dmg-acid2.gb`
      - 注意：為了讓圖案測試（如 dmg-acid2）與真機顯示一致，SDL 顯示預設已停用 3x3 pixel smoothing（原本會自動填補孤立像素）。若要在開發時重啟 smoothing，請告訴我我可以加上環境變數或旗標來切換。
2) 建置與執行（Windows，`cmd.exe`）：

```cmd
cargo run
```

3) 指定 ROM 執行（可選速度與音量）：

```cmd
cargo run -- --speed 1.00 --volume 0.35 roms\your_game.gb
```

- `--speed <factor>`：整體節奏倍率；`1.0` 為即時（預設）。可用 `0.98`–`1.02` 微調。
- `--volume <0..2>`：額外軟體總音量；預設 `0.35`，避免削波。

執行期間，主控台會週期性印出摘要，例如：

```
Perf: 1.00x | 59.7 FPS | 4,194,304 cycles/s
```

## 控制

- 方向鍵：Arrow keys
- A：Z
- B：X
- Select：Right Shift
- Start：Enter
- Quit：Escape

## 需求

- Rust（穩定版）與 Cargo
- Windows（MSVC 工具鏈）。其他作業系統可能可行，但不是主要目標。
- SDL3 執行階段（DLL 或系統套件）。此 repo 內含連結用的 `SDL3lib/SDL3.lib`，但您仍需在機器上安裝 SDL3 runtime。

### 安裝 SDL3（Windows）

1) 從官方下載 Windows 的 SDL3 預建二進位（開發用函式庫）：

   - https://github.com/libsdl-org/SDL/releases（尋找最新的 SDL3 `SDL3-*.zip` for Windows）
2) 解壓後找到 `SDL3.dll`（通常在 `SDL3-*/lib/x64/SDL3.dll`）。
3) 將 `SDL3.dll` 放在以下任一位置：

   - 放在已編譯的可執行檔旁（最簡單）。建置後一般位於 `target\debug\gameboy_emulator.exe` —— 把 `SDL3.dll` 複製到 `target\debug\`。
   - 或把包含 `SDL3.dll` 的資料夾加入 `PATH`。

若看到缺少 `SDL3.dll` 的錯誤，代表找不到執行階段 DLL —— 請把它放到 exe 同資料夾即可。

### 安裝 SDL3（Linux/macOS）

- Linux：使用發行版套件（名稱可能不同）：`libsdl3`、`libsdl3-dev` 等。在 Debian/Ubuntu 可能需要較新的軟體庫或 PPA。或從原始碼建置：https://github.com/libsdl-org/SDL
- macOS：可用 Homebrew 安裝 `brew install sdl3`，或自行從原始碼建置。

## 目前可用（概要）

- CPU 指令核心，含基本時序與中斷
- MBC1 / MBC3（含簡易 RTC stub）/ MBC5 銀行切換
- PPU 背景/視窗/精靈、8×16 精靈、簡單優先順序
- Joypad `FF00` 選擇語意（獨立的十字鍵/按鈕列）
- 極簡 APU：CH1/CH2 方波，尊重 NR50/NR51/NR52 基礎
- 即時步調至 ~59.73 FPS，並以週期節流

## 已知限制

- APU 刻意簡化：尚無 CH3（波形）、CH4（噪聲），也尚未有完整的包絡/長度/掃描/幀序列器
- PPU 時序相較測試 ROM 有所簡化
- 沒有電池存檔序列化，RTC 為 stub
- 沒有 BIOS；模擬器直接設定 post-BIOS 預設值

## 路線圖（Roadmap）

此專案仍在進行中，以下是規劃中的功能與改進，按優先順序分階段：

### 階段 1：核心精確度提升（當前重點）

- **更精準的 PPU 時序**：提升對測試 ROM 的相容性（如 dmg-acid2、Mooneye 測試集）
- **完整 APU 實作**：加入 CH3（波形通道）、CH4（噪聲通道），以及完整的包絡/長度/掃描/幀序列器
- **CPU 時序精準**：確保所有 blargg 測試 ROM 通過（cpu_instrs.gb, instr_timing.gb, mem_timing.gb）
- **中斷與時序修正**：修復 EI/DI 延遲、HALT bug、OAM bug 等

### 階段 2：功能完整性

- **電池存檔序列化**：為具電池備援 RAM 的遊戲加入存檔/讀檔
- **完整 RTC 支援**：完成 MBC3 的即時鐘支援
- **BIOS 支援**：可選擇載入與執行 BIOS，呈現真實開機序列
- **更多 MBC 類型**：支援 MBC2、MBC6、MBC7 等

### 階段 3：平台與工具

- **跨平台改進**：更好的 Linux/macOS 支援，包含自動化的 SDL3 設定
- **效能最佳化**：更進一步的週期精準與效能調校
- **偵錯工具**：內建偵錯器、記憶體檢視器與指令追蹤
- **UI 增強**：更好的縮放選項、全螢幕與 GUI 控制

### 階段 4：音訊與測試

- **音訊強化**：立體聲、更好的混音與額外音效
- **測試整合**：與 Game Boy 測試 ROM 自動化整合

歡迎貢獻與提出建議！

## 任務（Tasks）

以下是持續開發的具體任務，使用 blargg 測試 ROM 作為驗證標準：

### CPU 與指令測試

- [X] 修復 EI 延遲時序（02:04 子測試通過）
- [X] 運行 `cpu_instrs.gb` 確保所有 11 個子測試通過
  - [X] 01: 特殊指令
  - [X] 02: 中斷處理（EI/DI 延遲）
  - [X] 03: 載入/儲存操作
  - [X] 04: 算術/邏輯操作
  - [X] 05: HALT 指令
  - [X] 06: 堆疊操作
  - [X] 07: 條件跳轉
  - [X] 08: 雜項指令
  - [X] 09: 重置指令
  - [X] 10: 載入立即數
  - [X] 11: 載入遞增/遞減

### 時序測試

- [X] 運行 `instr_timing.gb` 確保指令時序精準
- [X] 運行 `mem_timing.gb` 確保記憶體存取時序正確
- [X] 修復任何時序相關的失敗

### 聲音測試

- [X] 在 APU 中實作 CH3（波形通道）（基本結構已存在）
- [X] 在 APU 中實作 CH4（噪聲通道）（基本結構已存在）
- [X] 為 APU 加入完整的包絡、長度、掃描與幀序列器支援
- [X] 修復 APU 寄存器讀寫（NR10-NR52）
- [X] 加入 DAC 啟用邏輯（CH1/2/3/4）
- [X] 修復 WAVE RAM 讀取行為（當 CH3 啟用時返回 0xFF）
- [ ] 運行 `01-registers.gb` 確保寄存器測試通過
- [ ] 運行 `dmg_sound.gb` 確保所有聲音通道正確
- [ ] 實作立體聲音訊輸出

### PPU 與顯示測試

- [ ] 改善 PPU 時序以提高測試 ROM 相容性（例如：dmg-acid2）
- [ ] 運行 `oam_bug.gb` 確保 OAM bug 行為正確
- [ ] 運行 Mooneye PPU 測試集

### 其他測試

- [ ] 運行 `halt_bug.gb` 確保 HALT bug 行為正確

### 功能實作

- [ ] 為具電池備援 RAM 的遊戲加入存/讀檔
- [ ] 完成 MBC3 的 RTC 實作
- [ ] 加入 BIOS 載入與執行
- [ ] 支援更多 MBC 類型（MBC6、MBC7）
- [ ] 增強跨平台支援（Linux/macOS）
- [ ] 加入內建偵錯器與記憶體檢視器
- [ ] 與 Game Boy 測試 ROM 整合自動化測試
- [ ] 加入全螢幕與更好的 UI 控制

## 專案佈局

- `src/` — 主要模擬器實作（CPU/MMU/PPU）、SDL 顯示與音訊後端
- `roms/` — 放置您的 `.gb` 檔案（不包含）
- `docs/` — 額外說明（`PROJECT_OVERVIEW.md`）
- `SDL3lib/` — Windows 連結使用的 SDL3 匯入程式庫

## 故障排除

- 「No ROM found」—— 確認已將 `.gb` 檔案放到 `roms/` 下並命名為 `roms/rom.gb`，或在 `--` 後傳入路徑。
- 「Missing SDL3.dll」—— 下載 SDL3，將 `SDL3.dll` 複製到可執行檔旁或設定到 PATH 中（參見安裝 SDL3 章節）。
- 白/空白畫面—— 檢查 ROM 是否有寫入 LCDC 與 VRAM；查看主控台日誌中是否有 LCDC 首次寫入訊息。可先嘗試已知良好的 `dmg-acid2.gb`。
- 音量過大/過小—— 調整 `--volume`。若節奏感覺不對，可微調 `--speed`。

## 精確度註記

這是一個 Game Boy 模擬器專案。相較於完全的硬體精確度，更優先追求易上手與良好的互動體驗。未來可能逐步加入更精細的 APU/PPU 細節。

## 參考

- Pan Docs（規格）：https://gbdev.io/pandocs/Specifications.html
- DMG-ACID2 : https://github.com/mattcurrie/dmg-acid2?tab=readme-ov-file

## 授權

TBD。如您計畫重用部分內容，歡迎開 issue 或留言告知。
# Rust_GameBoy
