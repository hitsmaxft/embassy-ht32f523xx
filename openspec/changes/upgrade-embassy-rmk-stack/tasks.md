## 1. Dependency migration

- [x] 1.1 Pin RMK and RMK Types to one reviewed current revision.
- [x] 1.2 Align Embassy USB, Sync, Time and related HID dependencies workspace-wide.
- [x] 1.3 Adapt the HT32 USB driver and other Embassy-facing APIs.
- [x] 1.4 Regenerate and audit `Cargo.lock` for duplicate/incompatible Embassy generations.

## 2. Lean keyboard target

- [x] 2.1 Remove Vial, storage and generated layout-blob dependencies from `rmk-ht32-60key`.
- [x] 2.2 Initialize a fixed in-memory keymap and asynchronous 5x14 matrix.
- [x] 2.3 Configure a stable USB identity for the RMK integration example.

## 3. Build and resource gates

- [x] 3.1 Build and link the release RMK target for `thumbv6m-none-eabi`.
- [x] 3.2 Record Flash and static RAM sections and retain at least 2 KiB stack headroom.
- [x] 3.3 Build existing blink, USART, CDC, HID and Raw HID examples.
- [x] 3.4 Run formatting, targeted checks and the applicable workspace matrix.

## 4. HAL hardware acceptance

- [ ] 4.1 Re-run standalone USB CDC and bidirectional Raw HID transport tests after migration.
- [ ] 4.2 Verify reset, reconnect and suspend/resume against the declared USB capability.
- [ ] 4.3 Keep matrix/HID semantics and composite-traffic acceptance in downstream RMK firmware tests.
