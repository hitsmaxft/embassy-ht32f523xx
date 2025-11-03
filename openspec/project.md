# Project Context

## Purpose
This project provides a unified Embassy asynchronous runtime and Hardware Abstraction Layer (HAL) for HT32F523xx microcontrollers. The goal is to deliver enterprise-grade embedded firmware support for RMK mechanical keyboards and general embedded applications, offering complete peripheral drivers, async/await support, and production-ready hardware abstraction with comprehensive validation and testing.

### Key Objectives
- **Zero-cost async embedded development** for HT32F523xx microcontrollers using Embassy framework
- **Production-ready mechanical keyboard firmware** with RMK compatibility and advanced features
- **Comprehensive hardware abstraction** verified against official peripheral access crate (PAC)
- **High-performance timing systems** with sub-microsecond precision and hardware fault tolerance
- **USB device implementation** with HID keyboard and advanced peripheral support
- **Extensive testing and validation** suitable for commercial embedded products

## Tech Stack

### Core Technologies
- **Rust Edition 2024** - Modern embedded systems programming language
- **Cortex-M (Thumb v6)** - ARM Cortex-M0+ microcontroller architecture (thumbv6m-none-eabi)
- **Embassy Framework v0.9+** - Asynchronous embedded runtime with cooperative scheduling
- **Embedded HAL v1.0** - Standard Hardware Abstraction Layer traits for embedded systems

### Hardware Platform
- **Target MCU**: HT32F52342/52 (Cortex-M0+ @ 48MHz)
- **Peripheral Access**: ht32f523x2 v0.5 (SVD-verified register definitions)
- **Memory**: 64KB/128KB Flash, 8KB/16KB RAM
- **Architecture**: ARM Cortex-M0+ with 32 peripheral interrupts
- **Development Boards**: ESK32-30501 with LEDs (PA4-PA6), Button (PB12), UART (PA2/PA3)

### Development & Debugging
- **probe-rs** v0.24+ - Modern debugging and flash programming
- **cargo-embed** - Embedded development workflow automation
- **defmt** v0.3 - Efficient embedded logging framework
- **panic-probe v0.3** - Structured panic handling for embedded debugging

### Quality & Testing
- **critical-section v1.0** - Race-free shared resource access for embedded-safe programming
- **embassy-sync v0.7+** - Embeddable synchronization primitives (Mutex, Channel, etc.)
- **embebed-storage v0.3+** - Flash memory abstractions for persistent storage

### USB & Communications
- **embassy-usb v0.5+** - USB device stack for HID keyboards and custom devices
- **embassy-usb-driver v0.2+** - USB driver abstraction layer
- **usbd-hid v0.8+** - HID class implementation for keyboard/productivity devices

## Project Conventions

### Code Style

#### Naming Conventions
- **CamelCase** for all public types, traits, modules (`TimeDriver`, `OutputPin`)
- **snake_case** for functions, variables, and private members (`set_high()`, `get_clock()`)
- **SCREAMING_SNAKE_CASE** for constants and register offsets (`GPIOA_BASE`, `ENABLE_BIT`)
- **Abbreviate** only when standard (GPIO, USART, USB) - avoid custom abbreviations
- **Hardware register names** should match PAC exactly for consistency

#### Architecture Patterns
- **Feature-gate MCU variants** using Cargo features (`ht32f52342`, `ht32f52352`)
- **Peripheral initialization** follows Embassy patterns with `Peripheral::init()` pattern
- **Async-first design** - all I/O operations should be async when possible
- **Zero-allocation APIs** - heap allocations forbidden in production code

#### Documentation Standards
- **Module-level docs** must include hardware register addresses and example usage
- **Function docs** document timing characteristics, power consumption, and safety requirements
- **Safety sections** explain exactly when unsafe code is required and why
- **README updates** track implementation progress with visual progress bars

