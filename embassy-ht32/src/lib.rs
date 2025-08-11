#![no_std]

pub use ht32_hal as hal;
pub use ht32f523x2 as pac;

pub mod uart;

#[cfg(feature = "usb")]
pub mod usb;

pub fn init() {
}