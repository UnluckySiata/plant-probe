#![no_std]
#![no_main]

use bsp::entry;
use embedded_hal::digital::v2::OutputPin;
use panic_halt as _;

use embedded_graphics::{
    mono_font::{ascii::FONT_8X13, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use ds18b20::{self, Ds18b20};
use one_wire_bus::{self, Address};

use core::fmt::Write;
use heapless::String;

// Provide an alias for our BSP so we can switch targets quickly.
use rp_pico as bsp;

use bsp::hal::{
    adc::{Adc, AdcPin},
    clocks::ClockSource,
    fugit::RateExtU32,
    gpio::{FunctionI2C, Pin},
    pac,
};

const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[derive(Debug)]
enum Err {}

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

    let pins = bsp::hal::gpio::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led = pins.gpio28.into_push_pull_output();
    led.set_high().unwrap();

    // pins for oled i2c display
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
        .font(&FONT_8X13)
        .text_color(BinaryColor::On)
        .build();

    let mut str: String<1024> = String::new();

    let ow_pin = pins.gpio22.into_push_pull_output();
    let mut one_wire_bus = one_wire_bus::OneWire::new(ow_pin).unwrap();

    let mut addr = Address(0x0);
    for device_address in one_wire_bus.devices(false, &mut delay) {
        let device_address = device_address.unwrap();
        if device_address.family_code() != ds18b20::FAMILY_CODE {
            continue;
        }

        addr = device_address;
        break;
    }

    let mut adc = Adc::new(pac.ADC, &mut pac.RESETS);
    let adc_pin_0 = AdcPin::new(pins.gpio27.into_floating_input()).unwrap();
    adc.free_running(&adc_pin_0);
    adc.wait_ready();

    let sensor = Ds18b20::new::<Err>(addr).unwrap();

    let mut led_on = true;
    loop {
        delay.delay_ms(1000);

        if led_on {
            led.set_low().unwrap();
        } else {
            led.set_high().unwrap();
        }
        led_on = !led_on;

        str.clear();
        display.clear(BinaryColor::Off).unwrap();

        sensor
            .start_temp_measurement(&mut one_wire_bus, &mut delay)
            .unwrap();
        let data = sensor.read_data(&mut one_wire_bus, &mut delay).unwrap();
        writeln!(&mut str, "Temp - {:.3} C", data.temperature).unwrap();

        let counts = adc.read_single();
        writeln!(&mut str, "Value: {}", counts).unwrap();

        Text::with_baseline(&str, Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
    }
}
