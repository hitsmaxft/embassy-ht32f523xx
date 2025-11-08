//! Absolute minimal critical section test without HAL

#![no_std]
#![no_main]

use embassy_ht32f523xx as _; // Import to link critical section symbols
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;

static mut TEST_VAR: u32 = 0;

#[entry]
fn main() -> ! {
    defmt::info!("Starting absolute minimal critical section test");

    // Test without HAL initialization at all
    critical_section::with(|_| {
        unsafe { TEST_VAR = 42; }
    });

    let val = critical_section::with(|_| {
        unsafe { TEST_VAR }
    });

    defmt::info!("Test result: {}", val);
    assert_eq!(val, 42);

    defmt::info!("✅ Basic test passed!");

    // Quick nested test
    critical_section::with(|_| {
        unsafe { TEST_VAR = 100; }
        critical_section::with(|_| {
            unsafe { TEST_VAR = 200; }
        });
        unsafe { TEST_VAR = 300; }
    });

    let final_val = critical_section::with(|_| {
        unsafe { TEST_VAR }
    });

    defmt::info!("Nested test result: {}", final_val);
    assert_eq!(final_val, 300);

    defmt::info!("✅ All tests passed!");

    loop {
        cortex_m::asm::nop();
    }
}