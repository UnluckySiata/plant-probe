#![no_std]
#![no_main]

mod state;

use embassy_executor::Spawner;
use embassy_rp::adc::{self, Adc, Channel};
use embassy_rp::bind_interrupts;
use embassy_rp::gpio::{Input, Level, Output, OutputOpenDrain, Pull};
use embassy_rp::i2c;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{self, Driver};
use embassy_time::{Delay, Timer};
use state::State;
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
    let mut adc_pin_2 = Channel::new_pin(p.PIN_28, Pull::None);

    let setting_btn = Input::new(p.PIN_18, Pull::Up);
    let switch_btn = Input::new(p.PIN_19, Pull::Up);
    let progress_btn = Input::new(p.PIN_20, Pull::Up);

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

    let mut s = State::new();

    loop {
        Timer::after_millis(50).await;

        display.clear(BinaryColor::Off).unwrap();

        if setting_btn.get_level() == Level::Low {
            s.switch_config();
        }

        if switch_btn.get_level() == Level::Low {
            s.switch_state();
        }

        if progress_btn.get_level() == Level::Low {
            s.progress();
        }

        if s.bad_level() {
            led.set_high();
        } else {
            led.set_low();
        }

        if s.is_measuring() {
            let light = adc.read(&mut adc_pin_0).await.unwrap();
            let humidity = adc.read(&mut adc_pin_1).await.unwrap();

            sensor
                .start_temp_measurement(&mut one_wire_bus, &mut delay)
                .unwrap();
            let data = sensor.read_data(&mut one_wire_bus, &mut delay).unwrap();

            s.update_measurements(data.temperature, light, humidity);
        } else if s.is_configuring() {
            let level = adc.read(&mut adc_pin_2).await.unwrap();

            s.update_config(level);
        }

        Text::with_baseline(s.get_repr(), Point::zero(), text_style, Baseline::Top)
            .draw(&mut display)
            .unwrap();
        display.flush().unwrap();
    }
}
