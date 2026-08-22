## ADDED Requirements

### Requirement: Reproducible Embassy and RMK dependency generation
The workspace SHALL use one compatible Embassy generation and SHALL pin RMK and
RMK Types to the same exact reviewed revision.

#### Scenario: Clean release build
- **WHEN** the repository is built from a clean checkout for `thumbv6m-none-eabi`
- **THEN** the fixed-keymap keyboard target links without dependency or API errors
- **AND** the resolved Embassy/RMK revisions match the reviewed lockfile

### Requirement: Lean fixed-keymap keyboard firmware
The HT32F52352 keyboard target SHALL provide a compile-time keymap without Vial,
VIA or persistent keymap storage and SHALL preserve usable stack headroom.

#### Scenario: Resource-constrained link
- **WHEN** the release keyboard image is linked for the 16 KiB RAM target
- **THEN** `.data`, `.bss` and reserved uninitialized static memory leave at least 2 KiB for stack
- **AND** no Vial or storage task is present in the image

### Requirement: RMK remains an integration example
The project SHALL treat the RMK target as a compatibility and resource-budget
example, not as acceptance evidence for HT32 peripheral behavior or complete
keyboard HID semantics.

#### Scenario: RMK example builds
- **WHEN** the fixed-keymap RMK target is built and linked
- **THEN** it demonstrates that the HT32 USB driver can integrate with the pinned framework
- **AND** matrix, keymap, rollover and report-semantic validation remains the responsibility of RMK or downstream firmware

### Requirement: Standalone USB transport regression
The HAL SHALL preserve its standalone CDC and bidirectional Raw HID examples
across the Embassy dependency migration.

#### Scenario: USB examples are rebuilt
- **WHEN** the migrated CDC and Raw HID examples are built and physically exercised
- **THEN** enumeration and bidirectional endpoint traffic continue to work
- **AND** composite keyboard scheduling is not claimed as a HAL acceptance result

### Requirement: Honest USB power capability
The firmware SHALL advertise remote wakeup only when the HT32 driver implements
resume signaling and that behavior has passed physical suspend/resume testing.

#### Scenario: Remote wakeup is unavailable
- **WHEN** resume signaling is not implemented or has not passed hardware validation
- **THEN** the USB configuration does not advertise remote-wakeup support
