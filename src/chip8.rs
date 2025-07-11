use rand::{RngCore, SeedableRng};
use rand_chacha::ChaCha8Rng;
use crate::instruction::Instruction;
use crate::quirks::Quirks;

// Emulator Cycle Return Values
const SUCCESSFUL_EXECUTION: u8 = 0;
const EXIT_ROM: u8 = 1;

// Chip8 Display Constants
const DISPLAY_WIDTH: usize = 64;
const DISPLAY_HEIGHT: usize = 32;

// Chip8 Memory Constants
const FONTSET_START: usize = 0x50;
const ROM_START: usize = 0x200;

const FONTSET: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80  // F
];

pub struct Chip8 {
    pub memory: [u8; 4096],
    pub v: [u8; 16],
    i: u16,
    pc: u16,
    pub display: [[bool; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
    stack: [u16; 16],
    sp: usize,
    pub delay_timer: u8,
    pub sound_timer: u8,
    pub keypad: [bool; 16],
    pub draw_flag: bool,
    wait_for_release: bool,
    wait_key: usize,
    rng: ChaCha8Rng,

    // Quirks
    quirks: Quirks,

    // Debug
    pub debug: bool,
    pub paused: bool
}

impl Chip8 {
    pub fn new(quirks: Quirks) -> Self {
        let mut chip8 = Chip8 {
            memory: [0; 4096],
            v: [0; 16],
            i: 0,
            pc: 0x200,
            display: [[false; DISPLAY_WIDTH]; DISPLAY_HEIGHT],
            stack: [0; 16],
            sp: 0,
            delay_timer: 0,
            sound_timer: 0,
            keypad: [false; 16],
            draw_flag: false,
            wait_for_release: false,
            wait_key: 0,
            rng: ChaCha8Rng::from_seed(Default::default()),
            quirks: quirks,

            // Debug flags
            debug: false,
            paused: false
        };

        for (i, byte) in FONTSET.iter().enumerate() {
            chip8.memory[FONTSET_START + i] = *byte;
        }

        chip8
    }

    pub fn debug_print(&mut self) {
        println!("PC: 0x{:X}", self.pc);
        let mut line: u8 = 0;
        for register in 0..16 {
            print!("v{:X}: 0x{:X}  \t", register, self.v[register]);
            if line >= 4 {
                print!("\r\n");
                line = 0;
            } else {
                line += 1;
            }
        }

        print!(" I: 0x{:X}\r\n\n", self.i)
    }

    pub fn load_rom(&mut self, filename: &str) -> std::io::Result<()> {
        // Open the file and auto-return if it fails
        let data = std::fs::read(filename)?;

        // Copy each byte into memory starting at 0x200
        for (i, byte) in data.iter().enumerate() {
            self.memory[ROM_START + i] = *byte;
        }

        Ok(())
    }

    pub fn fetch(&mut self) -> Instruction {
        let raw = (self.memory[self.pc as usize] as u16) << 8 |  self.memory[(self.pc + 1) as usize] as u16;
        self.pc += 2;
        Instruction::new(raw)
    }

    pub fn execute(&mut self, inst: Instruction) -> std::io::Result<u8> {
        // Execute
        match inst.nibble {
            0x0 => {
                match inst.nn {
                    0xE0 => {
                        // Clear display
                        self.display = [[false; DISPLAY_WIDTH]; DISPLAY_HEIGHT];
                    },
                    0xEE => {
                        // Return from main (close ROM)
                        if self.sp == 0 {
                            return Ok(EXIT_ROM);
                        }
                        // return from sub function
                        self.sp -= 1;
                        self.pc = self.stack[self.sp];
                    },
                    _ => { println!("Unknown opcode: {:04X}", inst.instruction); }
                }
            }
            0x1 => {
                // Jump: PC = NNN
                self.pc = inst.nnn;
            }
            0x2 => {
                // JAL: STACK[SP] = PC AND PC = NNN
                self.stack[self.sp] = self.pc;
                self.sp += 1;
                self.pc = inst.nnn;
            }
            0x3 => {
                // Skip 1 instruction if VX == NN
                if self.v[inst.x] == inst.nn {
                    self.pc += 2;
                }
            }
            0x4 => {
                // Skip 1 instruction if VX != NN
                if self.v[inst.x] != inst.nn {
                    self.pc += 2;
                }
            }
            0x5 => {
                match inst.n {
                    0x0 => {
                        // Skip 1 instruction if VX == VY
                        if self.v[inst.x] == self.v[inst.y] {
                            self.pc += 2;
                        }
                    }
                    _ => { panic!("Unknown opcode: {:04X}", inst.instruction); }
                }
            }
            0x6 => {
                // SET: VX = NN
                self.v[inst.x] = inst.nn;
            }
            0x7 => {
                // ADD: VX = VX + NN
                let (result, _) = self.v[inst.x].overflowing_add(inst.nn);
                self.v[inst.x] = result;
            }
            0x8 => {
                match inst.n {
                    // BINARY OPS
                    0x0 => {
                        // SET: VX = VY
                        self.v[inst.x] = self.v[inst.y];
                    }
                    0x1 => {
                        // OR: VX = VX OR VY
                        self.v[inst.x] = self.v[inst.x] | self.v[inst.y];
                        if self.quirks.vf_reset {
                            self.v[0xF] = 0; 
                        }
                    }
                    0x2 => {
                        // AND: VX = VX AND VY
                        self.v[inst.x] = self.v[inst.x] & self.v[inst.y];
                        if self.quirks.vf_reset {
                            self.v[0xF] = 0; 
                        }
                    }
                    0x3 => {
                        // XOR: VX = VX XOR VY
                        self.v[inst.x] = self.v[inst.x] ^ self.v[inst.y];
                        if self.quirks.vf_reset {
                            self.v[0xF] = 0; 
                        }
                    }
                    0x4 => {
                        // ADD (with overflow): VX = VX + VY
                        let (sum, carry) = self.v[inst.x].overflowing_add(self.v[inst.y]);
                        self.v[inst.x] = sum;
                        self.v[0xF] = if carry { 1 } else { 0 };
                    }
                    0x5 => {
                        // 8XY5 sets VX to the result of VX - VY.                        
                        let (result, borrowed) = self.v[inst.x].overflowing_sub(self.v[inst.y]);
                        self.v[inst.x] = result;
                        self.v[0xF] = if borrowed { 0 } else { 1 };
                    }
                    0x6 => {
                        // SHIFT QUIRK: 8XY6 VX = VY >> 1
                        // No QUIRK:    8XY6 VX = VX >> 1
                        let shift_src = if self.quirks.shift { inst.x } else { inst.y };
                        let lsb: u8 = self.v[shift_src] & 0x1;
                        self.v[inst.x] = self.v[shift_src] >> 1;
                        self.v[0xF] = lsb;
                    }
                    0x7 => {
                        // 8XY7 sets VX to the result of VY - VX.
                        let (result, borrowed) = self.v[inst.y].overflowing_sub(self.v[inst.x]);
                        self.v[inst.x] = result;
                        self.v[0xF] = if borrowed { 0 } else { 1 };
                    }
                    0xE => {
                        // YSHIFT:    8XYE VX = VY << 1
                        // No YSHIFT: 8XYE VX = VX << 1
                        let shift_src = if self.quirks.shift { inst.x } else { inst.y };
                        let msb: u8 = (self.v[shift_src] >> 7) & 0x1;
                        self.v[inst.x] = self.v[shift_src] << 1;
                        self.v[0xF] = msb;
                    }
                    _ => { panic!("Unknown opcode: {:04X}", inst.instruction); }
                }
            }
            0x9 => {
                match inst.n {
                    0x0 => {
                        // Skip 1 instruction if VX != VY
                        if self.v[inst.x] != self.v[inst.y] {
                            self.pc += 2;
                        }
                    }
                    _ => { panic!("Unknown opcode: {:04X}", inst.instruction) }
                }
            }
            0xA => {
                // I = NNN
                self.i = inst.nnn;
            }
            0xB => {
                // CHIP-8:     0xBNNN Jump to NNN + V0
                // SUPER-CHIP: 0xBXNN Jump to XNN + VX
                let v_src = if self.quirks.jump { inst.x } else { 0 };
                self.pc = inst.nnn + self.v[v_src] as u16;
            }
            0xC => {
                // VX = random number bitwise & with NN
                let random_byte: u8 = (self.rng.next_u32() & 0xFF) as u8;
                self.v[inst.x] = random_byte & inst.nn;
            }
            0xD => {
                // Alter Display
                let x_coord = self.v[inst.x] as usize % DISPLAY_WIDTH;
                let y_coord = self.v[inst.y] as usize % DISPLAY_HEIGHT;
                self.v[0xF] = 0; // Reset collision flag

                for index in 0..inst.n as usize {
                    let sprite_byte = self.memory[self.i as usize + index];

                    // Y-coordinate handling
                    let pixel_y = y_coord + index;
                    if self.quirks.clip && pixel_y >= DISPLAY_HEIGHT {
                        continue; // skip drawing if clipped vertically
                    }

                    for bit_index in 0..8 {
                        let pixel_x = x_coord + bit_index;
                        if self.quirks.clip && pixel_x >= DISPLAY_WIDTH {
                            continue; // skip drawing if clipped horizontally
                        }

                        // Apply wrapping if clipping is off
                        let px = if self.quirks.clip {
                            pixel_x
                        } else {
                            pixel_x % DISPLAY_WIDTH
                        };
                        let py = if self.quirks.clip {
                            pixel_y
                        } else {
                            pixel_y % DISPLAY_HEIGHT
                        };

                        let sprite_pixel_on = (sprite_byte >> (7 - bit_index)) & 1 == 1;
                        let current_pixel = self.display[py][px];

                        if sprite_pixel_on {
                            if current_pixel {
                                self.v[0xF] = 1; // Collision
                            }

                            self.display[py][px] ^= true;
                        }
                    }
                }
                self.draw_flag = true;
            }
            0xE => {
                match inst.nn {
                    0x9E => {
                        // Skip next instruction if X key is pressed
                        if self.keypad[self.v[inst.x] as usize] {
                            self.pc += 2;
                        }
                    }
                    0xA1 => {
                        // Skip next instruction if X key is NOT pressed
                        if !self.keypad[self.v[inst.x] as usize] {
                            self.pc += 2;
                        }
                    }
                    _ => { panic!("Unknown opcode: {:04X}", inst.instruction) }
                }
            }
            0xF => {
                match inst.nn {
                    // Timer Instructions
                    0x07 => {
                        // Set VX to current value of Delay Timer
                        self.v[inst.x] = self.delay_timer;
                    }
                    0x0A => {
                        if !self.wait_for_release { // If not actively waiting for key release
                            // Check if any button is pressed
                            for (i, &pressed) in self.keypad.iter().enumerate() {
                                if pressed {
                                    self.wait_for_release = true;
                                    self.wait_key = i;
                                    break;
                                }
                            }
                        } 
                        // If waiting for initial key press or key release
                        if !self.wait_for_release || (self.wait_for_release && self.keypad[self.wait_key]) {
                            // Don't advance to next instruction
                            self.pc -= 2;
                        } else { // The key was let go
                            self.v[inst.x] = self.wait_key as u8;
                            self.wait_for_release = false;
                        }
                    }
                    0x15 => {
                        // Sets the Delay Timer to the value in VX
                        self.delay_timer = self.v[inst.x];
                    }
                    0x18 => {
                        // Sets the sound timer to the value in VX
                        self.sound_timer = self.v[inst.x];
                    }
                    0x1E => {
                        // I = I + VX
                        let (result, overflow) = self.i.overflowing_add(self.v[inst.x] as u16);
                        self.i = result;
                        self.v[0xF] = if overflow { 1 } else { 0 };
                    }
                    0x29 => {
                        // I = location of sprite for digit in VX
                        self.i = FONTSET_START as u16 + (self.v[inst.x] as u16 * 5);
                    }
                    0x33 => {
                        // Store number in VX as three decimal digits, and stores in mem at location in reg I
                        let value = self.v[inst.x];
                        self.memory[self.i as usize] = value / 100;
                        self.memory[self.i as usize + 1] = (value % 100) / 10;
                        self.memory[self.i as usize + 2] = value % 10;
                    }
                    0x55 => {
                        // Store V0-VX variables in memory
                        for step in 0..=inst.x {
                            self.memory[self.i as usize + step] = self.v[step];
                        }
                        // Original Chip-8 incremented I, but modern don't update I
                        if self.quirks.load_store {
                            self.i += inst.x as u16 + 1;
                        }
                    }
                    0x65 => {
                        // Loads from memory variables into V0-VX
                        for step in 0..=inst.x {
                            self.v[step] = self.memory[self.i as usize + step];
                        }
                        // Original Chip-8 incremented I, but modern don't update I
                        if self.quirks.load_store {
                            self.i += inst.x as u16 + 1;
                        }
                    }
                    _ => { println!("Unknown opcode: {:04X}", inst.instruction); }
                }
            }
            _ => {
                panic!("IMPOSSIBLE NIBBLE! {}", inst.instruction)
            }
        }
        Ok(SUCCESSFUL_EXECUTION)        
    }

    pub fn cycle(&mut self) -> std::io::Result<()> {
        // Fetch
        let instruction: Instruction = self.fetch();
        
        if self.debug {
            print!("Instruction: 0x{:04X}\t", instruction.instruction);
            self.debug_print();            
        }

        // Decode/Execute
        let result = self.execute(instruction);

        if self.debug {
            self.paused = true;
        }
        
        result
    }
}
