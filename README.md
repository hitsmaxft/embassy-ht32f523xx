# Embassy HT32F523xx

**Embassy async runtime and HAL for HT32F523xx microcontrollers**

An in-development Embassy implementation for selected HT32F523xx peripherals,
with async GPIO/UART/timer support and a USB Full-Speed device driver.

## 🚀 Project Status

> **Version**: 0.2.0
> **Status**: 🟡 **Active Development**
> **Hardware Testing**: HT32F52352 on ESK32-30501; see the support matrix below

## 📋 Quick Overview

This is a **unified crate** providing Embassy async runtime and hardware abstraction for HT32F523xx microcontrollers:

- **Embassy-native**: Built from ground up for Embassy async runtime
- **Hardware-validated core**: clocks, GPIO/EXTI, Embassy time, BFTM,
  USART0 TX, Flash, USB CDC and vendor Raw HID on HT32F52352
- **USB Ready**: Full-Speed device mode with CDC-ACM and HID examples
- **RMK integration example**: pinned RMK 0.9 fixed-keymap target links within
  the HT32F52352 memory budget; keyboard semantics remain an upper-layer concern
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
cargo build --release -p blink
```

### Running Examples

#### Blink Example
```bash
# Flash and run blink example
cargo run --release -p blink

# Or use probe-rs directly
cargo embed -p blink
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
- Uses the `ht32f523x2` PAC from the Git revision recorded in `Cargo.lock`

### Development Boards
- **ESK32-30501** starter kit (default BSP configuration)
- BSP pin mappings: active-low LEDs (PC14/PC15), WAKEUP header signal
  (PB12), USART0 module connector (PA2/PA3)
- Compatible with any HT32F523xx development board

### Peripheral Support Matrix

| Peripheral | Status | Features | Hardware |
|------------|---------|----------|----------|
| **GPIO / EXTI** | 🟡 Functional, signal QA pending | Input/output, pull-up/down, drive selection and interrupt-driven async edge waits are board-tested; final edge timing/EXTI latency capture is pending | GPIOA-D, EXTI0-15; only package-bonded pins are usable |
| **USART** | 🟡 Partial | Blocking/async TX hardware-tested; RX implementation is present but board-level RX validation requires changing the Starter Kit jumper | USART0/1 |
| **Timer** | 🟡 Partial | GPTM0 Embassy 1 MHz time base; BFTM0/1 async one-shot delays | PWM is not implemented; GPTM/SCTM/MCTM output compare remains planned |
| **Flash** | 🟡 Partial | Async 512-byte erase and 32-bit program hardware-tested | Synchronous `NorFlash::erase/write` are not implemented |
| **USB** | ✅ Hardware-tested | Full-Speed device, CDC-ACM echo, 64-byte vendor Raw HID IN/OUT | USB FS (0x400a_8000) |
| **Clock** | ✅ Hardware-tested | 8 MHz HSI/HSE, PLL to 48 MHz, AHB and per-peripheral APB prescalers | CKCU (0x4008_8000) |
| **I2C** | ❌ Planned | Master/slave, async traits | I2C0/1 (0x4004_8000/9000) |
| **SPI** | ❌ Planned | Master/slave, configurable modes | SPI0/1 (0x4000_4000/4004_4000) |
| **ADC** | ❌ Planned | 12 external channels on the device; no HAL driver yet | ADC (0x4001_0000) |
| **DMA** | ❌ Planned | 6-channel PDMA integration | PDMA (0x4009_0000) |
| **RTC / WDT / I2S / SCI / EBI / CRC** | ❌ Planned | Present in the MCU, not exposed by this HAL | Device-dependent |

### External-signal acceptance

Host-visible USB tests and register/counter checks do not replace measurement
of signals at the pins. Before a production-oriented release, run one final
logic-analyzer acceptance pass using the public HAL APIs and real board pins:

