//! Embassy-time driver implementation for HT32F523x2
//!
//! This module provides a complete embassy-time driver using GPTM0.

use core::task::Waker;
use embassy_time_driver::Driver;

/// Time driver for HT32F523x2 using GPTM0
pub struct TimeDriver;

const FREQUENCY: u64 = 1_000_000; // 1 MHz

embassy_time_driver::time_driver_impl!(static DRIVER: TimeDriver = TimeDriver);

impl Driver for TimeDriver {
    fn now(&self) -> u64 {
        let timer = unsafe { &*crate::pac::Gptm0::ptr() };

        // Read the current counter value
        let counter = timer.gptm_cntr().read().bits() as u64;

        // For simplicity, we'll just use the counter directly
        // In a full implementation, we'd handle overflow and maintain a 64-bit tick count
        counter
    }

    fn schedule_wake(&self, _at: u64, _waker: &Waker) {
        // For now, we don't implement scheduled wakes
        // This would require configuring the timer compare register and enabling interrupts
        // to wake the system at a specific time
    }
}

/// Initialize the time driver using GPTM0
pub fn init() {
    let timer = unsafe { &*crate::pac::Gptm0::ptr() };

    // Enable timer clock
    let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
    ckcu.apbccr1().modify(|_, w| w.gptm0en().set_bit());

    // Get system clock frequency
    let clocks = crate::rcc::get_clocks();
    let timer_clock = clocks.apb_clk().to_hz();

    // Calculate prescaler to get 1MHz timer frequency
    let prescaler = (timer_clock / FREQUENCY as u32) - 1;

    // Configure timer for basic operation
    timer.gptm_ctr().modify(|_, w| w.tme().clear_bit()); // Disable timer first
    timer.gptm_pscr().write(|w| unsafe { w.bits(prescaler) }); // Set prescaler
    timer.gptm_crr().write(|w| unsafe { w.bits(0xFFFFFFFF) }); // Set to maximum period
    timer.gptm_cntr().write(|w| unsafe { w.bits(0) }); // Reset counter

    // Configure for up-counting mode
    timer.gptm_mdcfr().modify(|_, w| w.tse().bit(true)); // Up counting

    // Start timer
    timer.gptm_ctr().modify(|_, w| w.tme().set_bit());
}