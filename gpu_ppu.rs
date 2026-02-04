// GPU Accelerated PPU - 使用 SDL3 GPU API
// 這個模塊展示如何將 PPU pipeline 移到 GPU

use sdl3_sys::gpu::*;

// GPU 資源結構
pub struct GpuPpuResources {
    device: SDL_GPUDevice,
    vram_buffer: SDL_GPUBuffer,
    oam_buffer: SDL_GPUBuffer,
    sprite_list_buffer: SDL_GPUBuffer,
    framebuffer_texture: SDL_GPUTexture,
    compute_pipeline: SDL_GPUComputePipeline,
    uniform_buffer: SDL_GPUBuffer,
}

impl GpuPpuResources {
    pub fn new(window: *mut sdl3_sys::SDL_Window) -> Result<Self, String> {
        // TODO: 初始化 SDL3 GPU device
        // 這需要 SDL3 GPU API 的完整 Rust 綁定

        // 概念代碼：
        /*
        let device = unsafe { SDL_CreateGPUDevice(
            SDL_GPU_SHADERFORMAT_SPIRV | SDL_GPU_SHADERFORMAT_DXIL | SDL_GPU_SHADERFORMAT_METALLIB,
            true, // debug_mode
            std::ptr::null() // name
        )};

        if device.is_null() {
            return Err("Failed to create GPU device".to_string());
        }

        // 創建 buffers
        let vram_buffer = unsafe { SDL_CreateGPUBuffer(device, &SDL_GPUBufferCreateInfo {
            usage: SDL_GPU_BUFFERUSAGE_COMPUTE_STORAGE_READ,
            size: 8192, // 8KB VRAM
            props: 0,
        })};

        let oam_buffer = unsafe { SDL_CreateGPUBuffer(device, &SDL_GPUBufferCreateInfo {
            usage: SDL_GPU_BUFFERUSAGE_COMPUTE_STORAGE_READ,
            size: 160, // OAM
            props: 0,
        })};

        let sprite_list_buffer = unsafe { SDL_CreateGPUBuffer(device, &SDL_GPUBufferCreateInfo {
            usage: SDL_GPU_BUFFERUSAGE_COMPUTE_STORAGE_READ,
            size: 10 * std::mem::size_of::<SpriteData>() as u32, // Max 10 sprites
            props: 0,
        })};

        let uniform_buffer = unsafe { SDL_CreateGPUBuffer(device, &SDL_GPUBufferCreateInfo {
            usage: SDL_GPU_BUFFERUSAGE_COMPUTE_STORAGE_READ,
            size: std::mem::size_of::<PpuUniforms>() as u32,
            props: 0,
        })};

        // 創建 framebuffer texture
        let framebuffer_texture = unsafe { SDL_CreateGPUTexture(device, &SDL_GPUTextureCreateInfo {
            type_: SDL_GPU_TEXTURETYPE_2D,
            format: SDL_GPU_TEXTUREFORMAT_RGBA8_UNORM,
            usage: SDL_GPU_TEXTUREUSAGE_COMPUTE_STORAGE_WRITE | SDL_GPU_TEXTUREUSAGE_SAMPLER,
            width: 160,
            height: 144,
            layer_count_or_depth: 1,
            num_levels: 1,
            sample_count: SDL_GPU_SAMPLECOUNT_1,
            props: 0,
        })};

        // 載入 compute shader
        let shader_code = std::fs::read("shaders/ppu_compute.spv")
            .map_err(|e| format!("Failed to load shader: {}", e))?;
        let compute_pipeline = unsafe { SDL_CreateGPUComputePipeline(device, &SDL_GPUComputePipelineCreateInfo {
            code: shader_code.as_ptr(),
            code_size: shader_code.len(),
            entrypoint: b"main\0".as_ptr() as *const i8,
            format: SDL_GPU_SHADERFORMAT_SPIRV,
            num_samplers: 0,
            num_storage_textures: 1,
            num_storage_buffers: 4,
            num_uniform_buffers: 1,
            props: 0,
        })};

        Ok(GpuPpuResources {
            device,
            vram_buffer,
            oam_buffer,
            sprite_list_buffer,
            framebuffer_texture,
            compute_pipeline,
            uniform_buffer,
        })
        */

        Err("SDL3 GPU API not fully implemented in Rust bindings yet".to_string())
    }

