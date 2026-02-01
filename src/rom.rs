use std::fs;
use std::io::Read;
use std::path::Path;

#[derive(Debug)]
pub struct CartridgeHeader {
    pub title: String,
    pub cartridge_type: u8,
    pub rom_size: u8,
    pub ram_size: u8,
}

fn parse_rom_header(rom_data: &[u8]) -> Result<CartridgeHeader, &'static str> {
    if rom_data.len() < 0x150 {
        return Err("ROM too small");
    }

    let title_bytes: Vec<u8> = rom_data[0x0134..0x0143]
        .iter()
        .cloned()
        .filter(|&b| b != 0)
        .collect();
    let title = String::from_utf8(title_bytes).map_err(|_| "Invalid title")?;

    Ok(CartridgeHeader {
        title,
        cartridge_type: rom_data[0x0147],
        rom_size: rom_data[0x0148],
        ram_size: rom_data[0x0149],
    })
}

pub fn read_rom_file(path: &str) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    if Path::new(path).exists() {
        let mut file = fs::File::open(path)?;
        let mut rom_data = Vec::new();
        file.read_to_end(&mut rom_data)?;
        return Ok(rom_data);
    }

    Err(format!("ROM file not found: {}", path).into())
}

pub fn load_and_parse_rom(path: &str) -> Result<(Vec<u8>, CartridgeHeader), Box<dyn std::error::Error>> {
    let rom_data = read_rom_file(path)?;
    let header = parse_rom_header(&rom_data).map_err(|e| e.to_string())?;
    Ok((rom_data, header))
}
