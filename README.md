# HT32 Embassy Project

A complete Embassy async runtime workspace for HT32F523xx microcontrollers with probe-rs support.

## Overview

This workspace contains:
- `ht32-hal/` - Hardware Abstraction Layer (GPIO, RCC, Timer, UART)
- `embassy-ht32/` - Embassy async wrappers and trait implementations
- `bsp/` - Board Support Package (pin mappings for ESK32-30501)
- `examples/` - Ready-to-run examples

## Development Setup

### Prerequisites

Install required tools:
```bash
# Install Rust target for Cortex-M0+
rustup target add thumbv6m-none-eabi

# Install probe-rs for flashing and debugging
cargo install probe-rs --locked
cargo install cargo-embed --locked
```

### Building

```bash
# Build all workspace members
cargo build --release

# Build specific example
cargo build --release -p blink-embassy
```

### Running Examples

#### Blink Example
```bash
# Flash and run blink example
cargo run --release -p blink-embassy

# Or use probe-rs directly
cargo embed -p blink-embassy
```

#### Serial Echo Example
```bash
# Flash and run serial echo example
cargo run --release -p serial-echo
```

## Hardware Support

### Target MCU
- **HT32F52352** (Cortex-M0+, 48MHz max, 128KB Flash, 16KB SRAM)
- Uses community PAC: `ht32f523x2` from crates.io

### Development Board
- **ESK32-30501** starter kit (default BSP configuration)
- LEDs on PA4, PA5, PA6
- User button on PB12
- UART on PA2 (TX), PA3 (RX)

## Project Structure

```
embassy-ht32/
├── ht32-hal/           # Blocking HAL implementations
├── embassy-ht32/       # Async Embassy wrappers
├── bsp/                # Board support (ESK32-30501)
├── examples/
│   ├── blink-embassy/  # LED blink example
│   └── serial-echo/    # UART echo example
├── memory.x           # Linker script
├── Embed.toml         # probe-rs configuration
└── Cargo.toml         # Workspace configuration
```

## Features

### HAL (ht32-hal)
- ✅ GPIO (Input/Output with embedded-hal traits)
- ✅ RCC/Clock configuration
- ✅ Timer (GPTM0/GPTM1)
- ✅ UART (USART0/USART1) with embedded-hal-nb
- ✅ Time abstractions (Hertz, MicroSeconds, etc.)

### Embassy Support (embassy-ht32)
- ✅ Time driver based on GPTM0
- ✅ Async UART with interrupt handling
- ✅ Embassy executor integration

### Board Support (bsp)
- ✅ ESK32-30501 pin definitions
- ✅ LED abstractions
- ✅ UART pin configuration

## Usage

### Basic GPIO
```rust
use ht32_bsp::Leds;
use embedded_hal::digital::OutputPin;

let mut leds = Leds::new();
leds.led1.set_high().unwrap();
```

### Embassy Async
```rust
use embassy_time::{Duration, Timer};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    embassy_ht32::init();
    
    loop {
        Timer::after(Duration::from_millis(1000)).await;
        // Your async code here
    }
}
```

### UART
```rust
use embassy_ht32::uart::Uart;
use ht32_hal::uart::Config;

let config = Config {
    baudrate: 115_200.hz(),
    ..Default::default()
};
let mut uart = Uart::new(dp.USART0, config, &clocks);

uart.write(b"Hello, World!").await.unwrap();
```

## License

Licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.