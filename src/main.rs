use std::time::{Duration, Instant};
use rppal::{spi::{Spi, Mode, SlaveSelect, Bus}, gpio::{Gpio, Level}};

mod display;
use display::DisplayInterface;

mod chip8;
use chip8::Chip8;

mod quirks;
use quirks::Quirks;

mod instruction;

// Display Pin constants
const DC_PIN: u8 = 23;
const RST_PIN: u8 = 24;

// Keypad Pin constants
const ROW_PINS: [u8; 4] = [17, 27, 22, 10];
const COL_PINS: [u8; 4] = [0, 5, 6, 13];

const KEY_MAP: [[u8; 4]; 4] = [
    [0x1, 0x2, 0x3, 0xC],
    [0x4, 0x5, 0x6, 0xD],
    [0x7, 0x8, 0x9, 0xE],
    [0xA, 0x0, 0xB, 0xF],
];

// Display constants

fn run_game(
    rom: String, 
    quirks: Quirks, 
    debug: bool
) -> Result<(), Box<dyn std::error::Error>> {
    let timer_interval = Duration::from_millis(16);
    let mut last_timer_tick = Instant::now();

    // SPI setup: SPI0, CE0, 8 MHz, Mode0
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 8_000_000, Mode::Mode0)?;
    
    // rppal GPIO setup
    let gpio = Gpio::new()?;
    let dc = gpio.get(DC_PIN)?.into_output();   // Data/Command pin
    let rst = gpio.get(RST_PIN)?.into_output(); // Reset pin

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

    let mut chip8 = Chip8::new(quirks);

    chip8.load_rom(&rom)?;

    chip8.debug = debug;

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
                chip8.sound_timer -= 1;
            }
            last_timer_tick = Instant::now();
        }

        // Run Cycle 
        if !chip8.debug || (chip8.debug &&!chip8.paused) {
            chip8.cycle().unwrap();
        }

        // Update Display
        if chip8.draw_flag {
            chip8.draw_flag = false;
            screen.display_2d_array(chip8.display);
        }
    };

    screen.clear();
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filename = "roms/tests/zero-demo.ch8";
    let load_store = true;
    let clip = true;
    let vf_reset = true;
    let shift = false;
    let jump = false;
    let quirks = Quirks::new(load_store, shift, jump, vf_reset, clip);

    let debug = false;


    run_game(filename.to_string(), quirks, debug)?;

    Ok(())
}
