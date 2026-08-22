## Context

The workspace currently uses Embassy USB 0.5 and RMK 0.7.8. The latest reviewed
RMK revision uses Embassy USB 0.6, Embassy Sync 0.8 and newer HID descriptors.
Pinning old RMK while enabling `async_matrix` fails in upstream `select_slice`;
pinning current RMK without upgrading Embassy produces dependency conflicts.

The HT32F52352 has 16 KiB RAM. A prior Vial/storage-enabled ELF used 16,376
bytes of `.data + .bss`, leaving no viable stack. Persistence is not required
for the first fixed-keymap HID firmware.

## Goals / Non-Goals

- Goals:
  - one reproducible Embassy/RMK dependency generation;
  - a linked fixed-keymap 5x14 RMK integration image with usable RAM headroom;
  - preserve existing HT32 USB CDC and Raw HID behavior through the migration.
- Non-Goals:
  - Vial/VIA or mutable keymap persistence;
  - acceptance of RMK matrix, keymap, rollover, Consumer/System or LED Output
    semantics as HT32 HAL features;
  - composite keyboard plus Raw HID scheduling, which belongs to the selected
    keyboard framework/application;
  - BLE or external LED-controller integration;
  - bootloader/IAP implementation in this change;
  - declaring physical keyboard acceptance from compilation alone.

## Decisions

- Pin RMK and RMK Types to the same exact Git revision. Cargo.lock alone is not
  the source-level reproducibility contract for these direct Git dependencies.
- Upgrade the workspace Embassy versions instead of carrying a local patch to
  old RMK. This keeps async matrix scanning on the maintained upstream path.
- Remove storage/Vial code generation and dependencies from the 60-key target.
  The HAL Flash driver remains independently available, but is not on the HID
  critical path.
- Require a release ELF budget report. Static RAM sections must leave at least
  2 KiB for stack before optional application-level hardware tests.
- Do not advertise USB remote wakeup unless the HT32 driver implements and
  physically validates resume signaling.

## Risks / Trade-offs

- Embassy API changes can affect every example and test. Mitigation: migrate the
  root graph once, build targeted firmware first, then run the workspace matrix.
- Removing storage prevents runtime remapping. Mitigation: fixed compile-time
  layers remain supported; configurability can return in a separate proposal.
- A linked image can still overflow at runtime. Mitigation: static headroom and
  a documented application-level stack-watermark gate before product use.
- Updating HID descriptors can alter host enumeration. Mitigation: capture and
  compare descriptors and host-visible reports before and after migration.

## Migration Plan

1. Pin RMK/RMK Types and align workspace Embassy/HID versions.
2. Adapt the HT32 HAL and examples/tests until targeted builds pass.
3. Link the no-storage/no-Vial RMK image and enforce memory headroom.
4. Validate existing USB CDC and standalone Raw HID regressions.
5. Leave RMK keyboard behavior and composite HID acceptance to a downstream
   keyboard-firmware validation plan.

Rollback is dependency-lock and call-site restoration; no persistent on-device
format changes are introduced because storage is excluded.
