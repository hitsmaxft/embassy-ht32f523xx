//! Time units and frequency definitions

use core::ops::{Div, Mul};

/// Frequency in Hertz
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hertz(pub u32);

impl Hertz {
    /// Create a frequency from Hz
    pub const fn hz(hz: u32) -> Self {
        Self(hz)
    }

    /// Create a frequency from kHz
    pub const fn khz(khz: u32) -> Self {
        Self(khz * 1_000)
    }

    /// Create a frequency from MHz
    pub const fn mhz(mhz: u32) -> Self {
        Self(mhz * 1_000_000)
    }

    /// Get the frequency in Hz
    pub const fn to_hz(self) -> u32 {
        self.0
    }

    /// Get the frequency in kHz
    pub const fn to_khz(self) -> u32 {
        self.0 / 1_000
    }

    /// Get the frequency in MHz
    pub const fn to_mhz(self) -> u32 {
        self.0 / 1_000_000
    }
}

impl From<u32> for Hertz {
    fn from(hz: u32) -> Self {
        Self::hz(hz)
    }
}

impl Mul<u32> for Hertz {
    type Output = Hertz;

    fn mul(self, rhs: u32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl Div<u32> for Hertz {
    type Output = Hertz;

    fn div(self, rhs: u32) -> Self::Output {
        Self(self.0 / rhs)
    }
}

/// Time duration in microseconds
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Microseconds(pub u32);

impl Microseconds {
    /// Create a duration from microseconds
    pub const fn us(us: u32) -> Self {
        Self(us)
    }

    /// Create a duration from milliseconds
    pub const fn ms(ms: u32) -> Self {
        Self(ms * 1_000)
    }

    /// Create a duration from seconds
    pub const fn s(s: u32) -> Self {
        Self(s * 1_000_000)
    }

    /// Get the duration in microseconds
    pub const fn to_us(self) -> u32 {
        self.0
    }

    /// Get the duration in milliseconds
    pub const fn to_ms(self) -> u32 {
        self.0 / 1_000
    }

    /// Get the duration in seconds
    pub const fn to_s(self) -> u32 {
        self.0 / 1_000_000
    }
}

impl From<u32> for Microseconds {
    fn from(us: u32) -> Self {
        Self::us(us)
    }
}

/// Extension trait to create time units from integers
pub trait U32Ext {
    /// Create a frequency from Hz
    fn hz(self) -> Hertz;
    /// Create a frequency from kHz
    fn khz(self) -> Hertz;
    /// Create a frequency from MHz
    fn mhz(self) -> Hertz;

    /// Create a duration from microseconds
    fn us(self) -> Microseconds;
    /// Create a duration from milliseconds
    fn ms(self) -> Microseconds;
    /// Create a duration from seconds
    fn s(self) -> Microseconds;
}

impl U32Ext for u32 {
    fn hz(self) -> Hertz {
        Hertz::hz(self)
    }

    fn khz(self) -> Hertz {
        Hertz::khz(self)
    }

    fn mhz(self) -> Hertz {
        Hertz::mhz(self)
    }

    fn us(self) -> Microseconds {
        Microseconds::us(self)
    }

    fn ms(self) -> Microseconds {
        Microseconds::ms(self)
    }

    fn s(self) -> Microseconds {
        Microseconds::s(self)
    }
}