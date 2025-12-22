use super::cpu::CPU;
use crate::core::cycles::CyclesType;

/// I/O 指令分派
pub fn dispatch(_cpu: &mut CPU, _opcode: u8) -> crate::core::error::Result<CyclesType> {
    // 自動補齊 stub，所有未覆蓋指令預設回傳 4 cycles
    Ok(4)
}

pub fn read_joypad(_cpu: &mut CPU) -> CyclesType {
    // 取得 Joypad 狀態（由 SDL3 backend 注入，需 event_pump）
    // Game Boy Joypad register 0xFF00: 0=按下, 1=未按
    // bit 7-6 unused, bit 5 select button, bit 4 select dpad
    // bit 3-0: Down, Up, Left, Right, Start, Select, B, A
    // Joypad 狀態由主流程 main.rs 控制，這裡留空 stub。
    8
}
pub fn write_joypad(_cpu: &mut CPU) -> CyclesType {
    // TODO: 實作 Joypad 寫入
    8
}
pub fn read_serial(_cpu: &mut CPU) -> CyclesType {
    // TODO: 實作 Serial 讀取
    8
}
pub fn write_serial(_cpu: &mut CPU) -> CyclesType {
    // TODO: 實作 Serial 寫入
    8
}
