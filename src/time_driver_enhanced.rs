//! Enhanced Embassy time driver stub for HT32F523xx
//!
//! This is a stub implementation that forwards to the working GPTM-based driver
//! until the BFTM implementation is properly fixed.
//!
//! TODO: Revive the BFTM implementation when PAC integration issues are resolved

use embassy_time_driver::{Driver, Ticks};
use core::task::Waker;
use critical_section;

/// Stub enhanced time driver that forwards to the working GPTM implementation
pub struct EnhancedTimeDriver;

impl EnhancedTimeDriver {
    pub fn new() -> Self {
        Self
    }

    pub fn init(&self) -> Result<(), ()> {
        // Forward to existing time driver initialization
        critical_section::with(|cs| crate::time_driver::init(cs));
        Ok(())
    }

    pub fn now(&self) -> u64 {
        // Forward to embassy-time driver via embassy interface
        embassy_time::now().as_micros()
    }

    pub fn schedule_wake(&self, at: u64, waker: &Waker) -> Result<(), ()> {
        critical_section::with(|cs| {
            // Forward to the working time driver
            crate::time_driver::TIME_DRIVER.schedule_wake(at as Ticks, waker);
        });
        Ok(())
    }
}

impl Default for EnhancedTimeDriver {
    fn default() -> Self {
        Self::new()
    }
}