## Why

The HT32 USB transport and standalone HID examples work, but the keyboard target
is pinned to an older Embassy/RMK combination that no longer builds reliably
with the active toolchain and consumes nearly all HT32F52352 RAM when Vial and
storage are enabled. The project needs one aligned Embassy generation and a
lean, reproducible keyboard target before end-to-end HID work can continue.

## What Changes

- Upgrade the workspace Embassy USB/time/sync ecosystem to the versions required
  by a pinned RMK 0.9 revision, updating HT32 driver trait implementations and
  examples/tests for changed APIs.
- Convert `rmk-ht32-60key` to a fixed-keymap integration example without Vial
  or persistent storage.
- Keep asynchronous matrix support using the upstream fixed-array wait
  implementation rather than the incompatible older `select_slice` path.
- Add release link and memory-budget gates for the 16 KiB RAM target.
- Keep HAL acceptance scoped to the HT32 USB driver contract and standalone
  transport tests. Matrix semantics and complete keyboard HID behavior remain
  RMK/application responsibilities.

## Impact

- Affected specs: `hid-keyboard-firmware`
- Affected code: root dependency graph and lockfile, `src/usb.rs`, Embassy-facing
  HAL modules, workspace examples/tests, and `examples/ht32-rmk-60key/`
- Compatibility: dependency/API migration may require mechanical call-site
  changes; no stable 0.1 public API compatibility is promised.
- Explicitly out of scope: Vial/VIA, Flash-backed keymap writes, BLE, Anne Pro 2
  external LED MCU protocol, PWM, I2C and SPI drivers.