#### Memory Safety
- **Critical sections** used for all shared state access (`critical_section::with()`)
- **Volatile register access** via PAC with proper memory ordering annotations
- **Stack-based allocation** preferred over heap - minimize `Vec` and `Box` usage
- **Bounds checking** on all array/slice access - no unchecked indexing in production code

### Architecture Patterns

#### Embassy Integration
- **Async executors** use Embassy's work-stealing scheduler optimized for embedded
- **Peripheral drivers** follow Embassy patterns for initialization and power management
- **Interrupt handling** integrated with Embassy's interrupt management system
- **Time drivers** provide 64-bit timestamps with hardware overflow handling

#### Hardware Abstraction
- **Feature-flag chip selection** at compile time (single binary supports multiple variants)
- **Register access** through official PAC with SVD-verified bit definitions
- **Memory-mapped I/O** with proper volatile semantics and memory barriers
- **Clock/reset management** providing safe peripheral power management

#### Error Handling
- **Result-based APIs** return specific error types matching embedded-hal conventions
- **Graceful degradation** on resource exhaustion (return error vs panic)
- **Hardware fault detection** with automatic recovery where possible
- **Debugging information** exposed through defmt debug framework

#### Testing Architecture
- **Unit tests** for algorithmic components (interrupt overflow, timing calculations)
- **Hardware validation** against official HT32 documentation and SVD files
- **Integration tests** using real hardware with probe-rs debug tools
- **Real-time validation** with oscilloscope measurements for timing-critical paths

### Testing Strategy

#### Unit Testing
- **Algorithm validation** without hardware (overflow detection, frequency calculations)
- **Register bit manipulation** with simulated memory models
- **State machine testing** for complex peripheral configurations
- **Edge case coverage** including maximum/minimum values and boundary conditions

#### Hardware-in-the-Loop Testing
- **Performance measurement** using probe-rs to measure actual timing characteristics
- **Register verification** against official HT32F523xx technical documentation
- **Power consumption validation** using development board current measurement
- **Interrupt latency testing** with signal analysis for real-time performance

#### Integration Testing
- **End-to-end examples** demonstrating real-world usage patterns
- **USB compliance testing** with USB analyzer for HID device certification
- **Keyboard matrix testing** with full 60% keyboard implementation validation
- **Long-term stability** with 24+ hour continuous operation tests

### Git Workflow

#### Branch Strategy
- **`main`** - Production-ready code with stable APIs
- **`feature/[name]`** - New peripheral support or major features
- **`enhance/[name]`** - Performance improvements and optimizations
- **`fix/[name]`** - Bug fixes and targeted improvements
- **`time_driver`** (current) - Time driver enhancement branch

#### Commit Conventions
- **Emoji prefixes** for commit categories (🚀 feature, 🐛 bugfix, 📚 docs, ⚡ performance)
- **Conventional format** with optional scope: `category(scope): description`
- **Detailed bodies** for complex changes explaining motivation and implementation approach
- **Breaking changes** marked with `**BREAKING**` and migration guidance

#### Release Process
- **Version tagging** follows semver with MAJOR.MINOR.PATCH format
- **Feature freeze** ~1 week before major releases for validation
- **Hardware validation** required for all releases intended for production use
- **Documentation updates** must accompany all API changes

## Domain Context

### Embedded Development Domain
- **Bare metal programming** - no operating system, direct hardware register access
- **Resource constraints** - limited Flash/RAM (64KB/8KB minimum), no dynamic allocation
- **Real-time requirements** - deterministic timing for USB and keyboard applications
- **Power efficiency** important for battery-powered keyboard applications

### Microcontroller Architecture Context
- **Harvard architecture** - separate instruction and data memory spaces
- **Memory-mapped I/O** - peripheral registers accessed as memory locations
- **Interrupt-driven architecture** - external events trigger immediate CPU response
- **Clock domains** - different frequencies for CPU, peripherals, and USB

### Mechanical Keyboard Domain
- **Matrix scanning** - detecting key presses across rows/columns of switches
- **USB HID protocol** - standardized keyboard reports sent to host computer
- **Debouncing** - filtering mechanical contact bounce from switch closures
- **NKRO** (n-key rollover) - detecting unlimited simultaneous key presses
- **Key maps** - configuration defining physical key to logical key mappings

