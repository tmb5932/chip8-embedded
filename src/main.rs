use std::time::{Duration, Instant};
use rppal::{spi::{Spi, Mode, SlaveSelect, Bus}, gpio::{Gpio, Level}};
use std::thread::sleep;

mod display;
mod chip8;
mod quirks;
mod instruction;
use display::DisplayInterface;
use chip8::Chip8;
use quirks::Quirks;

// Emulator Cycle Return Value
const EXIT_ROM: u8 = 1;

// Load point for my custom game-choosing ROM
const MENU_LOAD_LOC: usize = 0x500;

// Display Pin constants
const DC_PIN: u8 = 23;
const RST_PIN: u8 = 24;

// Buzzer Pin constant
const BUZZER_PIN: u8 = 25;

// Pin for push button that ends current ROM
const END_PIN: u8 = 16;

// Keypad Pin constants
const ROW_PINS: [u8; 4] = [4, 27, 0, 5];
const COL_PINS: [u8; 4] = [2, 3, 6, 13];

const KEY_MAP: [[u8; 4]; 4] = [
    [0x1, 0x2, 0x3, 0xC],
    [0x4, 0x5, 0x6, 0xD],
    [0x7, 0x8, 0x9, 0xE],
    [0xA, 0x0, 0xB, 0xF],
];

fn run_game(chip8: &mut Chip8, fps: u64) -> Result<u8, Box<dyn std::error::Error>> {
    let timer_interval = Duration::from_millis(16);
    let mut last_timer_tick = Instant::now();

    // SPI setup: SPI0, CE0, 8 MHz, Mode0
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss0, 8_000_000, Mode::Mode0)?;
    
    // rppal GPIO setup
    let gpio = Gpio::new()?;
    let dc = gpio.get(DC_PIN)?.into_output();   // Data/Command pin
    let rst = gpio.get(RST_PIN)?.into_output(); // Reset pin

    let mut buzzer = gpio.get(BUZZER_PIN)?.into_output();
    buzzer.set_low();

    let rom_button = gpio.get(END_PIN)?.into_input_pullup(); // End current ROM pin

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

    let limit_frames: bool = fps != 0;
    let mut cycle_speed: u64 = 0; 
    if limit_frames {
        cycle_speed = 1_000_000 / fps; // convert fps into how long each frame is, to reach that fps
    }

    let cycle_duration = Duration::from_micros(cycle_speed);    // Controls cycles per second

    'running: loop {
        let loop_start = Instant::now();

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

        if rom_button.is_low() { // Skip to next ROM (or back to menu)
            while rom_button.is_low() {} // Wait for release to avoid skipping next ROM instantly
            break 'running;
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

        let elapsed = loop_start.elapsed();
        if limit_frames && elapsed < cycle_duration {
            sleep(cycle_duration - elapsed);
        }    
    };

    // Turn off buzzer if left on
    buzzer.set_low();

    screen.clear();

    let register_value: u8 = chip8.v[1];
    Ok(register_value)  // Return Register 1 (for when running my menu ROM)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let menu_file = "roms/menu-new.ch8";
    let quirks = Quirks::new(true, false, false, true, true);
    let debug = false;
    let mut chip8 = Chip8::new(quirks);
    chip8.debug = debug;

    let mut menu_item: u8 = 0; // Save where you are in menu between the games

    // Infinitely loop to allow for swapping games without restarting
    loop {
        chip8.load_rom(&menu_file.to_string())?;
        let files: Vec<String> = chip8.load_file_to_memory("data/roms.txt".to_string(), MENU_LOAD_LOC);

        chip8.v[1] = menu_item;
        menu_item = run_game(&mut chip8, 0).unwrap();

        chip8.reset();

        let filename = &files[menu_item as usize];
        let filename = format!("roms/{}", filename);

        chip8.load_rom(&filename)?;
        run_game(&mut chip8, 300).unwrap();

        chip8.reset();
    }
}
