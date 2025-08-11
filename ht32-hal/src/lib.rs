#![no_std]

pub use ht32f523x2 as pac;

pub mod gpio;
pub mod rcc;
pub mod time;
pub mod timer;
pub mod uart;

pub mod prelude {
    pub use crate::gpio::GpioExt;
    pub use crate::rcc::RccExt;
    pub use crate::time::U32Ext;
}
