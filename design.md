# SDL3 GameBoy 模擬器 — Design

版本: 1.0

設計目標：根據 `requirements.md`，描述系統架構、模組互動、資料結構、時序與測試策略，讓開發能直接依此實作。

1. 高階架構
- 核心模組：`CPU`、`BUS`（含 `bus_mem`、`bus_io`、`bus_apu`）、`PPU`、`APU`。
- 介面模組：`interface::sdl3_display`（顯示、事件）、`interface::audio`（SDL3 audio synth）、`interface::input`（鍵位 mapping）。
- 工具性模組：`utils`（logging、hash、快照）、`tests`（自動化 runner）。

2. 模組責任與介面
- CPU
  - 負責 opcode 解碼、執行與週期計數。
  - 與 `Bus` 的介面：`read(addr) -> u8`、`write(addr, val)`、`step(cycles)`。
  - 在每次 memory 存取或某些指令後呼叫 `Bus::step(cycles)` 以推進 PPU/APU/Timer。

- BUS
  - 管理記憶體映射、MBC、IO 註冊表、DMA 與中斷旗標。
  - 提供 `framebuffer()` 檢視給 `sdl3_display`。

- PPU (bus_io 的一部份)
  - 採掃描線時序；每個掃描線在對應週期產生像素資料到 `framebuffer`。
  - 在每次 `step(n)` 根據累積時鐘更新狀態並在一幀完成時設定 VBlank 中斷。

- APU / audio
  - 對外提供寄存器映射，內部以簡單合成器（正弦/方波混合或更簡化）輸出樣本給 SDL3 audio callback。

3. 資料結構要點
- Framebuffer: 160x144 的 u8 值（0..3），SdlDisplay 做整數縮放輸出。
- Memory map: 如 GB 規範，分區管理 ROM/RAM/I/O/HRAM/VRAM，MBC handler 管理銀行切換。
- Timer state: DIV、TIMA、TMA、TAC 與內部時鐘累積器，必須精確 increment 在 `step(cycles)`。

4. 時序設計與一致性保證
- 所有週期計數統一以 CPU cycles (T-cycles) 為基礎；`Bus::step(cycles)` 推進 PPU/Timers/APU。
- OAM DMA：寫入 DMA 啟動時（0xFF46），觸發 160 byte 的讀寫序列，且在 DMA 期間某些 CPU 訪存受限。
- 中斷：實作 IME 延遲（EI 指令後的下一條指令才生效）及中斷請求/服務流程。

5. 測試與驗證策略
- 單元測試：CPU 指令集若有小功能可抽出，撰寫 unit tests（例如 flag 操作、算術、邏輯）。
- 整合測試：使用 blargg ROMs（`cpu_instrs.gb`、`instr_timing.gb`、`dmg-acid2.gb` 等）並比對 stdout。
- 回歸測試：在 CI 中執行 headless 模式（mock display 或在 CI 放置 SDL3）以防止破壞性變更。

6. 建構與部署
- 使用 Cargo，預設 feature `sdl` 啟用，README 記載 Windows 上放置 `SDL3.dll` 的位置。
- 建議 release build 作效能測試：`cargo run --release -- <rom>`。

7. 設計決策記錄（ADR，摘要）
- ADR-01：暫不實作完整 GBC 模式以聚焦 DMG 正確性。
- ADR-02：採用 `Bus::step(cycles)` 中央時序驅動，以確保 PPU/APU/Timer 同步。

8. 交互圖與流程（簡述）
- 啟動：`main.rs`初始化 `Bus`、`CPU`、載入 ROM → 迴圈：CPU 執行指令 → 指令內呼叫 `bus.read/write/step` → `Bus::step` 更新 PPU/APU/Timers → PPU 完成一幀呼叫 display blit → SDL 顯示。

9. 可觀察性與日誌
- 提供不同等級的 log（info/debug/trace），能在 `logs/` 輸出 frame/time 歷史以協助比對。

10. 安全與相容性
- 檔案 I/O 與外部資源存取應有錯誤處理與清晰錯誤訊息（例：找不到 ROM、SDL3 dll）。

---
註：此設計描述為實作指引；具體函式簽章與 module 路徑可依現有程式碼庫（`src/GB/*`）調整。
