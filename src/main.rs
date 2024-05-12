#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::adc::{self, Adc, Channel};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, OutputOpenDrain, Pull};
use embassy_rp::i2c;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{self, Driver};
use embassy_time::{Delay, Timer};
use {defmt_rtt as _, panic_probe as _};

use embedded_graphics::{
    mono_font::{ascii::FONT_6X9, MonoTextStyleBuilder},
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

const ADC_MAX: u16 = 4096;

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
    let mut adc_pin_2 = Channel::new_pin(p.PIN_28, Pull::None);
    let mut ts = Channel::new_temp_sensor(p.ADC_TEMP_SENSOR);

    let btn_pin = Input::new(p.PIN_20, Pull::Up);

    let sda_pin = p.PIN_0;
    let scl_pin = p.PIN_1;
    let mut led = Output::new(p.PIN_21, Level::Low);

    let i2c = i2c::I2c::new_blocking(p.I2C0, scl_pin, sda_pin, i2c::Config::default());
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();

    display.init().unwrap();

    let text_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X9)
        .text_color(BinaryColor::On)
        .build();

    let mut str: String<1024> = String::new();

    let ow_pin = OutputOpenDrain::new(p.PIN_22, Level::Low);
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
    let sensor = Ds18b20::new::<Err>(addr).unwrap();

    loop {
        Timer::after_millis(500).await;

        str.clear();
        display.clear(BinaryColor::Off).unwrap();

        match btn_pin.get_level() {
            Level::Low => {
                led.set_high();
            }
            Level::High => {
                led.set_low();
            }
        }

        let level = adc.read(&mut adc_pin_0).await.unwrap();
        writeln!(&mut str, "Adc0: {:.2}", adc_ratio(level, false)).unwrap();

        let level = adc.read(&mut adc_pin_1).await.unwrap();
        writeln!(&mut str, "Adc1: {:.2}", adc_ratio(level, true)).unwrap();

        let level = adc.read(&mut adc_pin_2).await.unwrap();
        writeln!(&mut str, "Adc2: {:.2}", adc_ratio(level, false)).unwrap();

        let temp = adc.read(&mut ts).await.unwrap();
        writeln!(&mut str, "Temp inside: {:.3}", convert_to_celsius(temp)).unwrap();

        sensor
            .start_temp_measurement(&mut one_wire_bus, &mut delay)
            .unwrap();
        let data = sensor.read_data(&mut one_wire_bus, &mut delay).unwrap();
        writeln!(&mut str, "Temp outside: {:.3}", data.temperature).unwrap();

        Text::with_baseline(&str, Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
    }
}

fn adc_ratio(raw_read: u16, inversed: bool) -> f32 {
    let measurement = match inversed {
        true => ADC_MAX - raw_read,
        false => raw_read,
    };
    measurement as f32 / ADC_MAX as f32
}

fn convert_to_celsius(raw_temp: u16) -> f32 {
    let temp = 27.0 - (raw_temp as f32 * 3.3 / 4096.0 - 0.706) / 0.001721;
    let sign = if temp < 0.0 { -1.0 } else { 1.0 };
    let rounded_temp_x10: i16 = ((temp * 10.0) + 0.5 * sign) as i16;
    (rounded_temp_x10 as f32) / 10.0
}
