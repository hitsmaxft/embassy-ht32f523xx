#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embedded_hal::digital::OutputPin;
use ht32_bsp::Leds;
use embassy_ht32f523xx::{self, Config};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) -> ! {
    // Initialize HAL
    let config = Config::default();
    let _p = embassy_ht32f523xx::init(config);

    info!("HT32 Blink Example Started - Testing async timers!");

    let mut leds = Leds::new();
    info!("LEDs initialized");

    info!("Starting async blink loop with duration=1s");

    // Let's start with simple 1-second intervals to test the async timing
    loop {
        leds.led1.set_high().unwrap();
        leds.led2.set_low().unwrap();
        info!("LED1 ON, LED2 OFF");

        // Test basic async timer
        Timer::after(Duration::from_millis(1000)).await;

        leds.led1.set_low().unwrap();
        leds.led2.set_high().unwrap();
        info!("LED1 OFF, LED2 ON");

        // Test another async timer
        Timer::after(Duration::from_millis(1000)).await;

        info!("Completed one blink cycle");
    }
}