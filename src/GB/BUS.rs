use crate::GB::{bus_apu::BusAPU, bus_io::BusIO, bus_mem::BusMem, mbc::MBC};

pub struct Bus {
    pub bus_mem: BusMem,
    pub bus_io: BusIO,
    pub bus_apu: BusAPU,
}

impl Bus {
    pub fn new() -> Self {
        Self {
            bus_mem: BusMem::new(),
            bus_io: BusIO::new(),
            bus_apu: BusAPU::new(),
        }
    }

    // --- 對外委派方法 ---
    pub fn read(&self, addr: u16) -> u8 {
        self.bus_mem.read(addr, &self.bus_io, &self.bus_apu)
    }
    pub fn write(&mut self, addr: u16, val: u8) {
        if cfg!(test) {
            eprintln!(
                "[BUS] write addr=0x{:04X} val=0x{:02X} rom_banks={} mbc={:?} rom_bank=0x{:03X}",
                addr, val, self.bus_mem.rom_banks, self.bus_mem.mbc, self.bus_mem.mbc.rom_bank()
            );
        }
        self.bus_mem
            .write(addr, val, &mut self.bus_io, &mut self.bus_apu)
    }
    pub fn step(&mut self, cycles: u64) {
        self.bus_io
            .step(cycles, &mut self.bus_mem, &mut self.bus_apu);
        // Step APU with the same cycles (forward full width, BusAPU will chunk)
        self.bus_apu.step_apu(cycles);
    }
    pub fn is_dma_active(&self) -> bool {
        self.bus_io.is_dma_active()
    }
    pub fn get_ie_raw(&self) -> u8 {
        self.bus_io.get_ie_raw()
    }
    pub fn get_if_raw(&self) -> u8 {
        self.bus_io.get_if_raw()
    }
    pub fn set_if_raw(&mut self, v: u8) {
        self.bus_io.set_if_raw(v)
    }
    pub fn load_rom(&mut self, data: Vec<u8>) {
        if cfg!(test) {
            eprintln!("[BUS] load_rom size={} bytes", data.len());
        }
        self.bus_mem.load_rom(data)
    }
    pub fn attach_synth(
        &mut self,
        synth: std::sync::Arc<std::sync::Mutex<crate::interface::audio::SimpleAPUSynth>>,
    ) {
        self.bus_apu.attach_synth(synth)
    }
    pub fn set_joypad_rows(&mut self, dpad: u8, btns: u8) {
        self.bus_io.set_joypad_rows(dpad, btns)
    }
    pub fn framebuffer(&self) -> &[u8] {
        self.bus_io.framebuffer()
    }
}

#[cfg(test)]
mod tests {
    use super::Bus;

    #[test]
    fn bus_basic_new_and_io() {
        let mut bus = Bus::new();
        assert_eq!(bus.framebuffer().len(), 160 * 144);
        assert!(!bus.is_dma_active());
        // load empty rom should not panic
        bus.load_rom(Vec::new());
        // basic RAM write/read in working area
        bus.write(0xC000, 0x42);
        assert_eq!(bus.read(0xC000), 0x42);
        // IE/IF accessors should be callable
        let _ie = bus.get_ie_raw();
        let _ifv = bus.get_if_raw();
        let _fb = bus.framebuffer();
        assert_eq!(_fb.len(), 160 * 144);
    }

    #[test]
    fn bus_step_and_framebuffer_mutation() {
        let mut bus = Bus::new();
        // stepping a few cycles should not panic
        bus.step(4);
        assert_eq!(bus.framebuffer().len(), 160 * 144);
    }

    #[test]
    fn mbc1_bank_switch_basic() {
        let mut bus = Bus::new();
        // create 4 banks and mark cart as MBC1
        let banks = 4usize;
        let mut rom = vec![0u8; banks * 0x4000];
        // Fill each bank with distinct pattern: bankN -> value N
        for b in 0..banks {
            let base = b * 0x4000;
            for i in 0..0x4000 {
                rom[base + i] = (b as u8).wrapping_add(0x10);
            }
        }
        // header: cart type at 0x0147
        rom[0x0147] = 0x01; // MBC1
        bus.load_rom(rom);
        // bank 0 area should read from bank0 (value 0x10)
        assert_eq!(bus.read(0x0000), 0x10);
        // bank X area initial should be bank1 (value 0x11)
        assert_eq!(bus.read(0x4000), 0x11);
        // switch to bank 2 via MBC1 low5
        bus.write(0x2000, 0x02);
        assert_eq!(bus.read(0x4000), 0x12);
    }

    #[test]
    fn mbc5_bank_switch_basic() {
        let mut bus = Bus::new();
        let banks = 8usize;
        let mut rom = vec![0u8; banks * 0x4000];
        for b in 0..banks {
            let base = b * 0x4000;
            for i in 0..0x4000 {
                rom[base + i] = (b as u8).wrapping_add(0x20);
            }
        }
        rom[0x0147] = 0x1A; // MBC5
        bus.load_rom(rom);
        assert_eq!(bus.read(0x4000), 0x21); // bank1
                                            // set low 8 bits to 0x03 and verify rom_bank updated
        bus.write(0x2000, 0x03);
        assert_eq!(bus.bus_mem.rom_bank & 0xFF, 0x03);
        // set high bit (bit8) to 1 -> update rom_bank high bit
        bus.write(0x3000, 0x01);
        assert_eq!(((bus.bus_mem.rom_bank >> 8) & 0x01), 0x01);
    }

    #[test]
    fn mbc3_ram_enable_and_rw() {
        let mut bus = Bus::new();
        let banks = 2usize;
        let mut rom = vec![0u8; banks * 0x4000];
        for b in 0..banks {
            let base = b * 0x4000;
            for i in 0..0x4000 {
                rom[base + i] = (b as u8).wrapping_add(0x30);
            }
        }
        rom[0x0147] = 0x0F; // MBC3
        rom[0x0149] = 0x03; // RAM size code -> 4 banks of 8KB (we accept allocation)
        bus.load_rom(rom);
        // enable RAM via MBC register and verify flag
        bus.write(0x0000, 0x0A); // RAM enable
        assert!(bus.bus_mem.ram_enable, "MBC3 RAM enable flag not set");
        // write to first external RAM address (if ext_ram allocated)
        if !bus.bus_mem.ext_ram.is_empty() {
            bus.write(0xA000, 0x55);
            assert_eq!(bus.read(0xA000), 0x55);
        }
    }
}