| Capability | Analyzer acceptance | Current gate |
|------------|---------------------|--------------|
| **GPIO / EXTI** | Capture generated edges and the input-to-response marker; verify level, polarity, period/duty, re-arm behavior and interrupt latency/jitter | Required for the existing implementation |
| **USART** | Decode TX/RX at each supported framing configuration; verify baud error, 7/8/9-bit handling, parity, stop bits, long transfers, loopback and error reporting | TX waveform and physical RX/loopback remain required |
| **I2C** | Verify open-drain behavior, START/repeated START/STOP, address/data ACK/NACK, 100/400 kHz timing, clock stretching and bus recovery | Run after an I2C HAL exists; currently only planned |
| **SPI** | Verify modes 0-3, SCK rate, MOSI/MISO bit order, chip-select setup/hold, full-duplex loopback and boundary lengths | Run after an SPI HAL exists; currently only planned |
| **PWM** | Verify 0/1/25/50/75/99/100% duty, frequency error, polarity, enable/disable and glitch-free period/duty updates | Run after a public PWM HAL exists; currently only planned |

A logic analyzer is sufficient for digital protocol and timing assertions, but
not for GPIO drive-current compliance, rise/fall time under a specified load,
analog ADC accuracy, or power integrity. Those require an oscilloscope and/or
DMM with a defined electrical load. Flash erase/program verification is done by
readback, while USB needs host/protocol tests rather than GPIO-channel capture.
For later peripherals, I2S also needs protocol/timing capture, WDT reset timing
can use a GPIO marker plus reset observation, and PDMA must be stressed through
the peripheral it serves. RTC/LSE accuracy is better measured with a frequency
counter or a long-window reference than with a short logic-analyzer capture.

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
│   ├── timer.rs            # BFTM one-shot timers
│   ├── uart.rs             # USART with nb traits and inherent async methods
│   ├── usb.rs              # USB device driver
│   ├── flash.rs            # Asynchronous Flash erase/program
│   ├── exti.rs             # External interrupts
│   ├── interrupt.rs        # Interrupt handling
│   └── fmt.rs              # Formatting utilities
├── bsp/                     # Board Support Package
│   └── src/esk32_30501.rs  # ESK32-30501 development board
├── examples/                # Ready-to-run examples
│   ├── blink/              # LED blink (Embassy async)
│   ├── serial-echo/        # UART echo (Embassy async)
│   ├── usb-hid-keyboard/   # USB HID keyboard
│   ├── usb-raw-hid-speed-test/ # Vendor Raw HID throughput test
│   └── ht32-rmk-60key/     # Lean, pinned RMK integration example
├── IMPLEMENTATION_PROGRESS.md # Capability and acceptance roadmap
└── VALIDATION_REPORT.md    # Evidence-scoped hardware validation
```

## 🏗️ Architecture

### Unified Design Benefits
- **Single Crate**: No more ht32-hal vs embassy-ht32 split
- **Embassy-First**: Built for async/await from the ground up
- **Feature Flags**: Select chip variant and peripherals at compile time
- **Zero-Cost**: Embassy async with no runtime overhead
- **Evidence scoped**: hardware-tested paths are distinguished from planned peripherals

## 💻 Usage Examples

See `examples/ht32-rmk-60key/` for the fixed-keymap RMK integration and its
automated Flash/static-RAM budget gate. It demonstrates framework compatibility;
it does not make RMK matrix or complete HID semantics part of HAL acceptance.

## 📚 Documentation & Resources

### Project Documentation
- 📊 [**Implementation Progress**](./IMPLEMENTATION_PROGRESS.md) - Capability and analyzer acceptance roadmap
- 🔍 [**Hardware Validation Report**](./VALIDATION_REPORT.md) - Evidence-scoped source, board and measurement status
- 📋 [**OpenSpec changes**](./openspec/changes/) - Approved and proposed implementation work

### External Resources
- 📖 [**HT32F523xx Datasheet**](https://www.holtek.com/productdetail/-/vg/HT32F52342_52352) - Official hardware documentation
- 🚀 [**Embassy Framework**](https://embassy.dev/) - Async runtime documentation
- 🦀 [**Embedded Rust Book**](https://doc.rust-lang.org/stable/embedded-book/) - Rust embedded development guide
- 🎛️ [**RMK Keyboard Firmware**](https://github.com/HaoboGu/rmk) - Advanced keyboard features

## 🤝 Contributing

### Current Priorities
1. **Close existing signal-level gates** - GPIO/EXTI and USART analyzer tests
2. **I2C Driver** (`src/i2c.rs`) - High impact peripheral
3. **SPI Driver** (`src/spi.rs`) - Critical for displays/sensors
4. **PWM Driver** - GPTM/SCTM/MCTM output compare and Embassy-facing API
5. **ADC / PDMA Drivers** - Analog input and efficient transfers

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