### USB Device Protocol Context
- **USB 2.0 Full-Speed** - 12 Mbps communication rate, 1ms frame intervals
- **HID class** - Human Interface Device specification for keyboards
- **Descriptor chains** - structured device identification and capabilities
- **Endpoint management** - separate channels for control, input, and output data

### HT32-Specific Constraints
- **Limited peripherals** - 2×USART, 2×I2C, 2×SPI vs STM32's extensive options
- **Fixed interrupt mappings** - predefined interrupt vectors vs configurable STM32
- **Clock restrictions** - fewer PLL options, specific prescaler limitations
- **USB limitations** - 4 endpoints vs STM32's extensive endpoint allocation

## Important Constraints

### Technical Constraints
- **No heap allocation** in production code - stack-only allocation required
- **32KHz RTC limitation** - no dedicated RTC peripheral, timer-based only
- **Flash write protection** - manufacturer constraints on bootloader areas
- **Single USB peripheral** - no USB OTG, device-only operation
- **Cortex-M0+ limitation** - no hardware division, limited instruction set

### Memory Constraints
- **Minimum RAM**: 8KB on HT32F52342 requires careful memory budgeting
- **Flash storage**: 64KB-128KB limits code size and include optimization needs
- **Stack usage**: Deep call stacks prohibited due to limited RAM
- **Static allocation**: Prefer stack allocation, minimize global static storage

### Timing Constraints
- **USB timing**: 1ms frame intervals require precise timing for HID operations
- **Debouncing intervals**: 5-20ms typical mechanical switch bounce periods
- **Matrix scanning**: 100-1000Hz scan rates for responsive keyboard feel
- **Interrupt latency**: <100μs required for real-time USB communication

### Validation Requirements
- **Hardware testing required** - all features must be validated on physical boards
- **SVD verification** - register definitions validated against official SVD files
- **probe-rs compatibility** - development and debugging must work with modern tools
- **Long-term stability** - 24+ hour continuous operation verification for releases

### Compliance & Certification (Future)
- **USB-IF certification** - fast enumeration, proper descriptor handling for HID
- **EMC compliance** - electromagnetic compatibility for commercial products
- **Safety standards** - appropriate for consumer electronics applications
- **Rust embedded guidelines** - follow Rust Embedded Working Group best practices

## External Dependencies

### Essential Dependencies
- **HT32F523x2 PAC** (v0.5.0) - Official peripheral access crate from Holtek vendor
- **Embassy Framework** (v0.9.0+) - Async runtime and peripheral driver foundation
- **probe-rs** (v0.24+) - Hardware debugging and programming toolchain
- **defmt** (v0.3) - Specialized embedded logging framework for resource efficiency

### Development Infrastructure
- **cargo-embed** - Cargo extension for embedded development workflow
- **ESK32-30501** - Official Holtek development board for HT32F523xx evaluation
- **nucleo-stlink** or **J-Link** - Hardware debug probes for development

### Peripheral Ecosystem
- **RMK** - Mechanical keyboard firmware framework (future integration target)
- **USB HID** - Host operating systems (Windows/Linux/macOS) for keyboard compatibility
- **keyberon** - Rust-based keyboard matrix scanning library (future integration)

### Build & Validation Tools
- **GitHub Actions** - CI/CD for automated building and formatting validation
- **cargo-audit** - Security vulnerability scanning for dependency validation
- **cargo-tarpaulin** - Code coverage measurement for quality assurance
- **cargo-fuzz** - Fuzzing capabilities for robustness validation

### Community & Standards
- **Rust Embedded Community** - Forums (matrix.org), GitHub discussions
- **Embassy Discord** - Real-time support and development discussions
- **Holtek Support** - Official technical documentation and vendor support
- **USB-IF Standards** - Official USB specification compliance requirements