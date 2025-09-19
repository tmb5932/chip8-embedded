pub struct Instruction {
    pub instruction: u16,
    pub nibble: u8,
    pub x: usize,
    pub y: usize,
    pub n: u8,
    pub nn: u8,
    pub nnn: u16
}

impl Instruction {
    pub fn new(inst: u16) -> Self {
        Instruction {
            instruction: inst,
            nibble: ((inst & 0xF000) >> 12) as u8,
            // X: The second nibble. Used to look up one of the 16 registers (VX) from V0 through VF.
            x: ((inst & 0x0F00) >> 8) as usize,
            // Y: The third nibble. Also used to look up one of the 16 registers (VY) from V0 through VF.
            y: ((inst & 0x00F0) >> 4) as usize,
            // N: The fourth nibble. A 4-bit number.
            n: (inst & 0x000F) as u8,
            // NN: The second byte (third and fourth nibbles). An 8-bit immediate number.
            nn: (inst & 0x00FF) as u8,
            // NNN: The second, third and fourth nibbles. A 12-bit immediate memory address.
            nnn: inst & 0x0FFF,
        }
    }
}
