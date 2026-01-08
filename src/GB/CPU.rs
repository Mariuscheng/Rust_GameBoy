use crate::GB::BUS::Bus;
use crate::GB::instructions;
use crate::GB::registers; // Import of registers submodule // Temporary memory until MMU

pub struct CPU {
    pub registers: registers::Registers,
    pub ime: bool, // Interrupt Master Enable - True if you want to enable and intercept interrupts
    // EI delay mechanism: EI arms enable; after the following instruction completes, IME becomes true.
    pub ime_enable_armed: bool,
    pub ime_enable_pending: bool,
    pub opcode: u8,  // Running Instruction Opcode
    pub cycles: u64, // Total Cycles Count
    pub memory: Bus,
    pub halted: bool,
    pub halt_bug: bool,
    // debug flags
    dbg_unknown_once: bool,
    dbg_hit_rst38_once: bool,
    // cycles already stepped inside the current instruction (e.g., during fetch)
    instr_step_cycles: u64,
}

impl CPU {
    // Function to create a new instance
    pub fn new() -> Self {
        CPU {
            registers: registers::Registers::new(),
            ime: false,
            ime_enable_armed: false,
            ime_enable_pending: false,
            opcode: 0,
            cycles: 0,
            memory: Bus::new(),
            halted: false,
            halt_bug: false,
            dbg_unknown_once: false,
            dbg_hit_rst38_once: false,
            instr_step_cycles: 0,
        }
    }

    // During OAM DMA, CPU may only access HRAM (FF80..FFFE) and IE (FFFF). Others should stall until DMA completes.
    #[inline]
    pub fn dma_addr_allowed(&self, addr: u16) -> bool {
        if !self.memory.is_dma_active() {
            return true;
        }
        (0xFF80..=0xFFFE).contains(&addr) || addr == 0xFFFF
    }

    #[inline]
    pub fn dma_block_if_needed(&mut self, addr: u16) {
        if !self.memory.is_dma_active() {
            return;
        }
        if self.dma_addr_allowed(addr) {
            return;
        }
        while self.memory.is_dma_active() && !self.dma_addr_allowed(addr) {
            self.memory.step(4);
            self.instr_step_cycles = self.instr_step_cycles.wrapping_add(4);
        }
    }

    // DMA-aware memory access wrappers: stall on disallowed addresses until DMA completes
    #[inline]
    pub fn read8(&mut self, addr: u16) -> u8 {
        self.dma_block_if_needed(addr);
        self.memory.read(addr)
    }

    #[inline]
    pub fn write8(&mut self, addr: u16, val: u8) {
        self.dma_block_if_needed(addr);
        self.memory.write(addr, val);
    }

    #[inline]
    pub fn step_mem(&mut self, cycles: u64) {
        if cycles > 0 {
            self.memory.step(cycles);
            self.instr_step_cycles = self.instr_step_cycles.wrapping_add(cycles);
        }
    }

    // Interrupt helper methods (using memory addresses for IE/IF)
    #[inline]
    fn read_ie(&self) -> u8 {
        // Bypass DMA gating for interrupt registers
        self.memory.get_ie_raw()
    }
    #[inline]
    fn read_if(&self) -> u8 {
        // Bypass DMA gating for interrupt registers
        self.memory.get_if_raw()
    }
    #[inline]
    fn write_if(&mut self, v: u8) {
        // Bypass DMA gating for interrupt registers
        self.memory.set_if_raw(v);
    }

    // Returns (bit_index, vector) of highest-priority pending interrupt if any
    fn pending_interrupt(&self) -> Option<(u8, u16)> {
        // Only bits 0..4 are valid interrupt lines; IF upper bits may read as 1 on DMG.
        let pending = (self.read_ie() & self.read_if()) & 0x1F;
        if pending == 0 {
            return None;
        }
        let (bit, vec) = if (pending & 0x01) != 0 {
            (0, 0x0040)
        } else if (pending & 0x02) != 0 {
            (1, 0x0048)
        } else if (pending & 0x04) != 0 {
            (2, 0x0050)
        } else if (pending & 0x08) != 0 {
            (3, 0x0058)
        } else {
            (4, 0x0060)
        };
        Some((bit, vec))
    }

    // Service an interrupt: clear IME, clear IF bit, push PC, jump to vector. Returns cycles taken.
    fn service_interrupt(&mut self, bit: u8, vector: u16) -> u64 {
        self.ime = false;
        // Clear IF bit
        let mut ifv = self.read_if();
        ifv &= !(1u8 << bit);
        self.write_if(ifv);
        // Push PC
        let ret = self.registers.get_pc();
        let sp1 = self.registers.get_sp().wrapping_sub(1);
        self.write8(sp1, (ret >> 8) as u8);
        let sp2 = sp1.wrapping_sub(1);
        self.write8(sp2, (ret & 0x00FF) as u8);
        self.registers.set_sp(sp2);
        self.registers.set_pc(vector);
        20
    }

