#![no_std]

pub use embassy_ht32f523xx as hal;
pub use embassy_ht32f523xx::pac;

#[cfg(feature = "esk32-30501")]
pub mod esk32_30501;

#[cfg(feature = "esk32-30501")]
pub use esk32_30501::*;