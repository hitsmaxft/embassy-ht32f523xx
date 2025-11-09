## Why
The embassy-ht32 HAL currently lacks USB support, which is essential for the primary mechanical keyboard firmware use case. Adding USB driver implementation will enable complete HID keyboard functionality and unlock embassy-usb ecosystem integration.

## What Changes
- Implement complete embassy-usb-driver traits for HT32F52352 USB peripheral
- Add USB clock configuration (48MHz) and GPIO alternate function support
- Create USB interrupt handler with embassy task waking mechanism
- Implement endpoint buffer management using 1024-byte EP_SRAM
- Add USB enumeration validation using defmt logging
- Enable embassy-usb-serial example as test case

**BREAKING**: None - this adds entirely new capability

## Impact
- Affected specs: New `usb-driver` capability
- Affected code: `src/usb.rs`, `src/rcc.rs`, `src/gpio.rs`, examples/
- Dependencies: embassy-usb v0.5.0, embassy-usb-driver v0.2.0 (already configured)
- Testing: Requires probe-run for RTT logging validation