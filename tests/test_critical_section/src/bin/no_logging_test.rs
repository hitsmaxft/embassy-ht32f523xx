//! Test critical sections without any logging inside them

#![no_std]
#![no_main]

use embassy_ht32f523xx as hal;
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;

static mut SHARED_DATA: u32 = 0;

#[entry]
fn main() -> ! {
    let _p = hal::init(hal::Config::default());

    defmt::info!("Starting no-logging critical section test");

    // Test without any logging inside critical sections
    let mut result = 0u32;

    // Basic test
    critical_section::with(|_| {
        unsafe {
            SHARED_DATA = 123;
        }
    });

    result = critical_section::with(|_| {
        unsafe { SHARED_DATA }
    });

    defmt::info!("Basic test result: {}", result);
    assert_eq!(result, 123);

    // Nested test without logging
    critical_section::with(|_| {
        unsafe {
            SHARED_DATA = 456;
        }

        critical_section::with(|_| {
            unsafe {
                SHARED_DATA = 789;
            }
        });

        unsafe {
            SHARED_DATA = 999;
        }
    });

    result = critical_section::with(|_| {
        unsafe { SHARED_DATA }
    });

    defmt::info!("Nested test result: {}", result);
    assert_eq!(result, 999);

    defmt::info!("✅ All tests completed successfully!");

    loop {
        cortex_m::asm::nop();
    }
}