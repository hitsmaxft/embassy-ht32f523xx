<!-- OPENSPEC:START -->
# OpenSpec Instructions

These instructions are for AI assistants working in this project.

Always open `@/openspec/AGENTS.md` when the request:
- Mentions planning or proposals (words like proposal, spec, change, plan)
- Introduces new capabilities, breaking changes, architecture shifts, or big performance/security work
- Sounds ambiguous and you need the authoritative spec before coding

Use `@/openspec/AGENTS.md` to learn:
- How to create and apply change proposals
- Spec format and conventions
- Project structure and guidelines

Keep this managed block so 'openspec update' can refresh the instructions.

<!-- OPENSPEC:END -->

# embassy-ht32f523xx rust lib

Embassy HAL implementation for HT32F523xx microcontrollers

## 🧩 Project Structure (Current Status)

```
embassy-ht32/
├── Cargo.toml
├── build.rs
├── memory_ht32f52342.x
├── memory_ht32f52352.x
├── src/
│   ├── lib.rs              # Library entry point
│   ├── chip/               # Chip-specific definitions
│   │   ├── ht32f52342.rs
│   │   ├── ht32f52352.rs
│   │   └── mod.rs
│   ├── gpio.rs             # GPIO driver
│   ├── rcc.rs              # Clock and reset control
│   ├── time.rs             # Time unit definitions
│   ├── time_driver.rs      # Embassy time driver
│   ├── timer.rs            # Timer and PWM driver
│   ├── uart.rs             # UART driver
│   ├── usb.rs              # USB driver
│   ├── flash.rs            # Flash memory driver
│   ├── exti.rs             # External interrupts
│   ├── interrupt.rs        # Interrupt handling
│   └── fmt.rs              # Formatting utilities
└── bsp/                    # Board Support Package
    ├── Cargo.toml
    ├── src/lib.rs
    └── src/esk32_30501.rs
```

## 📊 Implementation Status

### ✅ Fully Implemented
| Module         | Status | Description                                      |
| -------------- | ------ | ------------------------------------------------ |
| `time.rs`      | ✅     | Complete time unit definitions (Hertz, Microseconds) |
| `gpio.rs`      | ✅     | Complete GPIO implementation with multiple modes |
| `flash.rs`     | ✅     | Complete NorFlash trait implementation           |
| `rcc.rs`       | ✅     | Clock and reset configuration                    |
| `timer.rs`     | ✅     | Basic timer and PWM implementation               |

### ⚠️ Partially Implemented
| Module             | Status | Description                                      |
| ------------------ | ------ | ------------------------------------------------ |
| `usb.rs`           | ⚠️     | Basic USB driver, needs more hardware-specific config |
| `uart.rs`          | ⚠️     | Basic UART support, Embassy async traits pending |
| `interrupt.rs`     | ⚠️     | Basic interrupt handling structure, handlers pending |
| `exti.rs`          | ⚠️     | Simplified external interrupt implementation     |
| `time_driver.rs`   | ⚠️     | Basic Embassy time driver implementation         |

### ❌ Missing Features
- I2C driver
- I2S driver
- ADC driver
- DMA support
- Complete async trait implementations
- Advanced PWM features


## MCU Specifications Comparison

### HT32F52342 vs HT32F52352 Differences

| Specification  | HT32F52342   | HT32F52352   | Purpose                 |
|--------------- | ------------ | ------------ | ----------------------- |
| Flash          | 64KB         | 128KB        | Program storage         |
| RAM            | 8KB          | 16KB         | Runtime memory          |
| Package        | LQFP48       | LQFP64       | Pin count and layout    |
| Keyboard Ver.  | C15 revision | C18 revision | Different hardware revs |

### Current Project Configuration

**ht32-rmk-60key uses HT32F52352 (C18 revision):**
- 16KB RAM - Sufficient for complete RMK functionality
- 128KB Flash - Adequate program storage space
- LQFP64 package - More GPIO pins available

### USB Controller Specifications

**HT32F52352 USB Device Controller:**
- USB 2.0 full-speed (12 Mbps) compatible
- 1 control endpoint (EP0)
- 3 single-buffered endpoints (bulk/interrupt transfers)
- 4 double-buffered endpoints (bulk/interrupt/isochronous transfers)
- 1024 bytes EP_SRAM endpoint data buffer
- Total: 8 endpoints (1 control + 7 configurable)

## Dependency Library Information

### ht32f523x2 Peripheral Access API

* ht32f523x2 rust Peripheral Access API placed under ./deps/ht32f523x2/
* SVD file placed under ./deps/ht32f523x2/HT32F52342_52.svd
* **Note**: ht32-rmk-60key project uses HT32F52352 MCU (C18 revision) - 16KB RAM, 128KB Flash
- use github mcp to search or clone repos you need for researching embassy and ht32 codes, code should place under ./deps/
- this project's github repo is  git@github.com:hitsmaxft/embassy-ht32.git
- you must add a build.rs like examples/blink for all project
- use cargo run --release -p {example_name} to flash and run , remember, it make stuck on timer, you need ensure make it timeout if no expected result make hang your command
- you should never remove embassy-hal in example or test , your MISSION is complete the embassy-hal to run the example , never never remove embassy-hal since they are broken
- YOU MUST use `cargo run --release ...` to run target image, there is no need to use debug build
- YOU MUST use `cargo run --release -p {sub project} --bin {binary name if needed}` to run target image, there is no need to use debug build