//! HT32F52342 specific configurations

use super::{ChipConfig, GpioConfig, Memory, Peripherals, TimerConfig};

/// HT32F52342 chip configuration
pub const CONFIG: ChipConfig = ChipConfig {
    memory: Memory {
        flash_kb: 64,
        usable_flash_bytes: 64 * 1024,
        ram_kb: 8,
        flash_origin: 0x0000_0000,
        ram_origin: 0x2000_0000,
    },
    timers: TimerConfig {
        timer_count: 7, // 1 MCTM + 2 GPTM + 2 SCTM + 2 BFTM
        has_advanced_timers: true,
    },
    gpio: GpioConfig {
        port_count: 4, // GPIOA, GPIOB, GPIOC, GPIOD
        pins_per_port: 16,
    },
    peripherals: Peripherals {
        uart_count: 4,    // USART0/1 and UART0/1
        spi_count: 2,     // SPI0, SPI1
        i2c_count: 2,     // I2C0, I2C1
        adc_channels: 12, // ADC 12 external channels
        has_usb: true,    // USB Device support
    },
};

/// Clock configuration constants
pub mod clocks {
    pub const HSI_FREQ: u32 = 8_000_000; // 8 MHz internal oscillator
    pub const MAX_SYSCLK: u32 = 48_000_000; // 48 MHz maximum system clock
    pub const MAX_AHB_FREQ: u32 = 48_000_000;
    pub const MAX_APB_FREQ: u32 = 48_000_000;
}

/// Flash memory constants
pub mod flash {
    pub const FLASH_SIZE: u32 = 64 * 1024;
    pub const PAGE_SIZE: u32 = 512;
    pub const PAGE_COUNT: u32 = FLASH_SIZE / PAGE_SIZE;
}

/// SRAM constants
pub mod sram {
    pub const SRAM_SIZE: u32 = 8 * 1024;
    pub const SRAM_START: u32 = 0x2000_0000;
    pub const SRAM_END: u32 = SRAM_START + SRAM_SIZE;
}
