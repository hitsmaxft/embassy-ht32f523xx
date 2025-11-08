//! Test only nested critical sections

#![no_std]
#![no_main]

use embassy_ht32f523xx as hal;
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let _p = hal::init(hal::Config::default());

    defmt::info!("Starting nested-only critical section test");

    // Simple nested test
    defmt::info!("Testing nested critical sections");

    let mut test_value = 0u32;

    critical_section::with(|_| {
        defmt::info!("Entered outer critical section");
        test_value = 100;

        critical_section::with(|_| {
            defmt::info!("Entered inner critical section");
            test_value = 200;
        });

        defmt::info!("Exited inner critical section");
        test_value = 300;
    });

    defmt::info!("Exited outer critical section");
    defmt::info!("Final test value: {}", test_value);

    assert_eq!(test_value, 300, "Nested critical section failed");

    defmt::info!("✅ Nested critical section test passed!");

    loop {
        cortex_m::asm::nop();
    }
}