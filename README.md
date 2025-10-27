# Embassy HT32F523xx

**Embassy async runtime and HAL for HT32F523xx microcontrollers**

A unified, production-ready Embassy implementation providing complete hardware abstraction for HT32F523xx MCUs with async/await support, USB capabilities, and mechanical keyboard integration.

## 🚀 Project Status

> **Version**: 0.1.0
> **Status**: 🟡 **Active Development** - Foundation Complete (45% implementation)
> **Hardware Testing**: ✅ **Ready**

## 📋 Quick Overview

This is a **unified crate** providing Embassy async runtime and hardware abstraction for HT32F523xx microcontrollers:

- **Embassy-native**: Built from ground up for Embassy async runtime
- **Hardware-validated**: All register usage verified against official SVD
- **USB Ready**: Full-speed USB device with HID keyboard support
- **RMK Compatible**: Ready for mechanical keyboard firmware integration
- **Probe-rs Ready**: Complete development environment with examples

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

## 🔧 Hardware Support

### Supported MCUs
- **HT32F52342** - Cortex-M0+, 48MHz, 64KB Flash, 8KB SRAM
- **HT32F52352** - Cortex-M0+, 48MHz, 128KB Flash, 16KB SRAM (default)
- Uses official PAC: `ht32f523x2` v0.5.0 from crates.io

### Development Boards
- **ESK32-30501** starter kit (default BSP configuration)
- Pin mappings: LEDs (PA4-PA6), Button (PB12), UART (PA2/PA3)
- Compatible with any HT32F523xx development board

### Peripheral Support Matrix

| Peripheral | Status | Features | Hardware |
|------------|---------|----------|----------|
| **GPIO** | ✅ Complete | All 4 ports, input/output modes, pull-up/down | PA0-PA15, PB0-PB15, PC0-PC15, PD0-PD15 |
| **USART** | ✅ Complete | Async traits, configurable baud/parity | USART0 (0x4000_0000), USART1 (0x4004_0000) |
| **Timer** | ✅ Complete | PWM, delays, Embassy time driver | GPTM0/1, SCTM0/1, BFTM0/1 |
| **Flash** | ✅ Complete | NorFlash trait, async operations | 64KB/128KB program memory |
| **USB** | ✅ Basic | HID keyboard, device mode | USB FS (0x400a_8000) |
| **Clock** | ✅ Complete | HSI/HSE/PLL, prescalers | CKCU (0x4008_8000) |
| **I2C** | ❌ Planned | Master/slave, async traits | I2C0/1 (0x4004_8000/9000) |
| **SPI** | ❌ Planned | Master/slave, configurable modes | SPI0/1 (0x4000_4000/4004_4000) |
| **ADC** | ❌ Planned | 8-channel, continuous conversion | ADC (0x4001_0000) |
| **DMA** | ❌ Planned | 6-channel PDMA integration | PDMA (0x4009_0000) |

## 📁 Project Structure (Unified)

```
embassy-ht32f523xx/           # Root crate (was: ht32-hal + embassy-ht32)
├── Cargo.toml               # Main crate configuration
├── build.rs                 # Chip selection build script
├── memory_ht32f52342.x      # Linker script for HT32F52342
├── memory_ht32f52352.x      # Linker script for HT32F52352
├── src/                     # Unified HAL + Embassy implementation
│   ├── lib.rs              # Crate entry point
│   ├── chip/               # Chip-specific definitions
│   │   ├── ht32f52342.rs   # HT32F52342 configuration
│   │   ├── ht32f52352.rs   # HT32F52352 configuration
│   │   └── mod.rs          # Chip selection logic
│   ├── gpio.rs             # GPIO with Embassy digital traits
│   ├── rcc.rs              # Clock management
│   ├── time.rs             # Time units (Hertz, Microseconds)
│   ├── time_driver.rs      # Embassy time driver
│   ├── timer.rs            # Timer/PWM functionality
│   ├── uart.rs             # UART with Embassy async traits
│   ├── usb.rs              # USB device driver
│   ├── flash.rs            # Flash memory (NorFlash trait)
│   ├── exti.rs             # External interrupts
│   ├── interrupt.rs        # Interrupt handling
│   └── fmt.rs              # Formatting utilities
├── bsp/                     # Board Support Package
│   └── src/esk32_30501.rs  # ESK32-30501 development board
├── examples/                # Ready-to-run examples
│   ├── blink-embassy/      # LED blink (Embassy async)
│   ├── serial-echo/        # UART echo (Embassy async)
│   ├── usb-hid-keyboard/   # USB HID keyboard
│   └── rmk-keyboard-ap2/   # RMK mechanical keyboard (WIP)
└── docs/                   # Comprehensive documentation
    ├── IMPLEMENTATION_PROGRESS.md
    ├── VALIDATION_REPORT.md
    └── todolist.md
```

## 🏗️ Architecture

### Unified Design Benefits
- **Single Crate**: No more ht32-hal vs embassy-ht32 split
- **Embassy-First**: Built for async/await from the ground up
- **Feature Flags**: Select chip variant and peripherals at compile time
- **Zero-Cost**: Embassy async with no runtime overhead
- **Hardware Validated**: All register usage verified against SVD

## 💻 Usage Examples

See `examples/rmk-keyboard-ap2/` for a complete 60% keyboard implementation.

## 📚 Documentation & Resources

### Project Documentation
- 📊 [**Implementation Progress**](./IMPLEMENTATION_PROGRESS.md) - Detailed 45% completion status
- 🔍 [**Hardware Validation Report**](./VALIDATION_REPORT.md) - SVD/PAC verification
- 📋 [**Detailed Todo List**](./todolist.md) - Development roadmap

### External Resources
- 📖 [**HT32F523xx Datasheet**](https://www.holtek.com/productdetail/-/vg/HT32F52342_52352) - Official hardware documentation
- 🚀 [**Embassy Framework**](https://embassy.dev/) - Async runtime documentation
- 🦀 [**Embedded Rust Book**](https://doc.rust-lang.org/stable/embedded-book/) - Rust embedded development guide
- 🎛️ [**RMK Keyboard Firmware**](https://github.com/HaoboGu/rmk) - Advanced keyboard features

## 🤝 Contributing

### Current Priorities
1. **I2C Driver** (`src/i2c.rs`) - High impact peripheral
2. **SPI Driver** (`src/spi.rs`) - Critical for displays/sensors
3. **ADC Driver** (`src/adc.rs`) - Analog input support
4. **Test Infrastructure** - Unit tests for existing modules
5. **Documentation** - API docs and tutorials

### Development Standards
- Embassy async patterns following existing implementations
- Hardware validation against PAC definitions
- Memory safety with documented unsafe usage
- Comprehensive error handling (no panics in production)

## 📄 License

Licensed under either of:
- **Apache License, Version 2.0** ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- **MIT License** ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
