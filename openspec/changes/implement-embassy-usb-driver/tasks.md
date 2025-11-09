## Checkpoint 1: Foundation & Compilation
- [x] 1.1 Research HT32 USB registers, 48MHz clock source, DM/DP pin alternate functions
- [x] 1.2 Define Usb struct, Irq struct, and interrupt handler in embassy-ht32 usb module
- [x] 1.3 Implement 48MHz USB clock configuration in rcc module
- [x] 1.4 Implement GPIO alternate function logic for DM/DP pins
- [x] 1.5 Implement Usb::new() constructor with peripheral setup
- [x] 1.6 Stub Driver trait implementation with todo!() functions
- [x] 1.7 Adapt embassy-usb-serial example and verify compilation

## Checkpoint 2: Core USB Implementation & Enumeration
- [ ] 2.1 Implement basic Driver methods (enable, set_address, stall, alloc_ep)
- [ ] 2.2 Implement Endpoint 0 packet buffer I/O (read/write)
- [ ] 2.3 Implement poll() handler with Reset/Setup/IN/OUT events
- [ ] 2.4 Add RTT logging and verify enumeration sequence via probe-run

## Checkpoint 3: Full Functionality & Data Transfer
- [ ] 3.1 Generalize driver methods for non-zero endpoints
- [ ] 3.2 Test bidirectional data transfer with serial terminal

## Bonus: Timer Conflict Resolution (Critical Issue Fixed)
- [x] 5.1 Identify thread-mode executor + USB feature timer hang issue
- [x] 5.2 Implement USB ISR Signal mechanism using embassy-sync
- [x] 5.3 Optimize GPTM0 critical section scope
- [x] 5.4 Convert to InterruptExecutor architecture for USB+Timer compatibility
- [x] 5.5 Verify Timer::after() works before/during/after USB operations
- [x] 5.6 Confirm USB peripheral access (CSR register read: 0x00000012)

## Validation & Testing
- [ ] 4.1 Verify USB enumeration via probe-run RTT logs
- [ ] 4.2 Confirm host-side device recognition (lsusb/dmesg)
- [ ] 4.3 Test complete USB serial communication
- [ ] 4.4 Document USB integration patterns