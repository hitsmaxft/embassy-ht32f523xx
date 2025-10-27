# RMK 60-Key Keyboard Example for HT32F523xx

## Current Status: ⚠️ NEEDS UPDATES

This example is currently **disabled** in the workspace due to structural changes in the codebase and incomplete USB driver implementation.

## Issues to Fix

### 1. GPIO API Changes
The example uses the old HAL GPIO API structure:
- `embassy_ht32f523xx::hal::gpio::GpioExt` - No longer exists
- `p.GPIOA.split()` - GPIO API has changed to use const generics
- Pin methods like `into_floating_input()` need updating

### 2. USB Driver Implementation
- The USB driver is not fully implemented yet
- RMK requires a working USB HID interface
- Embassy-USB driver trait compatibility issues need resolution

### 3. Dependencies
- Embassy crate version conflicts between RMK and workspace
- Some RMK KeyCode variants don't exist (LBracket, RBracket, Mute)

## Required Updates

### Phase 1: GPIO Migration
1. Update GPIO usage to new Pin<PORT, PIN, MODE> structure
2. Replace `.split()` calls with direct Pin constructors
3. Update pin configuration methods

### Phase 2: USB Driver Completion
1. Complete embassy-usb-driver trait implementation
2. Resolve version conflicts between embassy crates
3. Test USB HID functionality

### Phase 3: RMK Integration
1. Update imports and API usage
2. Fix KeyCode compatibility issues
3. Test matrix scanning and key mapping

## Usage (After Fixes)

Once updated, this example will provide:
- Full 60-key mechanical keyboard firmware
- Matrix scanning with debouncing
- USB HID keyboard functionality
- Vial configuration support
- Real-time key remapping

## Contributing

To work on this example:
1. Uncomment the line in the main workspace Cargo.toml
2. Fix the GPIO API usage to match the new structure
3. Ensure USB driver is fully implemented
4. Test with actual hardware

## Hardware Requirements

- HT32F523xx development board
- 60-key keyboard matrix (5 rows × 14 columns)
- Proper pull-up resistors on matrix lines