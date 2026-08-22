# Embassy HT32F523xx Implementation Progress

> Last updated: 2026-08-22<br>
> Target used for hardware validation: HT32F52352 on ESK32-30501<br>
> Status: active development; this is not a production-readiness certificate

## Evidence levels

- **Implemented**: a public HAL/API path exists and builds for the target.
- **Board-tested**: the public path has executed on the target board and its
  observable result was checked.
- **Signal-tested**: external pins were captured and decoded with suitable
  measurement equipment.
- **Planned**: the PAC contains the peripheral, but this crate does not expose a
  usable driver. A PAC-only experiment does not promote a planned driver to
  implemented status.

## Current capability matrix

| Capability | Implementation | Current hardware evidence | Remaining acceptance |
|------------|----------------|---------------------------|----------------------|
| Clock/RCC | HSI, HSE, PLL 48 MHz, AHB/APB and USB clock setup | HSI/HSE/PLL boot paths and timer-derived ratios board-tested | Optional clock-output/marker capture for absolute jitter characterization |
| GPIO | Input/output, pull configuration, drive selection, type-erased pins, embedded-hal digital traits | PC14/PC15 output and PB12/input paths board-tested | Logic-analyzer edge timing; oscilloscope/load test for drive strength and rise/fall time |
| EXTI | Edge configuration, grouped IRQ dispatch, `embedded_hal_async::digital::Wait` | Software trigger and physical self-driven edge board-tested | Two-channel input-to-response latency/jitter, polarity and re-arm capture |
| Embassy time | GPTM0 1 MHz time base | HSI/HSE/PLL delays and timer ratios board-tested | GPIO marker capture across short/long delays and concurrent load |
| BFTM | BFTM0/BFTM1 asynchronous one-shot timers | Multiple durations and concurrent waits board-tested | GPIO marker timing/jitter capture |
| USART0/1 | Blocking `embedded-hal-nb` read/write plus inherent async buffer methods | USART0 blocking/async TX board-tested | Logic-analyzer framing/baud test and physical RX loopback; this crate does not yet implement standard `embedded-hal-async` serial traits |
| USB FS device | Embassy USB driver, control pipe, bulk/interrupt endpoints | Enumeration, CDC echo and 64-byte vendor Raw HID bidirectional full-load test board-tested | Long soak, reconnect/reset/error recovery and USB electrical/compliance testing |
| Flash | Read plus asynchronous 512-byte erase and 32-bit program | Erase/program/readback/cleanup board-tested | Power-loss/protection/boundary tests; synchronous `NorFlash::erase/write` still return an error |
| PWM | No public HAL driver | None | Implement GPTM/SCTM/MCTM PWM, then run the analyzer matrix below |
| I2C0/1 | No HAL driver | None | Implement first, then protocol/timing tests below |
| SPI0/1 | No HAL driver | None | Implement first, then protocol/timing tests below |
| ADC, PDMA, RTC, WDT, I2S, SCI, EBI, CRC, CMP | No HAL driver | None | Separate implementation proposals and peripheral-specific validation |

## Keyboard-framework boundary

The pinned RMK 0.9 fixed-keymap example builds against Embassy USB 0.6 and
links with 8,056 bytes of RAM headroom. This is an integration/resource result,
not an HT32 peripheral capability. Matrix scanning semantics, debounce,
rollover, keymap behavior, Keyboard/Consumer/System reports, host LED Output and
composite Raw HID scheduling belong to RMK or downstream keyboard firmware.

The RMK revision currently advertises remote wakeup while the HT32 driver does
not implement resume signaling. The example therefore remains excluded from
hardware acceptance until that USB capability mismatch is resolved and tested.

## Final logic-analyzer acceptance experiment

The analyzer run is the final digital-I/O acceptance gate, after the relevant
public driver and its normal functional tests exist. Test firmware must use the
same public HAL API that applications use; direct PAC writes are useful for
bring-up only and are not acceptance evidence for the HAL.

### Existing capabilities

1. **GPIO/EXTI**
   - Drive a free header output into a separate EXTI input and observe both on
     analyzer channels.
   - Verify static high/low, push-pull polarity, programmed square-wave period
     and duty, both-edge detection, repeated arm/wake cycles, and no spurious
     responses.
   - Record input-edge-to-response-edge latency distribution under idle and
     concurrent USB load.
2. **Embassy time/BFTM**
   - Emit marker edges around 1 ms, 10 ms, 100 ms and 1 s waits and concurrent
     BFTM0/BFTM1 waits.
   - Compare measured intervals with the configured clock; record minimum,
     maximum, mean and worst error rather than only a pass/fail message.
3. **USART**
   - Decode continuous patterned data at supported baud rates and at 7/8/9 data
     bits, parity and one/two stop bits where the API accepts them.
   - Verify baud error, idle polarity, byte order, gaps and sustained transfers.
   - Use a physical TX-to-RX loopback or analyzer pattern generator to validate
     RX, framing/parity/overrun handling and async wake-up. Board routing/jumpers
     must be documented in the result.

### Planned capabilities

1. **I2C**: open-drain release, external pull-ups, START/repeated START/STOP,
   7-bit addressing, ACK/NACK, read/write/combined transactions, 100/400 kHz
   timing, clock stretching, arbitration/error paths and stuck-bus recovery.
2. **SPI**: modes 0-3, configured SCK rates, MSB/LSB order if exposed,
   chip-select setup/hold, full-duplex MOSI/MISO loopback, single/multi-byte and
   boundary-length transfers, and sustained traffic under interrupt load.
3. **PWM**: 0%, 1%, 25%, 50%, 75%, 99% and 100% duty; representative low/high
   frequencies; active-high/low polarity; enable/disable idle state; and
   period/duty changes at update boundaries without runt or extra pulses.

## Equipment boundary

A logic analyzer establishes digital timing and protocol behavior. It cannot
establish 4 mA versus 8 mA output-drive compliance, analog voltage margins,
rise/fall behavior under a specified load, oscillator analog quality, ADC
accuracy or supply integrity. Use an oscilloscope, DMM, known loads and signal
source for those claims. USB functional evidence comes from enumeration and
endpoint traffic; USB electrical compliance requires dedicated USB equipment.
For future drivers, I2S needs decoded clock/data capture, WDT timing can be
observed with a pre-reset GPIO marker, and PDMA must be checked through sustained
traffic on its client peripheral. RTC/LSE drift needs a frequency counter or a
long reference window rather than a short analyzer trace.

## Near-term order

1. Run the existing GPIO/EXTI, Embassy-time/BFTM and USART analyzer suite when
   a supported analyzer and safe jumper wiring are available.
2. Finish USART RX board routing/loopback validation.
3. Propose and implement I2C, then run its protocol/timing gate.
4. Propose and implement SPI, then run its protocol/timing gate.
5. Propose and implement PWM, then run its waveform/update gate.
6. Treat the combined analyzer run as the final digital-I/O acceptance
   experiment; keep analog/electrical compliance as a separate gate.
