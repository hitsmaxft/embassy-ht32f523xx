#![no_std]
#![deny(unused_must_use)]

//! Embassy async runtime and Hardware Abstraction Layer for HT32F523xx microcontrollers
//!
//! This crate provides both synchronous and asynchronous drivers for HT32F523xx series MCUs,
//! following the Embassy framework patterns used in embassy-stm32.
//!
//! ## Supported Chips
//! - HT32F52342 (64KB Flash, 8KB RAM, 5 Timers)
//! - HT32F52352 (128KB Flash, 16KB RAM, 6 Timers)
//!
//! ## Features
//! - `ht32f52342` - Enable support for HT32F52342
//! - `ht32f52352` - Enable support for HT32F52352 (default)
//! - `rt` - Enable runtime support (cortex-m-rt)
//! - `usb` - Enable USB device support
//!
//! ## Usage
//!
//! ```rust,no_run
//! #![no_std]
//! #![no_main]
//!
//! use embassy_executor::Spawner;
//! use embassy_ht32f523xx::{gpio, rcc, Config};
//! use embassy_time::Timer;
//! use {defmt_rtt as _, panic_probe as _};
//!
//! #[embassy_executor::main]
//! async fn main(_spawner: Spawner) {
//!     let p = embassy_ht32f523xx::init(Config::default());
//!
//!     let mut led = gpio::Output::new(p.PA0, gpio::Level::Low, gpio::Speed::Low);
//!
//!     loop {
//!         led.set_high();
//!         Timer::after_millis(500).await;
//!         led.set_low();
//!         Timer::after_millis(500).await;
//!     }
//! }
//! ```

// Re-export the PAC for direct register access
pub use ht32f523x2 as pac;

// Chip-specific memory configuration
#[cfg(flash_size_64k)]
pub const FLASH_SIZE: usize = 64 * 1024;
#[cfg(flash_size_128k)]
pub const FLASH_SIZE: usize = 128 * 1024;

#[cfg(ram_size_8k)]
pub const RAM_SIZE: usize = 8 * 1024;
#[cfg(ram_size_16k)]
pub const RAM_SIZE: usize = 16 * 1024;

// Chip-optimized buffer sizes
#[cfg(ram_size_8k)]
pub const LARGE_BUFFER_SIZE: usize = 2048; // Conservative for 8KB RAM
#[cfg(ram_size_16k)]
pub const LARGE_BUFFER_SIZE: usize = 4096; // Can use more with 16KB RAM

// Chip-specific configuration
pub mod chip;

// Core modules
pub mod interrupt;
pub mod time;
pub mod time_driver;

// Utility modules
pub mod fmt;

// Hardware abstraction layer modules
pub mod exti;
pub mod gpio;
pub mod rcc;
pub mod timer;
pub mod uart;
#[cfg(feature = "usb")]
pub mod usb;
pub mod flash;

// Re-exports for convenience
pub use embassy_executor;
pub use embassy_time;
pub use embassy_sync;

/// System configuration
pub struct Config {
    /// RCC (clock) configuration
    pub rcc: rcc::Config,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            rcc: rcc::Config::default(),
        }
    }
}

/// System peripherals
pub struct Peripherals {
    pub GPIOA: gpio::PortA,
    pub GPIOB: gpio::PortB,
    pub GPIOC: gpio::PortC,
    pub GPIOD: gpio::PortD,
    pub USART0: uart::Usart0,
    pub USART1: uart::Usart1,
    pub TIMER0: timer::Timer0,
    pub TIMER1: timer::Timer1,
    #[cfg(feature = "usb")]
    pub USB: usb::Usb,
    pub FLASH: flash::Flash,
}

/// Initialize the chip and return peripheral instances
pub fn init(config: Config) -> Peripherals {
    // Initialize clocks first
    let _clocks = rcc::init(config.rcc);

    // Initialize embassy-time driver using GPTM0
    time_driver::init();

    // Initialize interrupt system
    interrupt::init();

    // Initialize EXTI system
    exti::init();

    // Initialize GPIO ports
    let gpioa = gpio::PortA::new();
    let gpiob = gpio::PortB::new();
    let gpioc = gpio::PortC::new();
    let gpiod = gpio::PortD::new();

    // Initialize UART peripherals
    let usart0 = uart::Usart0::new();
    let usart1 = uart::Usart1::new();

    // Initialize Timer peripherals
    let timer0 = timer::Timer0::new();
    let timer1 = timer::Timer1::new();

    // Initialize USB peripheral if feature is enabled
    #[cfg(feature = "usb")]
    let usb = usb::Usb::new();

    // Initialize Flash controller
    let flash = flash::Flash::new();

    Peripherals {
        GPIOA: gpioa,
        GPIOB: gpiob,
        GPIOC: gpioc,
        GPIOD: gpiod,
        USART0: usart0,
        USART1: usart1,
        TIMER0: timer0,
        TIMER1: timer1,
        #[cfg(feature = "usb")]
        USB: usb,
        FLASH: flash,
    }
}

/// Prelude module - import commonly used types and traits
pub mod prelude {
    pub use crate::time::U32Ext;
    // TODO: Add other exports when modules are completed
}