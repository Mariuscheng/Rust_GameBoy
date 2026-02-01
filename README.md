# Game Boy 模擬器 - GPU 加速 PPU

這個專案展示如何將 Game Boy 的 PPU (Picture Processing Unit) 從 CPU 移到 GPU，使用 compute shader 進行完全 GPU 化的圖形渲染。

## 架構

```
CPU (Rust) ──> Uniforms/Buffers (VRAM/OAM) ──> Compute Shader (PPU Pipeline)
                      ↓
               Framebuffer Texture ──> Graphics Pipeline ──> Screen
```

## PPU Pipeline GPU 實現

PPU pipeline 的各個階段都在 compute shader 中實現：

1. **TILE FETCH**: 從 VRAM 讀取圖塊數據
2. **ATTRIBUTE**: 處理圖塊屬性（調色板、翻轉等）
3. **PIXEL FETCH**: 提取像素顏色索引
4. **PALETTE**: 應用背景/精靈調色板
5. **SPRITE PROCESSING**: 處理精靈渲染（優先級、混合）
6. **BLENDING**: 將顏色索引轉換為 RGB 並寫入 framebuffer

## 文件結構

- `src/ppu.rs` - CPU 實現的 PPU（用於比較）
- `src/gpu_ppu.rs` - GPU 加速 PPU 的框架（概念實現）
- `shaders/ppu_compute.glsl` - 完整的 PPU compute shader
- `src/sdl3.rs` - SDL3 渲染循環，整合 Game Boy 模擬器

## Compute Shader 詳解

Compute shader 使用 `local_size_x = 8, local_size_y = 8` 的工作組，每個工作組處理一個 8x8 的圖塊。

### Uniforms
- PPU 寄存器 (lcdc, stat, scy, scx, wy, wx, bgp, obp0, obp1, ly)
- 精靈計數

### Storage Buffers
- VRAM (8KB)
- OAM (160 bytes)
- 精靈列表 (當前掃描線的精靈)

### 輸出
- RGBA8 Framebuffer texture (160x144)

## 運行

```bash
cargo run
```

程序會：
1. 初始化 Game Boy 模擬器
2. 載入測試 ROM (`roms/dmg-acid2.gb`)
3. 運行 SDL3 渲染循環
4. 每幀更新 Game Boy 狀態並渲染

## GPU 實現狀態

目前 SDL3 的 GPU API 在 Rust 綁定中還不完整，因此 `gpu_ppu.rs` 包含概念代碼和 TODO 註釋。

完整的 GPU 實現需要：
1. SDL3 GPU API 的完整 Rust 綁定
2. SPIR-V shader 編譯
3. GPU 資源管理（buffers, textures, pipelines）

## 性能優勢

GPU 加速的 PPU 可以：
- 並行處理所有像素
- 利用 GPU 的高帶寬內存
- 卸載 CPU 的圖形處理工作
- 支持更高分辨率的渲染和後處理效果

## 未來擴展

- 實現完整的 SDL3 GPU 綁定
- 添加 shader 熱重載
- 支持多個後端 (Vulkan, DirectX, Metal)
- 添加後處理效果 (掃描線模擬、像素化等)
- 支持更高分辨率的渲染