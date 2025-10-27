//! Chip-specific configurations and memory layouts

#[cfg(feature = "ht32f52342")]
pub mod ht32f52342;
#[cfg(feature = "ht32f52352")]
pub mod ht32f52352;

// Re-export the current chip module
#[cfg(all(feature = "ht32f52342", not(feature = "ht32f52352")))]
pub use ht32f52342 as current;
#[cfg(feature = "ht32f52352")]
pub use ht32f52352 as current;

/// Memory configuration for the chip
pub struct Memory {
    pub flash_kb: u32,
    pub ram_kb: u32,
    pub flash_origin: u32,
    pub ram_origin: u32,
}

/// Timer configuration differences
pub struct TimerConfig {
    pub timer_count: u8,
    pub has_advanced_timers: bool,
}

/// GPIO configuration
pub struct GpioConfig {
    pub port_count: u8,
    pub pins_per_port: u8,
}

/// Peripheral availability
pub struct Peripherals {
    pub uart_count: u8,
    pub spi_count: u8,
    pub i2c_count: u8,
    pub adc_channels: u8,
    pub has_usb: bool,
}

/// Complete chip configuration
pub struct ChipConfig {
    pub memory: Memory,
    pub timers: TimerConfig,
    pub gpio: GpioConfig,
    pub peripherals: Peripherals,
}

// Current chip configuration constants
#[cfg(feature = "ht32f52342")]
pub const MEMORY: Memory = Memory {
    flash_kb: 64,
    ram_kb: 8,
    flash_origin: 0x0000_0000,
    ram_origin: 0x2000_0000,
};

#[cfg(not(feature = "ht32f52342"))]
pub const MEMORY: Memory = Memory {
    flash_kb: 128,
    ram_kb: 16,
    flash_origin: 0x0000_0000,
    ram_origin: 0x2000_0000,
};

#[cfg(feature = "ht32f52342")]
pub const TIMERS: TimerConfig = TimerConfig {
    timer_count: 5,  // TIM0-TIM4
    has_advanced_timers: false,
};

#[cfg(not(feature = "ht32f52342"))]
pub const TIMERS: TimerConfig = TimerConfig {
    timer_count: 6,  // TIM0-TIM5
    has_advanced_timers: false,
};

pub const GPIO: GpioConfig = GpioConfig {
    port_count: 3,      // GPIOA, GPIOB, GPIOC
    pins_per_port: 16,
};

#[cfg(feature = "ht32f52342")]
pub const PERIPHERALS: Peripherals = Peripherals {
    uart_count: 2,
    spi_count: 2,
    i2c_count: 2,
    adc_channels: 10,
    has_usb: true,
};

#[cfg(not(feature = "ht32f52342"))]
pub const PERIPHERALS: Peripherals = Peripherals {
    uart_count: 2,
    spi_count: 2,
    i2c_count: 2,
    adc_channels: 12,
    has_usb: true,
};

pub const CHIP: ChipConfig = ChipConfig {
    memory: MEMORY,
    timers: TIMERS,
    gpio: GPIO,
    peripherals: PERIPHERALS,
};