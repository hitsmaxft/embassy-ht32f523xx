# Embassy HT32F523xx Validation Report

> Updated: 2026-08-22<br>
> Hardware: HT32F52352 on ESK32-30501<br>
> Scope: current working tree; results are per capability, not a blanket pass

## How to read this report

Static agreement with a datasheet, user manual, PAC, Holtek FWLib or ChibiOS
does not prove physical behavior. Likewise, a target-side register/counter check
does not prove pin-level timing. Evidence is therefore kept in separate gates:

1. source/reference review;
2. target build and flash;
3. board-observable functional test;
4. external signal or protocol measurement;
5. stress, recovery and long-duration testing.

## Current results

| Area | Source/reference review | Board-functional evidence | External measurement | Status |
|------|-------------------------|---------------------------|----------------------|--------|
| Clock/RCC | Compared with PAC, official HT32 documentation and ChibiOS clock use | HSI, HSE and PLL-to-48-MHz paths boot; AHB/APB/USB clock relationships checked with timer counts | No dedicated clock-output/jitter capture | Functional; signal characterization pending |
| GPIO | Register layout, AF selection model, active-low LED and header mapping reviewed | PC14/PC15 output and GPIO input/drive-register behavior checked | No logic-analyzer or loaded-edge capture | Functional; final signal QA pending |
| EXTI | Grouped vectors, line selection and flag clearing reviewed | Software-triggered and physically driven edge wake-up checked | No two-channel latency/jitter distribution | Functional; final signal QA pending |
| Embassy time/GPTM0 | Timer clock source, prescaler, reload shadowing and update event reviewed | Delays pass with HSI/HSE/PLL; timer ratios follow configured APB divider | No marker-edge accuracy/jitter capture | Functional; final timing QA pending |
| BFTM0/1 | One-shot enable, compare, status clear and interrupt behavior compared with reference code | 1/10/100/500 ms waits and concurrent waits checked | No external marker capture | Functional; final timing QA pending |
| USART | Divider, format, FIFO and interrupt-source handling reviewed | USART0 blocking and inherent-async TX checked | RX path and decoded waveform not yet captured | Partial |
| USB FS | Control transfer, address sequencing, endpoint buffers and IRQ path reviewed | Enumeration, CDC echo and vendor Raw HID bidirectional sustained traffic checked | Host protocol evidence exists; USB electrical compliance not tested | Functionally validated, not electrically certified |
| Flash | FMC command sequence, protected region and readback checks reviewed | Page erase, word program, verify and cleanup checked | Not a logic-analyzer target | Partial API: async operations work; synchronous erase/write remain unsupported |
| I2C | PAC peripheral exists | No HAL driver | None | Planned |
| SPI | PAC peripheral exists | No HAL driver | None | Planned |
| PWM | Timer peripherals and a ChibiOS PWM reference exist | No public HAL driver | None | Planned |

## RMK integration evidence

The no-Vial/no-storage RMK 0.9 example now passes a release workspace build and
the automated image budget gate: 71,680 bytes of loadable Flash, 8,328 bytes of
static RAM and 8,056 bytes remaining for stack/runtime use. These are static
build facts and are separate from the following board evidence.

On 2026-08-22, the RMK validation image was flashed through Holtek CMSIS-DAP
`02000D2E` and enumerated at USB full speed as VID:PID `4c4b:4643`, product
`HT32 RMK Integration`, serial `HT32-RMK-60K-0001`. macOS registered Keyboard,
Mouse, Consumer and System Control collections.

The debugger then submitted row 2 / column 1 press and release commands through
the test-only SWD mailbox. Evidence from the normal RMK event path was:

| Step | Command | Ack | EP1 payload | Target evidence |
|------|---------|-----|-------------|-----------------|
| `A` press | `c0000201` | `e0000201` | `00 00 04 00 00 00 00 00` | RMK event logged; `IDTX`; endpoint write completed |
| `A` release | `80000201` | `a0000201` | `00 00 00 00 00 00 00 00` | RMK event logged; `IDTX`; endpoint write completed |

