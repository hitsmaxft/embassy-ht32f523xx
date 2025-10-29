# RMK HT32F523xx 60-Key Keyboard Example

This example demonstrates how to use the Embassy HT32F523xx HAL with RMK (Rust Mechanical Keyboard) firmware for a 60% keyboard layout.

## ⚠️ Memory Requirements Notice

**Important**: RMK is a feature-rich mechanical keyboard firmware that requires significant RAM and Flash memory. The HT32F523xx series microcontrollers have limited resources:

- **HT32F52342**: 64KB Flash, 8KB RAM
- **HT32F52352**: 128KB Flash, 16KB RAM

Due to the complexity of RMK with all its features (USB HID, matrix scanning, key processing, storage, etc.), this example **currently exceeds the available RAM** on both chips.

## Status

- ✅ **Compilation**: All GPIO, embedded-hal, and Embassy async traits compile successfully
- ✅ **GPIO Matrix**: AnyPin type and degrade() methods work correctly
- ✅ **Embassy Integration**: Async matrix scanning framework is ready
- ❌ **Linking**: Fails due to insufficient RAM for the complete RMK stack

## Alternatives

For HT32F523xx users interested in keyboard firmware:

### 1. Basic Keyboard Implementation
Use the simpler `usb-hid-keyboard` example as a starting point and implement basic matrix scanning without the full RMK framework.

### 2. Reduced Feature RMK
Disable some RMK features to reduce memory usage:
```toml
rmk = { git = "https://github.com/haobogu/rmk", features = ["async_matrix"], default-features = false }
```

### 3. More Powerful MCU
Consider using a microcontroller with more RAM (32KB+) such as:
- STM32F4xx series
- RP2040 (264KB RAM)
- ESP32-S3

## Code Structure

The example demonstrates proper integration patterns:

- **GPIO Configuration**: Using AnyPin for matrix pin arrays
- **Embassy Async**: Proper async matrix scanning setup
- **USB Integration**: Embassy USB driver configuration
- **Flash Storage**: RMK configuration storage

## Building

```bash
# Note: This will fail at linking stage due to memory constraints
cargo build -p rmk-ht32-60key
```

## Hardware Compatibility

This example is configured for:
- **MCU**: HT32F52352 (128KB Flash, 16KB RAM)
- **Layout**: 60% keyboard (5 rows × 14 columns)
- **Matrix**: GPIO-based scanning

## Future Work

The Embassy HT32F523xx HAL provides all the necessary building blocks:
- ✅ GPIO with embedded-hal traits
- ✅ USB HID device support
- ✅ Embassy async runtime
- ✅ Flash storage capabilities

For a working keyboard implementation on HT32F523xx, consider using the basic USB HID keyboard example and building up features incrementally.