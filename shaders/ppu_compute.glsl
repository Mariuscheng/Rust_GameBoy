// PPU Compute Shader - Game Boy Graphics Pipeline
// CPU (Rust) ──> Uniforms/Buffers (VRAM/OAM) ──> Compute Shader (PPU Pipeline)
//                       ↓
//               Framebuffer Texture ──> Graphics Pipeline ──> Screen

#version 450
#extension GL_KHR_vulkan_glsl : enable

// Uniforms - PPU Registers and State
layout(binding = 0) uniform PpuUniforms {
    uint lcdc;        // LCD Control
    uint stat;        // LCD Status
    uint scy;         // Scroll Y
    uint scx;         // Scroll X
    uint wy;          // Window Y
    uint wx;          // Window X
    uint bgp;         // Background Palette
    uint obp0;        // Sprite Palette 0
    uint obp1;        // Sprite Palette 1
    uint ly;          // Current Scanline
    uint sprite_count; // Number of sprites on this scanline
} uniforms;

// Storage Buffers
layout(binding = 1) readonly buffer VramBuffer {
    uint vram[8192]; // 8KB VRAM
};

layout(binding = 2) readonly buffer OamBuffer {
    uint oam[160];   // 160 bytes OAM (40 sprites * 4 bytes)
};

// Framebuffer Texture
layout(rgba8, binding = 3) writeonly uniform image2D framebuffer;

// Sprite data for current scanline (max 10 sprites)
struct SpriteData {
    uint y_pos;
    uint x_pos;
    uint tile_index;
    uint attributes;
};

layout(binding = 4) readonly buffer SpriteListBuffer {
    SpriteData sprites[10];
};

// Helper functions
uint get_bit(uint value, uint bit) {
    return (value >> bit) & 1u;
}

uint get_bits(uint value, uint start, uint count) {
    return (value >> start) & ((1u << count) - 1u);
}

// Read tile data from VRAM
void get_tile_data(uint tile_index, uint tile_line, bool tile_set, out uint low_byte, out uint high_byte) {
    uint base_addr = tile_set ? 0x8000u : 0x9000u;
    uint tile_addr;

    if (tile_set) {
        tile_addr = base_addr + (tile_index * 16u);
    } else {
        // Signed tile index for 0x8800 set
        int signed_index = int(tile_index);
        if (signed_index >= 128) signed_index -= 256;
        tile_addr = base_addr + uint((signed_index + 128) * 16);
    }

    uint line_addr = tile_addr + (tile_line * 2u);
    low_byte = vram[line_addr - 0x8000u];
    high_byte = vram[line_addr - 0x8000u + 1u];
}

// Get palette color
uint get_palette_color(uint palette, uint color_index) {
    uint shift = color_index * 2u;
    return (palette >> shift) & 3u;
}