    // Function to retrieve next instructions addressed by PC
    pub fn fetch_next(&mut self) -> u8 {
        // If OAM DMA is active, CPU cannot access most memory; on real HW, CPU is effectively stalled for ~640 cycles.
        // We emulate this by burning cycles in 4-cycle chunks until DMA completes.
        loop {
            if !self.memory.is_dma_active() {
                break;
            }
            let pc_probe = self.registers.get_pc();
            // During OAM DMA, CPU may execute from HRAM 0xFF80..=0xFFFE and IE 0xFFFF
            if (0xFF80..=0xFFFE).contains(&pc_probe) || pc_probe == 0xFFFF {
                break;
            }
            // Otherwise stall in 4-cycle chunks until DMA completes
            self.memory.step(4);
            self.instr_step_cycles = self.instr_step_cycles.wrapping_add(4);
        }
        let pc = self.registers.get_pc();
        let byte = self.memory.read(pc);
        if self.halt_bug {
            // Do not advance PC on this one fetch; clear bug for subsequent reads
            self.halt_bug = false;
        } else {
            self.registers.set_pc(pc.wrapping_add(1));
        }
        // On real HW, an opcode/operand fetch consumes one machine cycle (4 t-cycles).
        // Step PPU/timers immediately so memory access timing tests (mem_timing) observe correct phases.
        self.memory.step(4);
        // Track cycles stepped early to avoid double-stepping at the end of the instruction.
        self.instr_step_cycles = self.instr_step_cycles.wrapping_add(4);
        byte
    }

    // Read next immediate u8 from memory (little-endian helper counterpart below)
    pub fn read_u8_imm(&mut self) -> u8 {
        self.fetch_next()
    }

    // Read next immediate u16 (little endian: low byte first)
    pub fn read_u16_imm(&mut self) -> u16 {
        let lo = self.fetch_next() as u16;
        let hi = self.fetch_next() as u16;
        (hi << 8) | lo
    }

    // Function to decode an opcode
    pub fn decode(opcode: u8, cb_opcode: bool) -> Option<&'static instructions::Instruction> {
        let entry = if cb_opcode {
            instructions::OPCODES_CB[opcode as usize]
        } else {
            instructions::OPCODES[opcode as usize]
        };

        if entry.is_none() && !cb_opcode {
            // Generic decode: LD r,r' family (0x40..=0x7F) excluding HALT (0x76)
            if (opcode & 0b1100_0000) == 0b0100_0000 && opcode != 0x76 {
                return Some(&instructions::GENERIC_LD_R_R);
            }
            // Generic decode: LD r,n family (00 rrr 110)
            if (opcode & 0b1100_0111) == 0b0000_0110 {
                return Some(&instructions::GENERIC_LD_R_N);
            }
        }

