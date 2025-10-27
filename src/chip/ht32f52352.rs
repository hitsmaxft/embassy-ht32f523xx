//! HT32F52352 specific configurations

use super::{ChipConfig, Memory, TimerConfig, GpioConfig, Peripherals};

/// HT32F52352 chip configuration
pub const CONFIG: ChipConfig = ChipConfig {
    memory: Memory {
        flash_kb: 128,
        ram_kb: 16,
        flash_origin: 0x0000_0000,
        ram_origin: 0x2000_0000,
    },
    timers: TimerConfig {
        timer_count: 6,  // TIM0-TIM5
        has_advanced_timers: false,
    },
    gpio: GpioConfig {
        port_count: 3,      // GPIOA, GPIOB, GPIOC
        pins_per_port: 16,
    },
    peripherals: Peripherals {
        uart_count: 2,    // USART0, USART1
        spi_count: 2,     // SPI0, SPI1
        i2c_count: 2,     // I2C0, I2C1
        adc_channels: 12, // ADC 12 channels
        has_usb: true,    // USB Device support
    },
};

/// Clock configuration constants
pub mod clocks {
    pub const HSI_FREQ: u32 = 8_000_000;  // 8 MHz internal oscillator
    pub const MAX_SYSCLK: u32 = 48_000_000; // 48 MHz maximum system clock
    pub const MAX_AHB_FREQ: u32 = 48_000_000;
    pub const MAX_APB_FREQ: u32 = 48_000_000;
}

/// Flash memory constants
pub mod flash {
    pub const FLASH_SIZE: u32 = 128 * 1024;
    pub const PAGE_SIZE: u32 = 1024;
    pub const PAGE_COUNT: u32 = FLASH_SIZE / PAGE_SIZE;
}

/// SRAM constants
pub mod sram {
    pub const SRAM_SIZE: u32 = 16 * 1024;
    pub const SRAM_START: u32 = 0x2000_0000;
    pub const SRAM_END: u32 = SRAM_START + SRAM_SIZE;
}