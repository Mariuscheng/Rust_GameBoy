mod apu;
mod cpu;
mod gameboy;
mod instructions;
mod joypad;
mod mmu;
mod ppu;
mod rom;
mod sdl3;
mod timer;

fn main() {
    println!("=== 啟動 Game Boy 模擬器 ===");

    // 獲取命令行參數
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        println!("用法: {} <ROM 路徑>", args[0]);
        return;
    }

    let rom_path = args[1].clone();

    // 直接進入 SDL3 主程式
    sdl3::main(rom_path);
}