        entry
    }

    // Function to execute the decoded opcode
    pub fn execute_next(&mut self) -> u64 {
        // Reset per-instruction early step counter
        self.instr_step_cycles = 0;
        // Detect PC at 0x0038 (RST 38 loop) once
        if !self.dbg_hit_rst38_once {
            let pc_now = self.registers.get_pc();
            if pc_now == 0x0038 {
                let sp = self.registers.get_sp();
                let _lo = self.memory.read(sp);
                let _hi = self.memory.read(sp.wrapping_add(1));
                // println!(
                //     "[CPU] Hit PC=0038 (RST 38). SP={:04X} TOP=[{:02X} {:02X}] IE={:02X} IF={:02X} IME={}",
                //     sp,
                //     lo,
                //     hi,
                //     self.read_ie(),
                //     self.read_if(),
                //     self.ime
                // );
                self.dbg_hit_rst38_once = true;
            }
        }
        // Interrupts are checked between instructions
        if let Some((bit, vec)) = if self.ime { self.pending_interrupt() } else { None } {
            // If halted, wake before servicing
            self.halted = false;
            let taken = self.service_interrupt(bit, vec);
            self.cycles = self.cycles.wrapping_add(taken as u64);
            // Tick MMU timers & peripherals during interrupt service cycles
            self.memory.step(taken);
            return taken;
        }

        // HALT state handling
        if self.halted {
            // If any enabled interrupt becomes pending, wake up (HALT bug simplified: if pending but IME=0, we just wake without servicing)
            if let Some((bit, vec)) = self.pending_interrupt() {
                self.halted = false;
                if self.ime {
                    let taken = self.service_interrupt(bit, vec);
                    self.cycles = self.cycles.wrapping_add(taken as u64);
                    return taken;
                }
                // IME is 0: wake and continue to execute next instruction
            } else {
                // Remain halted; burn 4 cycles
                self.cycles = self.cycles.wrapping_add(4);
                // Tick timers while halted
                self.memory.step(4);
                return 4;
            }
        }

        // Fetch first byte
        let mut opcode = self.fetch_next();
        let mut cb_prefix = false;

        // Handle CB prefix
        if opcode == 0xCB {
            cb_prefix = true;
            opcode = self.fetch_next();
        }

        // Decode
        if let Some(instr) = CPU::decode(opcode, cb_prefix) {
            // Expose current opcode to executors that decode bitfields generically
            self.opcode = opcode;
            // Execute and accumulate cycles
            let taken = (instr.execute)(instr, self);
            self.cycles = self.cycles.wrapping_add(taken as u64);
            // Tick MMU timers & peripherals for the remaining cycles not already stepped during fetch/operand reads
            let rem = taken.saturating_sub(self.instr_step_cycles as u64);
            if rem > 0 {
                self.memory.step(rem);
            }
            // Apply EI delayed enable at end of each instruction
            if self.ime_enable_pending {
                self.ime = true;
                self.ime_enable_pending = false;
            }
            if self.ime_enable_armed {
                // Move armed -> pending so it applies after the NEXT instruction
                self.ime_enable_pending = true;
                self.ime_enable_armed = false;
            }
            taken
        } else {
            // Unknown opcode: log once, then treat as NOP
            if !self.dbg_unknown_once {
                let _pc_before =
                    self.registers.get_pc().wrapping_sub(1 + if cb_prefix { 1 } else { 0 });
                // println!(
                //     "[CPU] Unknown opcode {:02X}{} at PC={:04X}",
                //     opcode,
                //     if cb_prefix { " (CB prefix)" } else { "" },
                //     pc_before
                // );
                self.dbg_unknown_once = true;
            }
            self.cycles = self.cycles.wrapping_add(4);
            self.memory.step(4);
            4
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ld_a_n_sets_a_and_pc() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0100);
        // Program: LD A,0x42
        cpu.memory.write(0x0100, 0x3E); // LD A,n
        cpu.memory.write(0x0101, 0x42);
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.get_a(), 0x42);
        assert_eq!(cpu.registers.get_pc(), 0x0102);
    }

    #[test]
    fn test_timer_div_increments_every_256_cycles() {
        let mut cpu = CPU::new();
        // Reset DIV
        cpu.memory.write(0xFF04, 0xAA);
        assert_eq!(cpu.memory.read(0xFF04), 0);
        // 255 cycles -> no increment
        cpu.memory.step(255);
        assert_eq!(cpu.memory.read(0xFF04), 0);
        // +1 cycle -> +1
        cpu.memory.step(1);
        assert_eq!(cpu.memory.read(0xFF04), 1);
        // +512 cycles -> +2
        cpu.memory.step(512);
        assert_eq!(cpu.memory.read(0xFF04), 3);
    }

    #[test]
    fn test_timer_tima_basic_period_and_overflow_irq() {
        let mut cpu = CPU::new();
        // Enable timer: TAC = 0b100 | 0b01 (enable, select 262144 Hz -> 16 cycles)
        cpu.memory.write(0xFF07, 0b100 | 0b01);
        cpu.memory.write(0xFF06, 0xAB); // TMA
        cpu.memory.write(0xFF05, 0x00); // TIMA
        // 15 cycles -> no increment yet
        cpu.memory.step(15);
        assert_eq!(cpu.memory.read(0xFF05), 0x00);
        // +1 cycle -> TIMA = 1
        cpu.memory.step(1);
        assert_eq!(cpu.memory.read(0xFF05), 0x01);

        // Overflow path: set TIMA to 0xFF then tick 16 cycles
        cpu.memory.write(0xFF05, 0xFF);
        // Clear IF beforehand
        cpu.memory.write(0xFF0F, 0x00);
        cpu.memory.step(16);
        // TIMA reloads to TMA and IF bit 2 set
        assert_eq!(cpu.memory.read(0xFF05), 0xAB);
        assert_ne!(cpu.memory.read(0xFF0F) & 0x04, 0);
    }

    #[test]
    fn test_interrupt_service_ticks_tima() {
        let mut cpu = CPU::new();
        // Setup timer: enable with 16-cycle period, TIMA=0
        cpu.memory.write(0xFF07, 0b100 | 0b01);
        cpu.memory.write(0xFF05, 0x00);
        // Arm an interrupt (VBlank) and enable IME
        cpu.ime = true;
        cpu.memory.write(0xFFFF, 0x01); // IE
        cpu.memory.write(0xFF0F, 0x01); // IF
        // Execute: should service interrupt (20 cycles) and tick timer
        let taken = cpu.execute_next();
        assert_eq!(taken, 20);
        // 20 cycles -> TIMA increments once (period 16)
        assert_eq!(cpu.memory.read(0xFF05), 0x01);
    }

    #[test]
    fn test_halt_idle_ticks_tima() {
        let mut cpu = CPU::new();
        // Setup timer: enable with 16-cycle period
        cpu.memory.write(0xFF07, 0b100 | 0b01);
        cpu.memory.write(0xFF05, 0x00);
        // Enter halted state directly (no interrupts pending)
        cpu.halted = true;
        // Each execute_next while halted burns 4 cycles and should tick timers
        for _ in 0..4 {
            let c = cpu.execute_next();
            assert_eq!(c, 4);
        }
        // 16 cycles accumulated -> TIMA = 1
        assert_eq!(cpu.memory.read(0xFF05), 0x01);
    }

    #[test]
    fn test_timer_enable_disable() {
        let mut cpu = CPU::new();
        // Enable timer at 16-cycle period
        cpu.memory.write(0xFF07, 0b100 | 0b01);
        cpu.memory.write(0xFF05, 0x00);
        cpu.memory.step(16);
        assert_eq!(cpu.memory.read(0xFF05), 0x01);
        // Disable timer
        cpu.memory.write(0xFF07, 0b000 | 0b01);
        cpu.memory.step(32);
        // No change while disabled
        assert_eq!(cpu.memory.read(0xFF05), 0x01);
        // Re-enable and accumulate another 16 cycles
        cpu.memory.write(0xFF07, 0b100 | 0b01);
        cpu.memory.step(16);
        assert_eq!(cpu.memory.read(0xFF05), 0x02);
    }

    #[test]
    fn test_timer_multiple_overflows() {
        let mut cpu = CPU::new();
        // Enable timer at 16-cycle period
        cpu.memory.write(0xFF07, 0b100 | 0b01);
        cpu.memory.write(0xFF06, 0x10); // TMA
        cpu.memory.write(0xFF05, 0xFE); // TIMA close to overflow
        // Clear IF
        cpu.memory.write(0xFF0F, 0x00);
        // 16 cycles -> TIMA: FE->FF; 16 more -> overflow to TMA and set IF
        cpu.memory.step(16);
        assert_eq!(cpu.memory.read(0xFF05), 0xFF);
        cpu.memory.step(16);
        assert_eq!(cpu.memory.read(0xFF05), 0x10);
        assert_ne!(cpu.memory.read(0xFF0F) & 0x04, 0);
        // Clear IF and overflow again
        cpu.memory.write(0xFF0F, 0x00);
        // From 0x10, need 0xF0 more increments to overflow; that's many cycles. Shortcut: set to 0xFF again
        cpu.memory.write(0xFF05, 0xFF);
        cpu.memory.step(16);
        assert_eq!(cpu.memory.read(0xFF05), 0x10);
        assert_ne!(cpu.memory.read(0xFF0F) & 0x04, 0);
    }

    #[test]
    fn test_dma_oam_copy() {
        let mut cpu = CPU::new();
        // Prepare source at 0x8000
        for i in 0..160u16 {
            cpu.memory.write(0x8000 + i, (i & 0xFF) as u8);
        }
        // Start DMA to OAM
        cpu.memory.write(0xFF46, 0x80);
        // DMA takes 160 bytes * 4 cycles = 640 cycles; advance time to complete it
        cpu.memory.step(640);
        // Verify OAM contents copied
        for i in 0..160u16 {
            assert_eq!(cpu.memory.read(0xFE00 + i), (i & 0xFF) as u8);
        }
        // Read back DMA register value
        assert_eq!(cpu.memory.read(0xFF46), 0x80);
    }

    #[test]
    fn test_ppu_basic_registers_and_ly_step() {
        let mut cpu = CPU::new();
        // Ensure LCD off, LY should not advance
        cpu.memory.write(0xFF40, 0x00);
        let ly0 = cpu.memory.read(0xFF44);
        cpu.memory.step(456 * 2);
        assert_eq!(cpu.memory.read(0xFF44), ly0);

        // Turn LCD on
        cpu.memory.write(0xFF40, 0x80);
        // Step one scanline
        cpu.memory.step(456);
        assert_eq!(cpu.memory.read(0xFF44), ly0.wrapping_add(1));
        // Write to LY resets to 0
        cpu.memory.write(0xFF44, 0x99);
        assert_eq!(cpu.memory.read(0xFF44), 0);

        // STAT coincidence interrupt enabled: when LY==LYC, IF bit 1 should be set
        cpu.memory.write(0xFF41, 0x40); // enable coincidence int
        cpu.memory.write(0xFF45, 2); // LYC=2
        // Advance two lines to hit LY==2
        cpu.memory.step(456 * 2);
        assert_ne!(cpu.memory.read(0xFF0F) & 0x02, 0);

        // VBlank interrupt at LY transition 143->144
        // Fast-forward near the end of frame by stepping to LY=143
        cpu.memory.write(0xFF44, 0xAA); // reset LY to 0
        // advance 143 scanlines
        cpu.memory.step(456 * 143);
        assert_eq!(cpu.memory.read(0xFF44), 143);
        cpu.memory.write(0xFF0F, 0x00);
        // Next scanline should enter VBlank (LY=144) and set VBlank IF bit
        cpu.memory.step(456);
        assert_eq!(cpu.memory.read(0xFF44), 144);
        assert_ne!(cpu.memory.read(0xFF0F) & 0x01, 0);

        // Palette regs are mapped
        cpu.memory.write(0xFF47, 0xE4);
        assert_eq!(cpu.memory.read(0xFF47), 0xE4);
        cpu.memory.write(0xFF48, 0xA0);
        assert_eq!(cpu.memory.read(0xFF48), 0xA0);
        cpu.memory.write(0xFF49, 0x1F);
        assert_eq!(cpu.memory.read(0xFF49), 0x1F);
        cpu.memory.write(0xFF4A, 0x77);
        assert_eq!(cpu.memory.read(0xFF4A), 0x77);
        cpu.memory.write(0xFF4B, 0x33);
        assert_eq!(cpu.memory.read(0xFF4B), 0x33);
    }

    #[test]
    fn test_ppu_render_bg_basic() {
        let mut cpu = CPU::new();
        // Enable LCD and BG
        cpu.memory.write(0xFF40, 0x80 | 0x01 | 0x10); // LCD on, BG on, tiledata unsigned 0x8000
        cpu.memory.write(0xFF42, 0); // SCY
        cpu.memory.write(0xFF43, 0); // SCX
        cpu.memory.write(0xFF47, 0b10_01_00_11); // BGP mapping some shades
        // Put tile 0 at (0,0) in BG map 0x9800
        cpu.memory.write(0x9800, 0);
        // Define tile 0 in VRAM at 0x8000: first row: bits 76543210 -> pattern 10000000 (lo/hi)
        // Let's set entire tile row0 to color index 1 (lo=1,hi=0 at bit7)
        for row in 0..8u16 {
            let addr = 0x8000 + row * 2;
            cpu.memory.write(addr, 0x80); // lo
            cpu.memory.write(addr + 1, 0x00); // hi
        }
        // Step one visible line to HBlank so it renders
        cpu.memory.step(80 + (252 - 1)); // move near end of mode3
        cpu.memory.step(1); // enter HBlank -> render
        // Inspect top-left pixel (x=0,y=0), with SCX=0: should be shade mapped from color=1
        let shade = cpu.memory.get_fb_pixel(0, 0);
        assert_eq!(shade, (cpu.memory.read(0xFF47) >> (1 * 2)) & 0x03);
    }

    #[test]
    fn test_inc_a_flags() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0200);
        // A=0x0F -> INC -> 0x10 sets H, clears N; Z=0
        cpu.registers.set_a(0x0F);
        cpu.memory.write(0x0200, 0x3C); // INC A
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_a(), 0x10);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x00); // N=0
        assert_eq!(f & 0x20, 0x20); // H=1
        // C unchanged (masked, but ensure not set spuriously)
        assert_eq!(f & 0x10, 0x00);
    }

    #[test]
    fn test_add_a_b_flags() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0300);
        cpu.registers.set_a(0x8F);
        cpu.registers.set_b(0x81);
        cpu.memory.write(0x0300, 0x80); // ADD A,B
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_a(), 0x10);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x00); // N=0
        assert_eq!(f & 0x20, 0x20); // H=1 (0xF + 0x1)
        assert_eq!(f & 0x10, 0x10); // C=1 (0x8F + 0x81)
    }

    #[test]
    fn test_cb_rlc_b() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0400);
        cpu.registers.set_b(0x80);
        cpu.memory.write(0x0400, 0xCB);
        cpu.memory.write(0x0401, 0x00); // RLC B
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 8);
        assert_eq!(cpu.registers.get_b(), 0x01);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x10, 0x10); // C=1
    }

    #[test]
    fn test_cb_bit7_hl() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0500);
        cpu.registers.set_hl(0x9000);
        cpu.memory.write(0x9000, 0x7F);
        cpu.memory.write(0x0500, 0xCB);
        cpu.memory.write(0x0501, 0x7E); // BIT 7,(HL)
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 12);
        let f = cpu.registers.get_f();
        // bit 7 is 0 => Z=1; N=0; H=1; C unchanged (we default to 0 here)
        assert_eq!(f & 0x80, 0x80);
        assert_eq!(f & 0x40, 0x00);
        assert_eq!(f & 0x20, 0x20);
    }

    #[test]
    fn test_jp_hl_jumps_to_hl() {
        let mut cpu = CPU::new();
        // Place JP (HL) at 0x0100
        cpu.registers.set_pc(0x0100);
        cpu.registers.set_hl(0x1234);
        cpu.memory.write(0x0100, 0xE9);
        let _ = cpu.execute_next();
        assert_eq!(cpu.registers.get_pc(), 0x1234);
    }

    #[test]
    fn test_rst_pushes_pc_and_jumps_vector() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x2000);
        cpu.registers.set_sp(0xFFFE);
        // Use opcode 0xC7 -> vector 0x00
        cpu.memory.write(0x2000, 0xC7);
        let _ = cpu.execute_next();
        assert_eq!(cpu.registers.get_pc(), 0x0000);
        // Return address 0x2001 should be on stack (low at SP, high at SP+1)
        let sp = cpu.registers.get_sp();
        let lo = cpu.memory.read(sp) as u16;
        let hi = cpu.memory.read(sp + 1) as u16;
        assert_eq!(((hi << 8) | lo), 0x2001);
    }

    #[test]
    fn test_reti_pops_pc_and_sets_ime() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x3000);
        cpu.registers.set_sp(0xFFFC);
        // Prepare stack with address 0x4000
        cpu.memory.write(0xFFFC, 0x00); // low
        cpu.memory.write(0xFFFD, 0x40); // high
        cpu.memory.write(0x3000, 0xD9); // RETI opcode
        cpu.ime = false;
        let _ = cpu.execute_next();
        assert!(cpu.ime);
        assert_eq!(cpu.registers.get_pc(), 0x4000);
        assert_eq!(cpu.registers.get_sp(), 0xFFFE);
    }

    #[test]
    fn test_call_nn_pushes_return_and_jumps() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0100);
        cpu.registers.set_sp(0xFFFE);
        // Emit: CALL 0x3456
        cpu.memory.write(0x0100, 0xCD);
        cpu.memory.write(0x0101, 0x56);
        cpu.memory.write(0x0102, 0x34);
        let _ = cpu.execute_next();
        assert_eq!(cpu.registers.get_pc(), 0x3456);
        let sp = cpu.registers.get_sp();
        let lo = cpu.memory.read(sp) as u16;
        let hi = cpu.memory.read(sp + 1) as u16;
        assert_eq!(((hi << 8) | lo), 0x0103);
    }

    #[test]
    fn test_ei_enables_ime_after_next_instruction() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0100);
        cpu.ime = false;
        // Program: EI; NOP
        cpu.memory.write(0x0100, 0xFB);
        cpu.memory.write(0x0101, 0x00);
        let _ = cpu.execute_next();
        assert_eq!(cpu.ime, false, "IME should not enable immediately after EI");
        let _ = cpu.execute_next();
        assert_eq!(cpu.ime, true, "IME should enable after the next instruction completes");
    }

    #[test]
    fn test_ei_followed_by_di_results_ime_false() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x0200);
        cpu.ime = false;
        // Program: EI; DI
        cpu.memory.write(0x0200, 0xFB);
        cpu.memory.write(0x0201, 0xF3);
        let _ = cpu.execute_next(); // EI (arms)
        assert_eq!(cpu.ime, false);
        let _ = cpu.execute_next(); // DI executes before EI takes effect
        assert_eq!(cpu.ime, false, "DI should keep IME false despite pending EI");
    }

    #[test]
    fn test_inc_hl_updates_memory_and_flags() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1000);
        cpu.registers.set_hl(0x9000);
        cpu.memory.write(0x9000, 0x0F);
        cpu.memory.write(0x1000, 0x34); // INC (HL)
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 12);
        assert_eq!(cpu.memory.read(0x9000), 0x10);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x00); // N=0
        assert_eq!(f & 0x20, 0x20); // H=1
    }

    #[test]
    fn test_dec_c_sets_n_h_and_result() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1100);
        cpu.registers.set_c(0x10);
        cpu.memory.write(0x1100, 0x0D); // DEC C
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_c(), 0x0F);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x40); // N=1
        assert_eq!(f & 0x20, 0x20); // H=1 (borrow from bit 4)
    }

    #[test]
    fn test_inc_b_wraps_to_zero_and_sets_flags() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1200);
        cpu.registers.set_b(0xFF);
        cpu.memory.write(0x1200, 0x04); // INC B
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_b(), 0x00);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x80); // Z=1
        assert_eq!(f & 0x40, 0x00); // N=0
        assert_eq!(f & 0x20, 0x20); // H=1
    }

    #[test]
    fn test_dec_d_sets_half_borrow() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1210);
        cpu.registers.set_d(0x10);
        cpu.memory.write(0x1210, 0x15); // DEC D
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_d(), 0x0F);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x40); // N=1
        assert_eq!(f & 0x20, 0x20); // H=1
    }

    #[test]
    fn test_dec_e_zero_no_half_borrow() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1220);
        cpu.registers.set_e(0x01);
        cpu.memory.write(0x1220, 0x1D); // DEC E
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_e(), 0x00);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x80); // Z=1
        assert_eq!(f & 0x40, 0x40); // N=1
        assert_eq!(f & 0x20, 0x00); // H=0
    }

    #[test]
    fn test_inc_h_no_half_carry() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1230);
        cpu.registers.set_h(0x1E);
        cpu.memory.write(0x1230, 0x24); // INC H
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_h(), 0x1F);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x00); // N=0
        assert_eq!(f & 0x20, 0x00); // H=0
    }

    #[test]
    fn test_inc_l_sets_half_carry() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1240);
        cpu.registers.set_l(0x0F);
        cpu.memory.write(0x1240, 0x2C); // INC L
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_l(), 0x10);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x00); // N=0
        assert_eq!(f & 0x20, 0x20); // H=1
    }

    #[test]
    fn test_inc_a_wraps_to_zero_and_sets_flags() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1250);
        cpu.registers.set_a(0xFF);
        cpu.memory.write(0x1250, 0x3C); // INC A
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 4);
        assert_eq!(cpu.registers.get_a(), 0x00);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x80); // Z=1
        assert_eq!(f & 0x40, 0x00); // N=0
        assert_eq!(f & 0x20, 0x20); // H=1
    }

    #[test]
    fn test_push_pop_rr_and_af() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1300);
        cpu.registers.set_sp(0xFFFE);
        cpu.registers.set_bc(0x1234);
        cpu.registers.set_de(0xABCD);
        cpu.registers.set_hl(0x0F0F);
        cpu.registers.set_a(0x56);
        cpu.registers.set_f(0xF3); // 下 4 bits 會被遮掉（只保留高 4）
        // 程式: PUSH BC; PUSH DE; PUSH HL; PUSH AF; POP AF; POP HL; POP DE; POP BC
        let ops = [0xC5, 0xD5, 0xE5, 0xF5, 0xF1, 0xE1, 0xD1, 0xC1];
        for (i, op) in ops.iter().enumerate() {
            cpu.memory.write(0x1300 + i as u16, *op);
        }
        // 依序執行並驗證最終結果等於原值
        for _ in 0..ops.len() {
            let _ = cpu.execute_next();
        }
        assert_eq!(cpu.registers.get_bc(), 0x1234);
        assert_eq!(cpu.registers.get_de(), 0xABCD);
        assert_eq!(cpu.registers.get_hl(), 0x0F0F);
        // POP AF 時 F 低 4 bits 應為 0
        assert_eq!(cpu.registers.get_a(), 0x56);
        assert_eq!(cpu.registers.get_f() & 0x0F, 0x00);
    }

    #[test]
    fn test_alu_immediate_family() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1400);
        // 程式: ADD A,0x01; ADC A,0x01; SUB 0x02; SBC A,0x00; AND 0xF0; XOR 0xFF; OR 0x01; CP 0x01
        let program = [
            0xC6, 0x01, // A=0x00+1 => 0x01
            0xCE, 0x01, // ADC with C=0 => 0x02
            0xD6, 0x02, // SUB 0x02 => 0x00, Z=1
            0xDE, 0x00, // SBC 0 with C=0 => remains 0x00
            0xE6, 0xF0, // AND 0xF0 => 0x00, Z=1, H=1
            0xEE, 0xFF, // XOR 0xFF => 0xFF, Z=0
            0xF6, 0x01, // OR 0x01 => 0xFF
            0xFE, 0x01, // CP 0x01 => compare 0xFF vs 0x01, Z=0, N=1, C=1
        ];
        for (i, b) in program.iter().enumerate() {
            cpu.memory.write(0x1400 + i as u16, *b);
        }
        // 初始化 A 與 Flags
        cpu.registers.set_a(0x00);
        cpu.registers.set_f(0x00);
        // 執行所有指令
        while cpu.registers.get_pc() < 0x1400 + program.len() as u16 {
            let _ = cpu.execute_next();
        }
        // 驗證最終 A 與部分旗標
        assert_eq!(cpu.registers.get_a(), 0xFF);
        let f = cpu.registers.get_f();
        assert_eq!(f & 0x80, 0x00); // Z=0
        assert_eq!(f & 0x40, 0x40); // N=1 (最後 CP)
        assert_eq!(f & 0x10, 0x00); // C=0 (0xFF !< 0x01)
    }

    #[test]
    fn test_ld_hl_inc_a_and_a_hl_inc() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1500);
        cpu.registers.set_hl(0x9000);
        cpu.registers.set_a(0x77);
        // 程式: LD (HL+),A; LD A,(HL+)
        cpu.memory.write(0x1500, 0x22);
        cpu.memory.write(0x1501, 0x2A);
        // 記憶體 0x9001 提前寫入 0x42 供第二指令讀取
        cpu.memory.write(0x9001, 0x42);
        let c1 = cpu.execute_next();
        assert_eq!(c1, 8);
        assert_eq!(cpu.memory.read(0x9000), 0x77);
        assert_eq!(cpu.registers.get_hl(), 0x9001);
        let c2 = cpu.execute_next();
        assert_eq!(c2, 8);
        assert_eq!(cpu.registers.get_a(), 0x42);
        assert_eq!(cpu.registers.get_hl(), 0x9002);
    }

    #[test]
    fn test_ld_hl_dec_a_and_a_hl_dec() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1600);
        cpu.registers.set_hl(0x9001);
        cpu.registers.set_a(0x33);
        // 程式: LD (HL-),A; LD A,(HL-)
        cpu.memory.write(0x1600, 0x32);
        cpu.memory.write(0x1601, 0x3A);
        // 記憶體 0x9000 提前寫入 0x99 供第二指令讀取
        cpu.memory.write(0x9000, 0x99);
        let c1 = cpu.execute_next();
        assert_eq!(c1, 8);
        assert_eq!(cpu.memory.read(0x9001), 0x33);
        assert_eq!(cpu.registers.get_hl(), 0x9000);
        let c2 = cpu.execute_next();
        assert_eq!(c2, 8);
        assert_eq!(cpu.registers.get_a(), 0x99);
        assert_eq!(cpu.registers.get_hl(), 0x8FFF);
    }

    #[test]
    fn test_add_sp_e8_and_ld_hl_sp_e8() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1700);
        cpu.registers.set_sp(0x00F0);
        // 程式: ADD SP,+0x20 ; LD HL,SP-0x10
        cpu.memory.write(0x1700, 0xE8); // ADD SP,e8
        cpu.memory.write(0x1701, 0x20);
        cpu.memory.write(0x1702, 0xF8); // LD HL,SP+e8
        cpu.memory.write(0x1703, 0xF0); // -16

        let c1 = cpu.execute_next();
        assert_eq!(c1, 16);
        // 0x00F0 + 0x20 = 0x0110, low byte carry/half-carry
        assert_eq!(cpu.registers.get_sp(), 0x0110);
        let f1 = cpu.registers.get_f();
        assert_eq!(f1 & 0x80, 0x00); // Z=0
        assert_eq!(f1 & 0x40, 0x00); // N=0
        assert_eq!(f1 & 0x20, 0x00); // H=0
        assert_ne!(f1 & 0x10, 0x00); // C=1

        let c2 = cpu.execute_next();
        assert_eq!(c2, 12);
        // SP(0x0110) + (-16) = 0x0100
        assert_eq!(cpu.registers.get_hl(), 0x0100);
        let f2 = cpu.registers.get_f();
        // For 0x0110 + (-16) => low-byte add 0x10 + 0xF0 = 0x00 with carry, half-carry false
        assert_eq!(f2 & 0x20, 0x00); // H=0
        assert_ne!(f2 & 0x10, 0x00); // C=1
    }

    #[test]
    fn test_ld_sp_hl() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1800);
        cpu.registers.set_hl(0xABCD);
        cpu.memory.write(0x1800, 0xF9); // LD SP,HL
        let c = cpu.execute_next();
        assert_eq!(c, 8);
        assert_eq!(cpu.registers.get_sp(), 0xABCD);
    }

    #[test]
    fn test_ld_a16_a_and_ld_a_a16() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1900);
        cpu.registers.set_a(0x5A);
        // 程式: LD (0x9100),A ; LD A,(0x9100)
        cpu.memory.write(0x1900, 0xEA); // LD (a16),A
        cpu.memory.write(0x1901, 0x00); // low
        cpu.memory.write(0x1902, 0x91); // high => 0x9100
        cpu.memory.write(0x1903, 0xFA); // LD A,(a16)
        cpu.memory.write(0x1904, 0x00);
        cpu.memory.write(0x1905, 0x91);

        let c1 = cpu.execute_next();
        assert_eq!(c1, 16);
        assert_eq!(cpu.memory.read(0x9100), 0x5A);

        // 預先改變記憶體，確保讀入不同值
        cpu.memory.write(0x9100, 0xA5);
        let c2 = cpu.execute_next();
        assert_eq!(c2, 16);
        assert_eq!(cpu.registers.get_a(), 0xA5);
    }

    #[test]
    fn test_ld_a16_sp() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1A00);
        cpu.registers.set_sp(0xBEEF);
        // LD (0x9200),SP
        cpu.memory.write(0x1A00, 0x08);
        cpu.memory.write(0x1A01, 0x00);
        cpu.memory.write(0x1A02, 0x92);
        let c = cpu.execute_next();
        assert_eq!(c, 20);
        assert_eq!(cpu.memory.read(0x9200), 0xEF); // low byte
        assert_eq!(cpu.memory.read(0x9201), 0xBE); // high byte
    }

    #[test]
    fn test_interrupt_service_sequence() {
        let mut cpu = CPU::new();
        // Enable V-Blank IE and set IF, IME=1
        cpu.memory.write(0xFFFF, 0x01);
        cpu.memory.write(0xFF0F, 0x01);
        cpu.registers.set_pc(0x0100);
        cpu.registers.set_sp(0xFFFE);
        cpu.ime = true;
        // execute_next 應直接服務中斷，push 0x0100 並跳 0x0040
        let cycles = cpu.execute_next();
        assert_eq!(cycles, 20);
        assert_eq!(cpu.registers.get_pc(), 0x0040);
        let sp = cpu.registers.get_sp();
        let lo = cpu.memory.read(sp) as u16;
        let hi = cpu.memory.read(sp + 1) as u16;
        assert_eq!(((hi << 8) | lo), 0x0100);
        // 應該清掉 IF bit0 且 IME=0
        assert_eq!(cpu.memory.read(0xFF0F) & 0x01, 0);
        assert!(!cpu.ime);
    }

    #[test]
    fn test_halt_and_wakeup_on_interrupt() {
        let mut cpu = CPU::new();
        // 放入 HALT 指令
        cpu.registers.set_pc(0x2000);
        cpu.memory.write(0x2000, 0x76);
        // 執行 HALT，CPU 進入 halted 狀態
        let _ = cpu.execute_next();
        assert!(cpu.halted);

        // 模擬產生中斷：設 IE/IF V-Blank，IME=1
        cpu.memory.write(0xFFFF, 0x01);
        cpu.memory.write(0xFF0F, 0x01);
        cpu.ime = true;
        // 下一次 execute_next 應醒來並直接服務中斷
        let cyc = cpu.execute_next();
        assert_eq!(cyc, 20);
        assert_eq!(cpu.registers.get_pc(), 0x0040);
        assert!(!cpu.halted);
    }

    #[test]
    fn test_halt_bug_next_fetch_same_pc() {
        let mut cpu = CPU::new();
        // 佈署：IME=0, IE&IF != 0，執行 HALT 應觸發 halt_bug 而非停住
        cpu.ime = false;
        cpu.memory.write(0xFFFF, 0x01);
        cpu.memory.write(0xFF0F, 0x01);
        // 放入：HALT; NOP; NOP
        cpu.registers.set_pc(0x2100);
        cpu.memory.write(0x2100, 0x76); // HALT
        cpu.memory.write(0x2101, 0x00); // NOP (將會被重複抓取一次)
        cpu.memory.write(0x2102, 0x00); // NOP

        // 執行 HALT -> 觸發 halt_bug
        let c0 = cpu.execute_next();
        assert_eq!(c0, 4);
        assert!(!cpu.halted);
        assert!(cpu.halt_bug);
        // 下一次 execute_next: 先檢查中斷 (IME=0 不服務)，未 halted，fetch 第一個 NOP 但 PC 不前進
        let pc_before = cpu.registers.get_pc();
        let c1 = cpu.execute_next();
        assert_eq!(c1, 4);
        let pc_after = cpu.registers.get_pc();
        assert_eq!(pc_after, pc_before, "HALT bug 下第一個抓取不前進 PC");
        // 再下一個 NOP，這次 PC 正常前進 +1
        let c2 = cpu.execute_next();
        assert_eq!(c2, 4);
        assert_eq!(cpu.registers.get_pc(), 0x2102);
    }

    #[test]
    fn test_halt_bug_with_immediate_instruction() {
        let mut cpu = CPU::new();
        // 設定：IME=0，IE&IF 有 pending，執行 HALT 觸發 halt_bug
        cpu.ime = false;
        cpu.memory.write(0xFFFF, 0x01);
        cpu.memory.write(0xFF0F, 0x01);
        // 放入：HALT; LD HL,0x1234，理論上因 HALT bug，低位元組會被複製為 0x21（opcode）
        cpu.registers.set_pc(0x3000);
        cpu.memory.write(0x3000, 0x76); // HALT
        cpu.memory.write(0x3001, 0x21); // LD HL,nn (opcode)
        cpu.memory.write(0x3002, 0x34); // intended low, 但會被當作 high
        cpu.memory.write(0x3003, 0x12); // intended high，將變成下一指令的 opcode

        // 執行 HALT -> 觸發 halt_bug
        let _ = cpu.execute_next();
        assert!(cpu.halt_bug);
        // 執行 LD HL,nn（受 HALT bug 影響）：
        // 取 opcode=0x21（PC 不前進），再取低位=0x21（PC 由 0x3001 前進到 0x3002），高位=0x34
        let cyc = cpu.execute_next();
        assert_eq!(cyc, 12);
        assert_eq!(cpu.registers.get_hl(), 0x3421);
        assert_eq!(cpu.registers.get_pc(), 0x3003);
        // 下一個 opcode 應是原先的 0x12
        assert_eq!(cpu.memory.read(cpu.registers.get_pc()), 0x12);
    }

    #[test]
    fn test_ldh_high_page_and_c_indexed() {
        let mut cpu = CPU::new();
        cpu.registers.set_pc(0x1B00);
        // 程式: LDH (0x80),A ; LDH A,(0x80) ; LD (C),A ; LD A,(C)
        // 初始化
        cpu.registers.set_a(0x12);
        cpu.memory.write(0x1B00, 0xE0); // LDH (a8),A
        cpu.memory.write(0x1B01, 0x80);
        cpu.memory.write(0x1B02, 0xF0); // LDH A,(a8)
        cpu.memory.write(0x1B03, 0x80);
        cpu.registers.set_c(0x81);
        cpu.memory.write(0x1B04, 0xE2); // LD (C),A  -> (0xFF81)=A
        cpu.memory.write(0x1B05, 0xF2); // LD A,(C)  -> A=(0xFF81)

        // LDH (a8),A
        let c1 = cpu.execute_next();
        assert_eq!(c1, 12);
        assert_eq!(cpu.memory.read(0xFF80), 0x12);
        // LDH A,(a8)
        cpu.memory.write(0xFF80, 0x34);
        let c2 = cpu.execute_next();
        assert_eq!(c2, 12);
        assert_eq!(cpu.registers.get_a(), 0x34);
        // LD (C),A
        cpu.registers.set_a(0x56);
        let c3 = cpu.execute_next();
        assert_eq!(c3, 8);
        assert_eq!(cpu.memory.read(0xFF81), 0x56);
        // LD A,(C)
        cpu.memory.write(0xFF81, 0x9A);
        let c4 = cpu.execute_next();
        assert_eq!(c4, 8);
        assert_eq!(cpu.registers.get_a(), 0x9A);
    }
}
