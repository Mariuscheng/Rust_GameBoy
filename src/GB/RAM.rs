pub struct RAM {
    // Private memory attribute
    memory: [u8; 65536],
}

impl RAM {
    // All these function are "associated functions" as they're linked to RAM structure
    // We use this to create a new RAM instance. Note that we can't create direct RAM instance awithout this method as "memory" attribute is not public and only usable here
    pub fn new() -> Self {
        RAM { memory: [0; 65536] }
    }
    
    // Next two functions are instance "methods", associated function that working on instances with "self" parameter
    pub fn read(&self, address: u16) -> u8 {
        self.memory[address as usize]
    }

    pub fn write(&mut self, address: u16, byte: u8) {
        self.memory[address as usize] = byte;
    }
}