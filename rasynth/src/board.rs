use log::*;
use std::error::Error;
use std::thread;
use std::time::Duration;

use rppal::gpio::Gpio;
use rppal::i2c::I2c;
use rppal::system::DeviceInfo;

use display_interface_spi::SPIInterface;
use embedded_graphics::{
    mono_font::{ascii::FONT_10X20, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::*,
    text::Text,
};
use embedded_hal_bus::spi::ExclusiveDevice;
use mipidsi::{models::ST7789, options::ColorInversion, Builder};

use rppal::hal::Delay;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::process::ExitCode;

const LCD_DC: u8 = 23; // PIN 16 for LCD's DC(Data/Command Selection)
const LCD_BACKLIGH: u8 = 24; // PIN 18 for LCD's Backlight Control

// Display
const W: i32 = 240;
const H: i32 = 240;

pub fn test_gpio() -> Result<(), Box<dyn Error>> {
    println!("Blinking an LED on a {}.", DeviceInfo::new()?.model());
    let mut pin = Gpio::new()?.get(23)?.into_output();
    pin.set_high();
    thread::sleep(Duration::from_millis(500));
    pin.set_low();
    Ok(())
}

struct NoCs;

extern crate embedded_hal;
impl embedded_hal::digital::OutputPin for NoCs {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl embedded_hal::digital::ErrorType for NoCs {
    type Error = core::convert::Infallible;
}

/// test GPIO SPI LCD panel
pub fn test_display() {
    info!("Testing GPIO SPI LCD panel");

    // SPI_CLK -> LCD.SCL
    // SPI_MOSI -> LCD.SDA
    let gpio = Gpio::new().unwrap();
    let dc = gpio.get(LCD_DC).unwrap().into_output();
    let mut backlight = gpio.get(LCD_BACKLIGH).unwrap().into_output();

    // open raspi's /dev/spidev*
    let spi = Spi::new(Bus::Spi0, SlaveSelect::Ss1, 60_000_000_u32, Mode::Mode0).unwrap();
    let spi_device = ExclusiveDevice::new_no_delay(spi, NoCs).unwrap();
    let di = SPIInterface::new(spi_device, dc);
    let mut delay = Delay::new();
    let mut display = Builder::new(ST7789, di)
        .display_size(W as u16, H as u16)
        .invert_colors(ColorInversion::Inverted)
        .init(&mut delay)
        .unwrap();

    // Text
    let char_w = 10;
    let char_h = 20;
    let text_style = MonoTextStyle::new(&FONT_10X20, Rgb565::WHITE);
    let text = "Hello World ^_^;";
    let mut text_x = W;
    let mut text_y = H / 2;

    // Alternating color
    let colors = [Rgb565::RED, Rgb565::GREEN, Rgb565::BLUE];

    // Clear the display initially
    display.clear(colors[0]).unwrap();

    // Turn on backlight
    backlight.set_high();

    let start = std::time::Instant::now();
    let mut last = std::time::Instant::now();
    let mut led_flags = 0b000;
    let mut counter = 0;
    loop {
        let elapsed = last.elapsed().as_secs_f64();
        if elapsed < 0.125 {
            continue;
        }
        last = std::time::Instant::now();
        counter += 1;

        if counter == 256 {
            break;
        }

        // Fill the display with alternating colors every 8 frames
        display.clear(colors[(counter / 8) % colors.len()]).unwrap();

        // Draw text
        let right = Text::new(text, Point::new(text_x, text_y), text_style)
            .draw(&mut display)
            .unwrap();
        text_x = if right.x <= 0 { W } else { text_x - char_w };
    }

    // Turn off backlight and clear the display
    backlight.set_low();
    display.clear(Rgb565::BLACK).unwrap();

    info!("Finished testing GPIO SPI LCD panel");
}
