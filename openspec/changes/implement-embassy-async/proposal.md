# Implement Complete Embassy Async Support for HT32F523xx

## Why
Embassy-ht32 currently has incomplete async functionality - while basic HAL components exist, the core async runtime integration is broken. Time drivers use custom alarm handling instead of standard Embassy patterns, executor wake mechanisms are missing, and async peripheral traits are either missing or inefficient (polling-based instead of interrupt-driven). This prevents users from leveraging Embassy's true async capabilities for responsive, low-power embedded applications.

## What Changes
- Enable `arch-cortex-m` feature in embassy-executor for PendSV-based task wake mechanism
- Replace custom time driver alarm handling with standard `embassy-time::alarm::on_interrupt()` integration
- Implement interrupt-driven async peripherals (GPIO, UART) using AtomicWaker patterns
- Add complete `embedded_hal_async` trait implementations for all supported peripherals
- Verify critical section implementation follows Embassy standards
- Create comprehensive test examples validating each async component
- Update all examples to use proper async patterns instead of blocking workarounds

## Impact
- **Affected specs**: async-executor, time-driver, peripherals, critical-section
- **Affected code**: `src/time_driver.rs`, `src/interrupt.rs`, `src/gpio.rs`, `src/uart.rs`, `Cargo.toml`
- **Breaking changes**: All async examples will need updating, custom alarm handling removed, correct executor features required
- **Performance gains**: True interrupt-driven async operations, reduced CPU usage, better power efficiency
- **Developer experience**: Full Embassy async/await support, standard embedded-hal-async compatibility