This validates debugger injection -> RMK event channel -> fixed keymap -> RMK
keyboard report -> HT32 endpoint 1 -> completed USB IN transaction. It does
not validate the illustrative physical matrix wiring, nor does it claim an OS
application-level text result. Complete HID semantics remain an RMK/application
responsibility, as required by the project scope.

The pinned RMK revision advertises remote wakeup while
`Bus::remote_wakeup` remains unsupported. Basic RMK execution and keyboard
press/release therefore pass, but suspend/remote-wakeup remains unaccepted.

Flashing was reliable only at 500 kHz with probe-rs double buffering disabled;
an initial erase attempt could leave the core in `LockedUp`, while a retry with
`--disable-double-buffering --verify` succeeded. This is retained as a probe /
Flash-algorithm robustness issue rather than hidden by the functional result.

The crates.io `0.2.1` candidate replaces the Git-only PAC with registry
`ht32f523x2 0.6.0`. After adapting to its svd2rust 0.37 API, both MCU feature
builds and the complete workspace passed. The RMK image was then reflashed and
the same SWD `A` press/release experiment again produced `e0000201` /
`a0000201`, the expected EP1 payloads, `IDTX`, and successful endpoint-write
completion. This is a hardware regression result for the PAC migration.

## Logic-analyzer gate

The final digital-I/O acceptance experiment SHALL use public HAL APIs. A direct
PAC test can help diagnose hardware, but cannot be recorded as a HAL pass.

### Required now

- **GPIO/EXTI**: output edge and response marker on separate channels; verify
  polarity, period/duty, rising/falling/both-edge behavior, re-arm, latency and
  jitter under idle and concurrent USB load.
- **Embassy time/BFTM**: marker intervals at 1 ms, 10 ms, 100 ms and 1 s, plus
  concurrent BFTM0/BFTM1 waits; record measured error and jitter.
- **USART**: protocol decode for supported baud/data/parity/stop settings,
  sustained TX, physical TX-RX loopback, RX wake-up, and framing/parity/overrun
  errors. The Starter Kit routing/jumper state must be recorded.

### Required after implementation

- **I2C**: open-drain release and pull-ups, START/repeated START/STOP, ACK/NACK,
  combined transfers, 100/400 kHz timing, clock stretching and stuck-bus
  recovery.
- **SPI**: modes 0-3, SCK accuracy, bit order, chip-select setup/hold,
  full-duplex loopback, transfer boundaries and sustained traffic.
- **PWM**: frequency and 0/1/25/50/75/99/100% duty, polarity, idle state and
  glitch-free updates to period/duty.

I2C, SPI and PWM are not presently implemented, so their analyzer cases are a
release plan rather than an outstanding failure of an existing driver.

## Equipment limits

Logic analyzers measure digital levels, timing and decoded protocols. They do
not validate GPIO 4 mA/8 mA drive-current limits, loaded rise/fall time, analog
threshold margin, ADC accuracy, oscillator analog quality or power integrity.
Use an oscilloscope/DMM, defined loads and an analog source for those tests.
USB electrical compliance likewise requires dedicated USB measurement tools.
For future peripherals, I2S should be protocol-decoded, WDT timeout/reset can be
timed from a GPIO marker, and PDMA should be validated through sustained client
peripheral traffic. RTC/LSE drift is better measured with a frequency counter
or long reference window.

## Release interpretation

The current implementation is suitable for continued bring-up and driver
development. It must not be described as fully hardware validated or production
ready until:

- the existing GPIO/EXTI/time/BFTM/USART external-signal gate passes;
- USART RX and error paths pass physical tests;
- USB recovery and soak cases pass;
- every newly implemented I2C, SPI or PWM driver passes its protocol/waveform
  matrix; and
- electrical/analog claims are supported by the appropriate instruments.
