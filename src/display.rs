use rppal::spi::Spi;
use std::{thread, time::Duration};

const NUM_PAGES: u8 = 8;

const PAGE_ADDRESS_START: u8 = 0xB0;
const DISPLAY_OFF: u8 = 0xAE;
const DISPLAY_ON: u8 = 0xAF;
const VERT_START_MASK: u8 = 0x3F;

const SSD1309_WIDTH: usize = 128;

const SOURCE_WIDTH: usize = 64;
const SOURCE_HEIGHT: usize = 32;

// ==== SSD1309 Normal Commands (DC = 0) ==== From https://www.hpinfotech.ro/SSD1309.pdf at roughly page 27
// 0xA5 => Entire Display on (ignore ram)
// 0xAF => Display ON in normal mode
// 0xA6/A7 => Set Normal/Inverse Display
// 0xAE => Display OFF in sleep mode
// 0xE3 => NOP (no command)

// 0xAE => Display OFF
// 0xAF => Display ON
// 0xD5 => Clock config
// 0xA8 => Set display height
// 0x20 => Set memory mode
// 0xA1 => Flip orientation
// 0xC8 => Flip orientation
// 0xA4 => Use RAM for display
// 0xA6 => Normal display (not inverted)
// 0x81 => Set brightness
// 0x8D => Charge pump (must set this!)

// ==== Scrolling Commands ====
// 0x26 => Right Horizontal Scroll
// 0x00 => Dummy byte (always 0x00)
// 0x00 => Start page address (0 to 7)
// 0x07 => Scroll interval (time between steps)
// 0x07 => End page address (0 to 7)
// 0x00 => Dummy byte (always 0x00)
// 0xFF => Dummy byte (always 0xFF)
// 0x2F => Activate Scroll
// 0x2E => Deactivate scroll

pub struct DisplayInterface {
    spi: Spi,
    dc: rppal::gpio::OutputPin,
    rst: rppal::gpio::OutputPin
}

impl DisplayInterface {
    pub fn new(spi: Spi, dc: rppal::gpio::OutputPin, rst: rppal::gpio::OutputPin) -> DisplayInterface {
        Self { spi, dc, rst }
    }

    pub fn initialize(&mut self) {
        // SSD1309 init sequence
        let init_cmds = [
            DISPLAY_OFF,    // Display OFF
            0xD5, 0x80,     // Clock divide
            0xA8, 0x3F,     // Multiplex: 64
            0xD3, 0x00,     // Display offset
            0x40,           // Start line
            0x8D, 0x14,     // Charge pump ON
            0x20, 0x00,     // Memory mode: horizontal
            0xA1,           // Seg remap
            0xC8,           // COM scan dec
            0xDA, 0x12,     // COM pins
            0x81, 0xCF,     // Contrast
            0xD9, 0xF1,     // Precharge
            0xDB, 0x40,     // VCOM detect
            0xA4,           // Resume from RAM
            0xA6,           // Normal display
            DISPLAY_ON      // Display ON
        ];
        
        // Reset pulse
        self.rst.set_high();
        thread::sleep(Duration::from_millis(10));
        self.rst.set_low();
        thread::sleep(Duration::from_millis(10));
        self.rst.set_high();

        for &cmd in init_cmds.iter() {
            self.send_cmd(cmd);
        }
    }

    pub fn send_cmd(&mut self, cmd: u8) {
        self.dc.set_low(); // Command mode
        self.spi.write(&[cmd]).unwrap();
    }
    
    pub fn send_data(&mut self, data: &[u8]) {
        self.dc.set_high(); // Data mode
        self.spi.write(data).unwrap();
    }
    
    pub fn clear(&mut self) {
        // Fill display with all pixels off
        for page in 0..NUM_PAGES {
            self.send_cmd(PAGE_ADDRESS_START + page);
            self.send_cmd(0x00);
            self.send_cmd(0x10);
            self.send_data(&[0x00; 128]);
        }
    }

    pub fn turn_off(&mut self) {
        self.send_cmd(DISPLAY_OFF);
    }

    pub fn turn_on(&mut self) {
        self.send_cmd(DISPLAY_ON);
    }

    pub fn shift_up(&mut self, shift_amount: usize, delay: u64) {
        for vertical_start in 0..shift_amount {
            let start_point: u8 = (vertical_start % 64) as u8;
            self.send_cmd(0x40 | (start_point & VERT_START_MASK));
            thread::sleep(Duration::from_millis(delay));
        }    
    }

    pub fn fill(&mut self) {
        for page in 0..NUM_PAGES {
            self.send_cmd(PAGE_ADDRESS_START + page);
            self.send_cmd(0x00);
            self.send_cmd(0x10);
            self.send_data(&[0xFF; 128]);
        }
    }

    pub fn display_2d_array(&mut self, array: [[bool; SOURCE_WIDTH]; SOURCE_HEIGHT]) {
        let mut pages: [[u8; SSD1309_WIDTH]; NUM_PAGES as usize] = [[0; SSD1309_WIDTH]; NUM_PAGES as usize];
        for row in 0..SOURCE_HEIGHT {
            for col in 0..SOURCE_WIDTH {
                let value = array[row][col];
                if value {
                    // Scale the coordinates
                    let x0 = col * 2;
                    let y0 = row * 2;
    
                    // Each pixel on 64x32 is 2x2 on a 128x64 screen
                    for dy in 0..2 {
                        let y = y0 + dy;
                        let page = y / 8;
                        let bit = y % 8;
                        for dx in 0..2 {
                            let x = x0 + dx;
                            pages[page][x] |= 1 << bit;
                        }
                    }
                }
            }
        }

        for page in 0..8 {
            self.send_cmd(PAGE_ADDRESS_START + page);
            self.send_cmd(0x00);
            self.send_cmd(0x10);
            self.send_data(&pages[page as usize]);
        }
    }
}
