#![no_std]

pub use ht32_hal as hal;
pub use ht32f523x2 as pac;

#[cfg(feature = "esk32-30501")]
pub mod esk32_30501;

#[cfg(feature = "esk32-30501")]
pub use esk32_30501::*;