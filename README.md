# HT32 Embassy Project

A complete Embassy async runtime workspace for HT32F523xx microcontrollers with probe-rs support.

It is intended to provide a foundation for async programming on HT32F523xx MCUs using the Embassy framework.

## WARNING

This is Vibe Coding project Currently

!!! Currently, it's a draft project and is not yet ready for production use. 


## Overview


This workspace contains:
- `ht32-hal/` - Hardware Abstraction Layer (GPIO, RCC, Timer, UART)
- `embassy-ht32/` - Embassy async wrappers and USB driver implementation
- `bsp/` - Board Support Package (pin mappings for ESK32-30501)
- `examples/` - Ready-to-run examples including USB HID keyboard

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

#### USB HID Keyboard Example
```bash
# Flash and run USB HID keyboard example
cargo run --release -p usb-hid-keyboard
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
│   ├── blink-embassy/     # LED blink example
│   ├── serial-echo/       # UART echo example
│   └── usb-hid-keyboard/  # USB HID keyboard example
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
- ✅ Async UART with interrupt handling
- ✅ Embassy executor integration
- ✅ USB Full-Speed driver for embassy-usb
- ✅ USB HID class support for keyboards

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

### USB HID Keyboard
```rust
use embassy_ht32::usb::Driver;
use embassy_usb::{Builder, Config};
use embassy_usb::class::hid::{HidReaderWriter, State};
use usbd_hid::descriptor::KeyboardReport;

// Create USB driver
let driver = Driver::new();

// Configure USB device
let mut config = Config::new(0xc0de, 0xcafe);
config.manufacturer = Some("Embassy");
config.product = Some("HT32 HID Keyboard");

// Create HID class
let mut state = State::new();
let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut state, hid_config);

// Send keypress
let report = KeyboardReport {
    modifier: 0,
    reserved: 0,
    leds: 0,
    keycodes: [0x04, 0, 0, 0, 0, 0], // 'A' key
};
hid.write_serialize(&report).await.unwrap();
```

## RMK Integration

This USB driver is designed to be compatible with [RMK](https://github.com/HaoboGu/rmk), a Rust mechanical keyboard firmware. The embassy-usb implementation provides the foundation for advanced keyboard features:

- **USB HID**: Full keyboard, mouse, and consumer device support
- **Key Matrix**: GPIO-based key scanning with embassy async
- **Advanced Features**: Ready for RGB, rotary encoder, and wireless modules
- **Low Latency**: Async design optimizes response time

To use with RMK, simply replace the USB driver in your RMK configuration with the HT32 embassy-usb driver implementation.

## License

Licensed under either of

- Apache License, Version 2.0
- MIT license

at your option.
