# SDL3 GameBoy 模擬器 — Requirements

版本: 1.0

目標：採用「規範驅動開發（Specification-Driven Development, SDD）」，先定義明確需求與驗收準則，再實作 SDL3 GameBoy 模擬器。

1. 產品願景
- 提供一個準確且可測試的 GameBoy (DMG) 模擬器，以 SDL3 作為顯示與音訊後端，能在 Windows 開發機上執行並通過主要相容性測試（例：blargg ROMs）。

2. 利害關係人
- 開發者：需要可維護、可測試之程式碼與清楚設計。
- 測試者：需要自動化測試與可重現的結果（hash 比對、影像比對）。
- 使用者：能在目標平台上順暢執行遊戲。

3. 範圍與界限
- 支援目標：DMG（原始 GameBoy）遊戲映像（.gb），暫不強求 GBC/GBA 特有功能。
- 顯示：使用 SDL3，提供整數縮放、灰階 4 級色階、可選視窗大小。
- 音訊：使用 SDL3 Audio，提供最小化 APU 合成（可開/關）。
- 周邊：鍵盤映射為 Joypad（可設定），序列列印支援 blargg 測試。
- 不支援：BIOS 模擬（預設 post-BIOS 狀態）、網路或多人共享存檔。

4. 功能性需求（Functional Requirements）
- FR-01 ROM 載入：能載入本地 .gb 檔並啟動模擬器。
- FR-02 CPU 正確性：實作 Z80-like CPU（GameBoy CPU）並通過 `cpu_instrs.gb`、`instr_timing.gb` 等 blargg 測試輸出或行為驗證。
- FR-03 記憶體與 MBC：支援 ROM/RAM、至少 MBC1/3/5 所需之 bank switching 與外部 RAM 映射。
- FR-04 PPU/顯示：照時序產生掃描線並輸出 `framebuffer`（4 色值），用 SDL3 顯示並支援整數縮放。
- FR-05 APU/音訊：實作基本 APU 寄存器映射，透過簡單合成器輸出音訊，可設定音量與靜音。
- FR-06 DMA、中斷與計時器：實作 OAM DMA 行為、DIV/TIMA 計時器與中斷（IME、IF/IE）。
- FR-07 輸入：鍵盤轉 Joypad，支援事件 poll 與寫回 FF00。
- FR-08 測試執行：能以自動化腳本執行 ROM 並以 stdout/快照/影像比對判定結果。

5. 非功能性需求（Non-Functional Requirements）
- NFR-01 可測試性：每個主要模組（CPU、BUS、PPU、APU、interface）應有單元或整合測試。
- NFR-02 可維護性：程式碼應分模組、清楚文件、採用一致命名與介面。
- NFR-03 可重現性：在相同輸入與 flag 下，模擬器行為一致（例如輸出序列、framebuffer hash）。
- NFR-04 效能：在開發機上以 debug 模式能達到足以互動之速度（非真實效能保證），release 下達到接近實際速度。

6. 介面需求
- CLI：接受 `--speed`、`--volume`、ROM path 等參數。
- 配置文件（選配）：簡單 JSON/TOML 用於鍵位與視窗設定。

7. 驗收準則（Acceptance Criteria）
- AC-01: 執行 `cargo run -- cpu_instrs.gb`，模擬器輸出 blargg 測試序列且與預期輸出匹配（或測試通過）。
- AC-02: 在 `dmg-acid2.gb` 或對應 PPU tests 上，畫面輸出正確顯示（無明顯視覺錯誤）。
- AC-03: 音訊播放可切換並在 `dmg_sound.gb` 上產生預期行為（可部分模擬）。

8. 風險與假設
- 假設開發環境可取得 SDL3 runtime（Windows: SDL3.dll）。
- 時序問題為最大風險；需以逐步測試（blargg ROMs）來驗證。

9. 未來擴充（非必須）
- 支援 GBC 色彩、SGB、儲存電池或 RTC（MBC3）。

---
註：此文件為 SDD 的「需求」階段輸出，接下來依據此需求產出設計文件與任務清單。
