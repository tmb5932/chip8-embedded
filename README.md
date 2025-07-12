# chip8-embedded
A rework of my chip8 emulator to work with on a Raspberry Pi with SSD1306 OLED Display


# Raspberry Pi GPIO Pins

## Keypad
I used a 4x4 matrix keypad, so there are 8 pins, 4 for rows, 4 for columns.
### Rows
2, 3, 4, 27

### Columns
0, 5, 6, 13

## SSD1309
MOSI => 10
CLK => ?
CS => ?
DC => 23
RST => 24

### Buzzer
25

### LED
26