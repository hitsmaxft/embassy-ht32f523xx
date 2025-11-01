#![no_std]
#![no_main]

use cortex_m_rt::entry;
use embedded_hal::digital::OutputPin;
use ht32_bsp::Leds;
use embassy_ht32f523xx::{self, Config};
use panic_halt as _;

#[entry]
fn main() -> ! {
    // Initialize HAL
    let config = Config::default();
    let _p = embassy_ht32f523xx::init(config);

    let mut leds = Leds::new();

    let mut led1_on = true;

    loop {
        // Toggle LEDs
        if led1_on {
            leds.led1.set_high().unwrap();
            leds.led2.set_low().unwrap();
        } else {
            leds.led1.set_low().unwrap();
            leds.led2.set_high().unwrap();
        }
        led1_on = !led1_on;

        // Simple delay - no Embassy Timer
        for _ in 0..1_000_000 {
            cortex_m::asm::nop();
        }
    }
}