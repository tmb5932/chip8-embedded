use std::time::{Duration, Instant};
use rppal::{spi::{Spi, Mode, SlaveSelect, Bus}, gpio::{Gpio, Level}};

mod display;
mod chip8;
mod quirks;
mod instruction;
use display::DisplayInterface;
use chip8::Chip8;
use quirks::Quirks;

// Emulator Cycle Return Values
const SUCCESSFUL_EXECUTION: u8 = 0;
const EXIT_ROM: u8 = 1;

// Display Pin constants
const DC_PIN: u8 = 23;
const RST_PIN: u8 = 24;

// Buzzer Pin constant
const BUZZER_PIN: u8 = 25;

// Keypad Pin constants
const ROW_PINS: [u8; 4] = [17, 27, 22, 10];
const COL_PINS: [u8; 4] = [0, 5, 6, 13];

const KEY_MAP: [[u8; 4]; 4] = [
    [0x1, 0x2, 0x3, 0xC],
    [0x4, 0x5, 0x6, 0xD],
    [0x7, 0x8, 0x9, 0xE],
    [0xA, 0x0, 0xB, 0xF],
];

fn load_file_to_memory(chip8: &mut Chip8, path: String, start_location: usize) -> (&mut Chip8, Vec<String>) {
    let mut i: bool = false;
    let mut offset: usize = 0;
    let mut files: Vec<String> = Vec::new();

    // Open file
    let file = File::open(path).unwrap();
    let reader = io::BufReader::new(file);

    for line_result in reader.lines() {
        let line = line_result.unwrap();
        let trimmed = line.trim();
        if i {
            files.push(trimmed.to_owned()); // Add file names to file vector
            i = false;
            continue;
        } else {
            i = true;
        }

        // Add to chip8 memory
        for ch in trimmed.chars() {
            let ascii_value = ch as u8;
            chip8.memory[start_location + offset as usize] = ascii_value;
            offset += 1;
        }
        chip8.memory[start_location + offset as usize] = 0x06; // ACK byte at end of each word
        offset += 1;
    }

    (chip8, files)
}

fn run_game(chip8: &mut Chip8) -> Result<(), Box<dyn std::error::Error>> {
    let timer_interval = Duration::from_millis(16);
    let mut last_timer_tick = Instant::now();

    // SPI setup: SPI0, CE0, 8 MHz, Mode0
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 8_000_000, Mode::Mode0)?;
    
    // rppal GPIO setup
    let gpio = Gpio::new()?;
    let dc = gpio.get(DC_PIN)?.into_output();   // Data/Command pin
    let rst = gpio.get(RST_PIN)?.into_output(); // Reset pin

    let buzzer = gpio.get(BUZZER_PIN)?.into_output(); // Buzzer pin

    // Create SPI interface
    let mut screen = DisplayInterface::new(spi, dc, rst);

    // Initialize the display
    screen.initialize();

    screen.clear();

    // Get all keypad row pins
    let mut rows: Vec<_> = ROW_PINS.iter()
        .map(|&pin| gpio.get(pin).unwrap().into_output_high())
        .collect();

    // Get all keypad col pins
    let cols: Vec<_> = COL_PINS.iter()
        .map(|&pin| gpio.get(pin).unwrap().into_input_pullup())
        .collect();

    'running: loop {
        // Handle keyboard
        for (i, row) in rows.iter_mut().enumerate() {
            row.set_low(); // pull current row low

            for (j, col) in cols.iter().enumerate() {
                let key = KEY_MAP[i][j];
                if col.read() == Level::Low {
                    chip8.keypad[key as usize] = true;
                } else {
                    chip8.keypad[key as usize] = false;
                }
            }

            row.set_high(); // reset row to high
        }

        // Timers
        if last_timer_tick.elapsed() >= timer_interval {
            if chip8.delay_timer > 0 {
                chip8.delay_timer -= 1;
            }
            if chip8.sound_timer > 0 {
                buzzer.set_high();
                chip8.sound_timer -= 1;
            } else {
                buzzer.set_low();
            }
            last_timer_tick = Instant::now();
        }

        // Run Cycle 
        if !chip8.debug || (chip8.debug &&!chip8.paused) {
            let result = chip8.cycle().unwrap();
            
            if result == EXIT_ROM {
                break 'running;
            }
        }

        // Update Display
        if chip8.draw_flag {
            chip8.draw_flag = false;
            screen.display_2d_array(chip8.display);
        }
    };

    screen.clear();
    Ok(chip8.v[1])  // Return Register 1 (for when running my menu ROM)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = "roms/tests/zero-demo.ch8";
    let quirks = Quirks::new(true, false, false, true, true);
    let debug = false;

    let mut chip8 = Chip8::new(quirks);
    chip8.debug = debug;

    chip8.load_rom(&filename.to_string())?;

    run_game(&mut chip8)?;

    Ok(())
}
