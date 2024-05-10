#![allow(unused_imports)]
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::adc::{self, Adc, Channel};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{self, AnyPin, Flex, Level, Output, OutputOpenDrain, Pull};
use embassy_rp::i2c;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{self, Driver, InterruptHandler};
use embassy_time::{Delay, Timer};
use embedded_hal_1::digital::{OutputPin, StatefulOutputPin};
use {defmt_rtt as _, panic_probe as _};

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

#[derive(Debug)]
enum Err {}

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => usb::InterruptHandler<USB>;
    ADC_IRQ_FIFO => adc::InterruptHandler;
});

#[embassy_executor::task]
async fn logger_task(driver: Driver<'static, USB>) {
    embassy_usb_logger::run!(1024, log::LevelFilter::Info, driver);
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let driver = Driver::new(p.USB, Irqs);
    spawner.spawn(logger_task(driver)).unwrap();

    let mut delay = Delay;

    let mut adc = Adc::new(p.ADC, Irqs, adc::Config::default());

    let mut adc_pin_0 = Channel::new_pin(p.PIN_26, Pull::None);
    let mut adc_pin_1 = Channel::new_pin(p.PIN_27, Pull::None);
    let mut ts = Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);

    let sda_pin = p.PIN_0;
    let scl_pin = p.PIN_1;
    let mut led = Output::new(p.PIN_21, Level::Low);

    let i2c = i2c::I2c::new_blocking(p.I2C0, scl_pin, sda_pin, i2c::Config::default());
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_8X13)
        .text_color(BinaryColor::On)
        .build();

    let mut str: String<1024> = String::new();

    Text::with_baseline("fdashfasd", Point::zero(), text_style, Baseline::Top)
        .draw(&mut display)
        .unwrap();
    display.flush().unwrap();

    // let ow_pin = Flex::new(p.PIN_22);
    let ow_pin = OutputOpenDrain::new(p.PIN_22, Level::Low);
    let mut one_wire_bus = one_wire_bus::OneWire::new(ow_pin).unwrap();

    let mut addr = Address(0x0);
    for device_address in one_wire_bus.devices(false, &mut delay) {
        let device_address = device_address.unwrap();
        if device_address.family_code() != ds18b20::FAMILY_CODE {
            continue;
        }

        log::info!("fdsafd");
        addr = device_address;
        break;
    }
    //
    // let sensor = Ds18b20::new::<Err>(addr).unwrap();

    let mut led_on = false;
    Timer::after_secs(5).await;
    loop {
        log::info!("addr: {:?}", addr);
        Timer::after_secs(1).await;

        str.clear();
        display.clear(BinaryColor::Off).unwrap();

        if led_on {
            led.set_low();
        } else {
            led.set_high();
        }
        led_on = !led_on;

        let temp = adc.read(&mut ts).await.unwrap();
        writeln!(&mut str, "Temp: {} degrees", convert_to_celsius(temp)).unwrap();
        let level = adc.read(&mut adc_pin_0).await.unwrap();
        writeln!(&mut str, "Adc0: {}", level).unwrap();
        let level = adc.read(&mut adc_pin_1).await.unwrap();
        writeln!(&mut str, "Adc1: {}", level).unwrap();
        // sensor
        //     .start_temp_measurement(&mut one_wire_bus, &mut delay)
        //     .unwrap();
        // let data = sensor.read_data(&mut one_wire_bus, &mut delay).unwrap();
        // writeln!(&mut str, "Temp - {:.3} C", data.temperature).unwrap();

        Text::with_baseline(&str, Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
    }
}

fn convert_to_celsius(raw_temp: u16) -> f32 {
    let temp = 27.0 - (raw_temp as f32 * 3.3 / 4096.0 - 0.706) / 0.001721;
    let sign = if temp < 0.0 { -1.0 } else { 1.0 };
    let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    (rounded_temp_x10 as f32) / 10.0
}
