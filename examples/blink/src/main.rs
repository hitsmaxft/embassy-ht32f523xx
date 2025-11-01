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

    info!("HT32 Blink Example Started");

    let mut leds = Leds::new();
    info!("LEDs initialized");

    let mut led1_on = true;
    info!("Starting blink loop");

    loop {
        if led1_on {
            leds.led1.set_high().unwrap();
            leds.led2.set_low().unwrap();
            info!("LED1 ON, LED2 OFF");
        } else {
            leds.led1.set_low().unwrap();
            leds.led2.set_high().unwrap();
            info!("LED1 OFF, LED2 ON");
        }

        led1_on = !led1_on;

        // Simple delay - no Embassy Timer
        for _ in 0..1_000_000 {
            cortex_m::asm::nop();
        }
        // Use Embassy Timer for async delay
        //Timer::after(Duration::from_millis(500)).await;
    }
}