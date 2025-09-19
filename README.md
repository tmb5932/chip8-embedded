# chip8-embedded
A rework of my [Chip8 Emulator](https://github.com/tmb5932/chip8-emulator) to work with on a Raspberry Pi with SSD1309 OLED Display

## The Goal
My main goal of this project (besides learning more about Rust) was to alter my [Chip8 Emulator](https://github.com/tmb5932/chip8-emulator) to be runnable on a Raspberry Pi. 

The much more distant hope was to create a nice looking "Chip8 Handheld Device", that looks as if it wasn't designed and put together by someone who wasn't 3d printing or soldering for the first time.

## Current Support
- SSD1309 over SPI
- a 4x4 matrix keypad
- Buzzer & Led
- Support for my custom game ROM
- a quit button to close current ROM, and choose another to play
- Full Chip8 emulator support
- Currently Supported Quirks:
    - Load / Store
    - Shift
    - Jump
    - vF Reset
    - Clip

### Future Improvements
- The main one is to add some sort of clock limiter, as some games are currently extremely difficult / impossible due to how fast the game is updating
- I also plan on designing / creating my own 4x4 keypad for this at some point
- A custom case to hold the parts, probably 3d printed

## Raspberry Pi 5 GPIO Pins

### Keypad
I used a 4x4 matrix keypad, so there are 8 pins, 4 for rows, 4 for columns.
#### Rows: 
Pins 2, 3, 4, 27

#### Columns: 
Pins 0, 5, 6, 13

### SSD1309
MOSI: Pin 10

CLK: Pin 11

CS: Pin 8

DC: Pin 23

RST: Pin 24

### Buzzer
Pin 25

### LED
Pin 26

## Road Bumps
Along the path of working on this, there have been a few road bumps.

- The original display was supposed to be a SSD1306 over I2C (IIC). Unfortunately I forgot that I2C is a bit too slow for rapidly updating game screens. The solution to this was to swap to SPI, which meant a new display. I went with an SSD1309.
- The SSD1306 issue led into my second issue, which was that the crate I was using for communicating with the display only had support for I2C, not SPI.
    - After more than 10 hours of straight research into finding a working SPI Display library, I gave up, and made my own simple Display driver, using rppal's SPI crate.
- Buying the SSD1309 led directly into another issue, I had no clue how to solder. The ssd1309's, or at least the ones I find on Amazon, come un soldered, with some pin headers to attach yourself if you feel inclined. This led to me going and learning how to solder (from youtube), and buying everything I thought I would need.
- This one is closely related to the first two issues as well. I tested and created this on the Raspberry Pi 5, which has a new "RP1 I/O controller chip". This meant that nearly all standard GPIO libraries for the Raspberry Pi's didn't work with 5, and I needed to find one that did. Eventually I found rppal, which seems to work.


## Author
Travis Brown (tmb5932)