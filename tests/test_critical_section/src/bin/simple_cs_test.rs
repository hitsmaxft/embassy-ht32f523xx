//! Simple critical section test without defmt

#![no_std]
#![no_main]

use panic_probe as _;
use embassy_ht32f523xx as _;
use cortex_m_rt::entry;

static mut TEST_VAR: u32 = 0;

#[entry]
fn main() -> ! {
    // Test critical sections without any defmt logging
    critical_section::with(|_| {
        unsafe { TEST_VAR = 42; }
    });

    let val = critical_section::with(|_| {
        unsafe { TEST_VAR }
    });

    // Use the value to prevent optimization
    if val != 42 {
        // If assertion fails, loop forever
        loop {
            cortex_m::asm::nop();
        }
    }

    // Test nesting
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

    if final_val != 300 {
        loop {
            cortex_m::asm::nop();
        }
    }

    // Success - blink loop
    loop {
        cortex_m::asm::nop();
    }
}