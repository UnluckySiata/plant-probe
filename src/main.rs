#![no_std]
#![no_main]

use bsp::entry;
use embedded_hal::digital::v2::OutputPin;
use panic_halt as _;

use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

// Provide an alias for our BSP so we can switch targets quickly.
use rp_pico as bsp;

use bsp::hal::{
    clocks::ClockSource,
    fugit::RateExtU32,
    gpio::{FunctionI2C, Pin},
    pac,
};

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = bsp::hal::Watchdog::new(pac.WATCHDOG);
    let sio = bsp::hal::sio::Sio::new(pac.SIO);

    let clocks = bsp::hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.get_freq().to_Hz());
    // The single-cycle I/O block controls our GPIO pins

    // Set the pins to their default state
    let pins = bsp::hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let sda_pin: Pin<_, FunctionI2C, _> = pins.gpio0.reconfigure();
    let scl_pin: Pin<_, FunctionI2C, _> = pins.gpio1.reconfigure();

    let i2c = bsp::hal::I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin,
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(BinaryColor::On)
        .build();

    display.flush().unwrap();

    let mut led = pins.gpio28.into_push_pull_output();
    led.set_high().unwrap();
    loop {
        display.clear(BinaryColor::Off).unwrap();
        Text::with_baseline("Hello world!", Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        Text::with_baseline("Hello Rust!", Point::new(0, 16), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
        
        delay.delay_ms(500);

        display.clear(BinaryColor::Off).unwrap();
        Text::with_baseline("Hello Rust!", Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();

        Text::with_baseline("Hello World!", Point::new(0, 16), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();

        delay.delay_ms(500);
    }
}