// Main compute shader - processes one tile (8x8 pixels)
layout(local_size_x = 8, local_size_y = 8) in;
void main() {
    uint tile_x = gl_GlobalInvocationID.x;
    uint tile_y = gl_GlobalInvocationID.y;

    // Convert tile coordinates to pixel coordinates
    uint pixel_x = tile_x * 8u + gl_LocalInvocationID.x;
    uint pixel_y = tile_y * 8u + gl_LocalInvocationID.y;

    if (pixel_x >= 160u || pixel_y >= 144u) return;

    // Skip if LCD disabled
    if ((uniforms.lcdc & 0x80u) == 0u) {
        imageStore(framebuffer, ivec2(pixel_x, pixel_y), vec4(1.0, 1.0, 1.0, 1.0));
        return;
    }

    uint final_color = 0u;

    // 1. TILE FETCH - Get background/window tile
    bool bg_enabled = (uniforms.lcdc & 0x01u) != 0u;
    bool window_enabled = (uniforms.lcdc & 0x20u) != 0u;
    bool in_window = window_enabled && (pixel_y >= uniforms.wy) && (pixel_x + 7u >= uniforms.wx);

    uint bg_pixel_x, bg_pixel_y;
    bool map_select;

    if (in_window) {
        // Window coordinates
        bg_pixel_x = pixel_x + 7u - uniforms.wx;
        bg_pixel_y = pixel_y - uniforms.wy;
        map_select = (uniforms.lcdc & 0x40u) != 0u; // Window map select
    } else {
        // Background coordinates with scrolling
        bg_pixel_x = (pixel_x + uniforms.scx) % 256u;
        bg_pixel_y = (pixel_y + uniforms.scy) % 256u;
        map_select = (uniforms.lcdc & 0x08u) != 0u; // BG map select
    }

    uint tile_index;
    if (in_window) {
        // Read from window map
        uint map_base = map_select ? 0x9C00u : 0x9800u;
        uint tile_x_in_map = bg_pixel_x / 8u;
        uint tile_y_in_map = bg_pixel_y / 8u;
        uint tile_addr = map_base + (tile_y_in_map * 32u) + tile_x_in_map;
        tile_index = vram[tile_addr - 0x8000u];
    } else {
        // Read from BG map
        uint map_base = map_select ? 0x9C00u : 0x9800u;
        uint tile_x_in_map = bg_pixel_x / 8u;
        uint tile_y_in_map = bg_pixel_y / 8u;
        uint tile_addr = map_base + (tile_y_in_map * 32u) + tile_x_in_map;
        tile_index = vram[tile_addr - 0x8000u];
    }

    // 2. ATTRIBUTE - Get tile data
    uint tile_line = bg_pixel_y % 8u;
    bool tile_set = (uniforms.lcdc & 0x10u) != 0u;
    uint low_byte, high_byte;
    get_tile_data(tile_index, tile_line, tile_set, low_byte, high_byte);

    // 3. PIXEL FETCH - Extract pixel color
    uint pixel_bit = bg_pixel_x % 8u;
    uint bit_index = 7u - pixel_bit;
    uint low_bit = get_bit(low_byte, bit_index);
    uint high_bit = get_bit(high_byte, bit_index);
    uint bg_color_index = (high_bit << 1u) | low_bit;

    // 4. PALETTE - Apply background palette
    uint bg_color = bg_enabled ? get_palette_color(uniforms.bgp, bg_color_index) : 0u;

    final_color = bg_color;

    // 5. SPRITE PROCESSING - Check sprites (from right to left, low index priority)
    bool sprites_enabled = (uniforms.lcdc & 0x02u) != 0u;
    if (sprites_enabled) {
        for (int i = int(uniforms.sprite_count) - 1; i >= 0; i--) {
            SpriteData sprite = sprites[i];

            int sprite_x = int(sprite.x_pos) - 8;
            int sprite_y = int(sprite.y_pos) - 16;

            // Check if pixel is within sprite bounds
            if (int(pixel_x) >= sprite_x && int(pixel_x) < sprite_x + 8 &&
                int(pixel_y) >= sprite_y && int(pixel_y) < sprite_y + (get_bit(uniforms.lcdc, 2) ? 16 : 8)) {

                // Calculate relative coordinates
                uint rel_x = uint(int(pixel_x) - sprite_x);
                uint rel_y = uint(int(pixel_y) - sprite_y);

                // Handle flipping
                if (get_bit(sprite.attributes, 6)) { // Vertical flip
                    uint sprite_height = get_bit(uniforms.lcdc, 2) ? 16u : 8u;
                    rel_y = sprite_height - 1u - rel_y;
                }
                if (get_bit(sprite.attributes, 5)) { // Horizontal flip
                    rel_x = 7u - rel_x;
                }

                // Get tile index (8x16 sprites use two tiles)
                uint sprite_tile_index = sprite.tile_index;
                if (get_bit(uniforms.lcdc, 2) && rel_y >= 8u) {
                    sprite_tile_index |= 0x01u;
                } else {
                    sprite_tile_index &= 0xFEu;
                }

                // Get sprite tile data (always from 0x8000 set)
                uint sprite_tile_line = rel_y % 8u;
                get_tile_data(sprite_tile_index, sprite_tile_line, true, low_byte, high_byte);

                // Extract sprite pixel
                bit_index = 7u - rel_x;
                low_bit = get_bit(low_byte, bit_index);
                high_bit = get_bit(high_byte, bit_index);
                uint sprite_color_index = (high_bit << 1u) | low_bit;

                // Skip transparent pixels
                if (sprite_color_index != 0u) {
                    // Apply sprite palette
                    bool use_obp1 = get_bit(sprite.attributes, 4);
                    uint sprite_palette = use_obp1 ? uniforms.obp1 : uniforms.obp0;
                    uint sprite_color = get_palette_color(sprite_palette, sprite_color_index);

                    // Sprite priority: show sprite unless BG pixel is non-zero and sprite is behind BG
                    bool behind_bg = get_bit(sprite.attributes, 7);
                    if (!behind_bg || bg_color == 0u) {
                        final_color = sprite_color;
                    }
                }
            }
        }
    }

    // 6. BLENDING - Convert to RGB (Game Boy palette)
    vec4 rgb_color;
    switch (final_color) {
        case 0u: rgb_color = vec4(1.0, 1.0, 1.0, 1.0); break; // White
        case 1u: rgb_color = vec4(0.667, 0.667, 0.667, 1.0); break; // Light gray
        case 2u: rgb_color = vec4(0.333, 0.333, 0.333, 1.0); break; // Dark gray
        case 3u: rgb_color = vec4(0.0, 0.0, 0.0, 1.0); break; // Black
    }

    // Write to framebuffer texture
    imageStore(framebuffer, ivec2(pixel_x, pixel_y), rgb_color);
}