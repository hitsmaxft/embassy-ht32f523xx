# RMK HT32F52352 integration example

This target checks that the HT32 Embassy USB driver can be integrated with a
reproducibly pinned RMK 0.9 stack on the 128 KiB Flash / 16 KiB RAM
HT32F52352. It is deliberately a framework integration example, not the HAL's
acceptance test for complete keyboard behavior.

## Scope

- fixed 5x14, three-layer in-memory keymap;
- asynchronous GPIO matrix construction;
- RMK USB transport;
- no Vial/VIA, persistent storage, logging, BLE or Raw HID composite class;
- stable USB strings: `Embassy HT32`, `HT32 RMK Integration`, and
  `HT32-RMK-60K-0001`.

Matrix pin selection in `src/main.rs` is illustrative and has not been matched
to a production keyboard PCB. Debounce, rollover, keymap semantics,
Keyboard/Consumer/System reports and host LED behavior are RMK/application
responsibilities rather than HT32 peripheral capabilities.

## Build and memory gate

From the workspace root:

```bash
cargo build --release -p rmk-ht32-60key
examples/ht32-rmk-60key/check-size.sh
```

The checked build on 2026-08-22 uses 71,680 bytes of loadable Flash and 8,328
bytes of static RAM (`.data + .bss + .uninit`), leaving 8,056 bytes for stack
and runtime use. The script fails if static RAM leaves less than 2 KiB or the
image exceeds the 127.5 KiB application Flash region.

## Hardware status and remote wakeup

Basic RMK execution and its keyboard-report path were physically exercised on
2026-08-22 with the ESK32-30501 and Holtek CMSIS-DAP `02000D2E`. The test image
enumerated at USB full speed as VID:PID `4c4b:4643`, product
`HT32 RMK Integration`, serial `HT32-RMK-60K-0001`, and exposed the expected
Keyboard, Mouse, Consumer and System Control collections.

With `swd-key-inject`, the debugger injected row 2 / column 1 through RMK's
normal event channel. The fixed keymap converted it to keyboard usage `0x04`
(`A`). For press, endpoint 1 contained
`00 00 04 00 00 00 00 00`; for release it contained eight zero bytes. In both
directions the mailbox acknowledged the command and the USB controller raised
`IDTX`, after which Embassy completed the endpoint write. This proves the path
from debugger mailbox through RMK event/keymap processing and the HT32 USB IN
endpoint. It does not claim that a physical switch matrix was exercised or
that an OS application displayed the character.

The standalone CDC and vendor Raw HID examples remain the HT32 USB transport
acceptance tests. Complete keyboard semantics remain an RMK/application test.

The pinned RMK revision currently advertises USB remote wakeup internally,
while the HT32 driver's `Bus::remote_wakeup` still returns `Unsupported`.
Basic execution and key press/release are therefore accepted, but
suspend/remote-wakeup acceptance remains blocked until the resume signal is
implemented and physically verified, or RMK exposes a switch that disables the
descriptor capability.

## SWD key-event injection

For controlled bring-up without a wired matrix, build with
`--features swd-key-inject`. The resulting ELF exports two volatile test
mailboxes:

- `RMK_SWD_KEY_COMMAND`: bit 31 valid, bit 30 pressed, row in bits 15:8 and
  column in bits 7:0;
- `RMK_SWD_KEY_ACK`: the consumed command with bit 29 set.

For example, row 2 / column 1 is `A` in the fixed keymap. Write `0xc0000201`
for press and `0x80000201` for release. This feature feeds RMK's public event
path and is excluded from normal builds. Successful acknowledgements are
`0xe0000201` and `0xa0000201`, respectively.

Two diagnostic command bits are reserved for bring-up isolation: bit 28 forces
RMK's USB routing state to `Configured`, while bit 27 sends a direct `A` report
through RMK's USB report queue. Neither path is used by normal firmware.