    pub fn render_frame(&self, ppu: &crate::ppu::Ppu, mmu: &crate::mmu::Mmu) {
        // TODO: GPU 渲染實現
        /*
        // 1. 更新 VRAM buffer
        unsafe {
            SDL_UpdateGPUBuffer(self.device, self.vram_buffer,
                mmu.get_vram_data().as_ptr() as *const std::ffi::c_void,
                mmu.get_vram_data().len(), 0);
        }

        // 2. 更新 OAM buffer
        unsafe {
            SDL_UpdateGPUBuffer(self.device, self.oam_buffer,
                mmu.get_oam_data().as_ptr() as *const std::ffi::c_void,
                mmu.get_oam_data().len(), 0);
        }

        // 3. 更新 sprite list (從 PPU 的 oam_sprites)
        let sprite_data: Vec<SpriteData> = ppu.oam_sprites.iter().map(|s| SpriteData {
            y_pos: s.y_pos,
            x_pos: s.x_pos,
            tile_index: s.tile_index,
            attributes: s.attributes,
        }).collect();

        unsafe {
            SDL_UpdateGPUBuffer(self.device, self.sprite_list_buffer,
                sprite_data.as_ptr() as *const std::ffi::c_void,
                sprite_data.len() * std::mem::size_of::<SpriteData>(), 0);
        }

        // 4. 更新 uniforms
        let uniforms = PpuUniforms {
            lcdc: ppu.lcdc as u32,
            stat: ppu.stat as u32,
            scy: ppu.scy as u32,
            scx: ppu.scx as u32,
            wy: ppu.wy as u32,
            wx: ppu.wx as u32,
            bgp: ppu.bgp as u32,
            obp0: ppu.obp0 as u32,
            obp1: ppu.obp1 as u32,
            ly: ppu.ly as u32,
            sprite_count: ppu.oam_sprites.len() as u32,
        };

        unsafe {
            SDL_UpdateGPUBuffer(self.device, self.uniform_buffer,
                &uniforms as *const PpuUniforms as *const std::ffi::c_void,
                std::mem::size_of::<PpuUniforms>(), 0);
        }

        // 5. 開始 compute pass
        let cmd_buf = unsafe { SDL_AcquireGPUCommandBuffer(self.device) };
        let compute_pass = unsafe { SDL_BeginGPUComputePass(cmd_buf) };

        // 綁定資源
        unsafe {
            SDL_BindGPUComputePipeline(compute_pass, self.compute_pipeline);
            SDL_BindGPUComputeStorageBuffers(compute_pass, 0, [self.vram_buffer, self.oam_buffer, self.sprite_list_buffer, self.uniform_buffer].as_ptr(), 4);
            SDL_BindGPUComputeStorageTextures(compute_pass, 0, [self.framebuffer_texture].as_ptr(), 1);

            // Dispatch compute shader (160/8 = 20, 144/8 = 18 tiles)
            SDL_DispatchGPUCompute(compute_pass, 20, 18, 1);
        }

        unsafe {
            SDL_EndGPUComputePass(compute_pass);
            SDL_SubmitGPUCommandBuffer(cmd_buf);
        }
        */
    }

    pub fn get_framebuffer_texture(&self) -> SDL_GPUTexture {
        self.framebuffer_texture
    }
}

// Shader 數據結構 (匹配 GLSL)
#[repr(C)]
struct PpuUniforms {
    lcdc: u32,
    stat: u32,
    scy: u32,
    scx: u32,
    wy: u32,
    wx: u32,
    bgp: u32,
    obp0: u32,
    obp1: u32,
    ly: u32,
    sprite_count: u32,
}

#[repr(C)]
struct SpriteData {
    y_pos: u32,
    x_pos: u32,
    tile_index: u32,
    attributes: u32,
}
