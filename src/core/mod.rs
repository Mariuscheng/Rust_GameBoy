// 高階 API struct 定義
pub struct MMU {
    // ...屬性可依需求擴充...
}

pub struct CPU {
    // ...屬性可依需求擴充...
}

pub struct PPU {
    // ...屬性可依需求擴充...
}
pub mod apu;
pub mod cpu;
pub mod cycles;
pub mod emulator;
pub mod error;
pub mod mmu;
pub mod ppu;
pub mod utils;

// 這裡僅公開核心模組，結構與方法請分別在各自模組內定義與實作。
#[cfg(feature = "run_tests")]
mod basic_function_test;
