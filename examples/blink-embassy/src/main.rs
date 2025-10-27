#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embedded_hal::digital::OutputPin;
use ht32_bsp::Leds;
use embassy_ht32f523xx::{self, Config};
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting blink example");

    // Initialize HAL
    let config = Config::default();
    let _p = embassy_ht32f523xx::init(config);

    let mut leds = Leds::new();

    info!("LED blink started");

    loop {
        info!("LED ON - led1");
        leds.led1.set_high().unwrap();
        leds.led2.set_low().unwrap();
        leds.led3.set_low().unwrap();

        Timer::after_millis(500).await;

        info!("LED ON - led2");
        leds.led1.set_low().unwrap();
        leds.led2.set_high().unwrap();
        leds.led3.set_low().unwrap();

        Timer::after_millis(500).await;

        info!("LED ON - led3");
        leds.led1.set_low().unwrap();
        leds.led2.set_low().unwrap();
        leds.led3.set_high().unwrap();

        Timer::after_millis(500).await;
    }
}