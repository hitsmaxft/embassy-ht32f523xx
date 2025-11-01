//! Formatting utilities for debugging

use core::fmt::Write;

/// A writer that ignores everything written to it
pub struct Sink;

impl Write for Sink {
    fn write_str(&mut self, _s: &str) -> core::fmt::Result {
        Ok(())
    }
}

/// Initialize logging/formatting infrastructure
pub fn init() {
    // This function can be extended to setup RTT, defmt, or other logging
}

/// Print to defmt if available (no-op for now)
#[cfg(feature = "defmt")]
pub fn println(_args: core::fmt::Arguments) {
    // TODO: Implement proper defmt formatting
}

/// Print to defmt if available (no-op otherwise)
#[cfg(not(feature = "defmt"))]
pub fn println(_args: core::fmt::Arguments) {
    // No-op when defmt is not available
}

// Note: Panic handler is intentionally not provided by the HAL
// Applications should choose their own panic handler (panic-probe, panic-halt, etc.)