# SDL3 GameBoy 模擬器 — Implementation Tasks

# SDL3 GameBoy 模擬器 — Implementation Tasks

此清單依 `requirements.md` 與 `design.md` 分解為可執行任務，含優先度、依賴與估時（小時）。

高層次階段：
- A. 基礎架構與核心（CPU/BUS/Memory）
- B. PPU/顯示整合（framebuffer -> SDL3）
- C. APU/音訊（簡易合成）
- D. 測試與自動化（blargg runner、影像比對）
- E. 優化與整合（CI、文件）

任務清單（已同步當前工作狀態）
1. 建立專案規格檔：已完成 — 0.5h — 優先度：高
2. 核心：確認 `Bus` 接口並寫入測試存根：已完成 — 4h — 依賴：1 — 優先度：高
3. CPU：實作或完善 opcode loop（呼叫 `Bus::read/write/step`）：已完成 — 24h — 依賴：2 — 優先度：高
4. Memory/MBC：ROM 載入與 MBC1/3/5 實作：已完成 — 12h — 依賴：2 — 優先度：高
5. Timer/Interrupt：已完成 — 8h — 依賴：2,3 — 優先度：高
6. PPU：掃描線時序與 framebuffer 寫入：已完成（基礎 PPU 與 mode/STAT 修正） — 20h — 依賴：3,5 — 優先度：高
7. Display：SDL3 整合與 `SdlDisplay::blit_framebuffer()`：進行中（dmg-acid2 不完整） — 6h — 依賴：6 — 優先度：高
8. DMA：OAM DMA 與 CPU 訪存限制：已完成（含多項 DMA 邊界測試） — 6h — 依賴：2,3,6 — 優先度：中
9. APU：最小化合成器（最小可用實作）：已完成 — 16h — 依賴：2,3 — 優先度：中
   - APU 細節完善（CH3/CH4 掃描/頻率調整）：已完成 — 4h — 依賴：9 — 優先度：中
10. Input：鍵盤映射與 Joypad 同步：已完成 — 4h — 依賴：7 — 優先度：高
11. 測試 runner：blargg/mooneye 執行器（收集 stdout、framebuffer hash）：已完成 — 8h — 依賴：1,3,6,7 — 優先度：高
12. CI：新增 headless 測試（mock display 或在 CI 放置 SDL3）：未開始 — 8h — 依賴：11,12 — 優先度：中
13. 文件：補充 README、開發者 notes、SDL3.dll 安裝指引：已完成（初版 SDD 已新增） — 3h — 優先度：中

備註與執行順序
- 初期目標：完成 2–7、10、11（能載入 ROM、執行 CPU、產生 framebuffer 並用 SDL3 顯示、可執行 blargg tests）。
- 先做 完整APU（10% 功能）以驗證音訊管線，之後再完善細節。
- 每完成一個主要功能（例如 CPU 或 PPU），立即加入對應的測試與 CI 工作項，確保回歸安全。

下一步建議
- 優先實作：`13. CI：新增 headless 測試（mock display 或在 CI 放置 SDL3）`
- 其次：MBC3 RTC 支援與更多 APU 包絡/長度/掃描細節

修正與更新
- **Interrupt Timing (2025-12-23)**: 
  - 已參考SameBoy實現`ime_toggle`機制，改善EI delayed enable語義
  - interrupt_time.gb測試部分通過，但仍有timing差異(0D值)
  - 核心邏輯正確：EI後1條指令執行，第2條指令前檢查interrupt
  - 決策：暫緩完善(需cycle-perfect模擬)，當前實現已足夠大多數遊戲運行
  - 後續：待完成其他核心功能後，使用test framework精確對比cycle差異
  
- **下一步**: APU 02-len-ctr.gb出現#7 Failed，修正 APU 長度計數器行為

參考資料
- [GameBoy CPU Manual](https://gbdev.io/pandocs/)
- [Game Boy 的硬體設計與運作原理](https://hackmd.io/@RinHizakura/BJ6HoW29v)
- [SDL3 Documentation](https://wiki.libsdl.org/SDL3/FrontPage)
- [Gameboy sound hardware](https://gbdev.gg8.se/wiki/articles/Gameboy_sound_hardware)
