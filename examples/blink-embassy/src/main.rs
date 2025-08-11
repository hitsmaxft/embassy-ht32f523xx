#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embedded_hal::digital::OutputPin;
use ht32_bsp::Leds;
use ht32_hal::prelude::*;
use ht32_hal::rcc::RccExt;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting blink example");

    let dp = unsafe { ht32_bsp::pac::Peripherals::steal() };
    
    let rcc = dp.ckcu.constrain();
    let _clocks = rcc.configure()
        .hclk(48.mhz())
        .freeze();

    embassy_ht32::init();

    let mut leds = Leds::new();

    info!("LED blink started");

    loop {
        info!("LED ON");
        leds.led1.set_high().unwrap();
        leds.led2.set_low().unwrap();
        leds.led3.set_low().unwrap();
        
        cortex_m::asm::delay(8_000_000);

        info!("LED OFF");
        leds.led1.set_low().unwrap();
        leds.led2.set_high().unwrap();
        leds.led3.set_low().unwrap();
        
        cortex_m::asm::delay(8_000_000);

        leds.led1.set_low().unwrap();
        leds.led2.set_low().unwrap();
        leds.led3.set_high().unwrap();
        
        cortex_m::asm::delay(8_000_000);
    }
}