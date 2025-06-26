use linux_embedded_hal::I2cdev;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use embedded_graphics::{
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::Pixel,
    Drawable,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Open I2C bus
    let i2c = I2cdev::new("/dev/i2c-1").unwrap();
    let interface = I2CDisplayInterface::new(i2c);

    // Create display instance
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    // Initialize display
    display.init().unwrap();

    // Clear display
    display.clear(BinaryColor::Off).unwrap();

    // Create a text style
    let style = MonoTextStyle::new(&FONT_6X10, BinaryColor::On);
    
    let bitmap: [[bool; 128]; 64] = [[false; 128]; 64]; // Replace with your actual data
    for (y, row) in bitmap.iter().enumerate() {
        for (x, &pixel_on) in row.iter().enumerate() {
            if x % 2 == 0 && y % 2 != 0 {
                bitmap[x][y] = true;
            }
        }
    }

    for (y, row) in bitmap.iter().enumerate() {
        for (x, &pixel_on) in row.iter().enumerate() {
            if pixel_on {
                Pixel(Point::new(x as i32, y as i32), BinaryColor::On)
                    .draw(&mut display)
                    .unwrap();
            }
        }
    }
    // Show the result
    display.flush().unwrap();

    Ok(())
}
