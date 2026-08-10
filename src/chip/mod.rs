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
    /// Device Flash size class in KiB.
    pub flash_kb: u32,
    /// User-addressable main Flash; excludes the final 512-byte Option Byte page.
    pub usable_flash_bytes: u32,
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
    usable_flash_bytes: 64 * 1024,
    ram_kb: 8,
    flash_origin: 0x0000_0000,
    ram_origin: 0x2000_0000,
};

#[cfg(not(feature = "ht32f52342"))]
pub const MEMORY: Memory = Memory {
    flash_kb: 128,
    usable_flash_bytes: 128 * 1024 - 512,
    ram_kb: 16,
    flash_origin: 0x0000_0000,
    ram_origin: 0x2000_0000,
};

#[cfg(feature = "ht32f52342")]
pub const TIMERS: TimerConfig = TimerConfig {
    timer_count: 7, // 1 MCTM + 2 GPTM + 2 SCTM + 2 BFTM
    has_advanced_timers: true,
};

#[cfg(not(feature = "ht32f52342"))]
pub const TIMERS: TimerConfig = TimerConfig {
    timer_count: 7, // 1 MCTM + 2 GPTM + 2 SCTM + 2 BFTM
    has_advanced_timers: true,
};

pub const GPIO: GpioConfig = GpioConfig {
    port_count: 4, // GPIOA, GPIOB, GPIOC, GPIOD
    pins_per_port: 16,
};

#[cfg(feature = "ht32f52342")]
pub const PERIPHERALS: Peripherals = Peripherals {
    uart_count: 4, // 2 USART + 2 UART
    spi_count: 2,
    i2c_count: 2,
    adc_channels: 12,
    has_usb: true,
};

#[cfg(not(feature = "ht32f52342"))]
pub const PERIPHERALS: Peripherals = Peripherals {
    uart_count: 4, // 2 USART + 2 UART
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
