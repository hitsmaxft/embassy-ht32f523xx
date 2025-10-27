//! Embassy-time driver implementation for HT32F523x2
//!
//! This module provides basic timer initialization for HT32F523x2.
//! Embassy-time will use its default SysTick-based driver for timing.

/// Initialize timer hardware for embassy-time
pub fn init() {
    // For now, just initialize basic timer hardware
    // Embassy-time will use SysTick for timing until we implement a full driver

    let timer = unsafe { &*crate::pac::Gptm0::ptr() };

    // Enable timer clock
    let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
    ckcu.apbccr1().modify(|_, w| w.gptm0en().set_bit());

    // Get system clock frequency
    let clocks = crate::rcc::get_clocks();
    let timer_clock = clocks.apb_clk().to_hz();

    // Calculate prescaler to get 1MHz timer frequency
    let prescaler = (timer_clock / 1_000_000) - 1;

    // Configure timer for basic operation
    timer.gptm_ctr().modify(|_, w| w.tme().clear_bit()); // Disable timer first
    timer.gptm_pscr().write(|w| unsafe { w.bits(prescaler) }); // Set prescaler
    timer.gptm_crr().write(|w| unsafe { w.bits(0xFFFFFFFF) }); // Set to maximum period
    timer.gptm_cntr().write(|w| unsafe { w.bits(0) }); // Reset counter

    // Configure for up-counting mode
    timer.gptm_mdcfr().modify(|_, w| w.tse().bit(true)); // Up counting

    // Start timer (for future use)
    timer.gptm_ctr().modify(|_, w| w.tme().set_bit());

    // Note: Embassy-time will use SysTick or another default mechanism for now
}