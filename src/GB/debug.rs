use std::collections::VecDeque;
use std::sync::atomic::{AtomicU16, Ordering};
use std::sync::Mutex;

// Global debug helper: stores the last PC value fetched by the CPU.
// This is intended only for debugging and tracing, not for production logic.
pub static LAST_PC: AtomicU16 = AtomicU16::new(0);

// Snapshot of CPU registers for debug purposes (small, copyable)
#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub struct DebugRegs {
    pub pc: u16,
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub last_opcode: u8,
}

// Global last CPU state under a Mutex so the Bus can read quiet snapshots during serial writes
pub static LAST_STATE: Mutex<DebugRegs> = Mutex::new(DebugRegs {
    pc: 0,
    a: 0,
    f: 0,
    b: 0,
    c: 0,
    d: 0,
    e: 0,
    h: 0,
    l: 0,
    sp: 0,
    last_opcode: 0,
});

#[allow(dead_code)]
pub fn store_pc(pc: u16) {
    LAST_PC.store(pc, Ordering::SeqCst);
}

pub fn load_pc() -> u16 {
    LAST_PC.load(Ordering::SeqCst)
}

pub fn store_state(s: DebugRegs) {
    if let Ok(mut sref) = LAST_STATE.lock() {
        *sref = s;
    }
}

pub fn load_state() -> DebugRegs {
    if let Ok(sref) = LAST_STATE.lock() {
        *sref
    } else {
        DebugRegs {
            pc: 0,
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            last_opcode: 0,
        }
    }
}

pub static LAST_OPS: Mutex<VecDeque<(u16, u8)>> = Mutex::new(VecDeque::new());

pub fn store_op(pc: u16, op: u8) {
    if let Ok(mut q) = LAST_OPS.lock() {
        if q.len() == 64 {
            q.pop_front();
        }
        q.push_back((pc, op));
    }
}

pub fn get_last_ops() -> Vec<(u16, u8)> {
    if let Ok(q) = LAST_OPS.lock() {
        return q.iter().cloned().collect();
    }
    Vec::new()
}
