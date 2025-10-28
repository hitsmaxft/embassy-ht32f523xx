# hardware specification 

> all these text is copy from datasheet of HT325f52342_52

## FEATURES

### Core

▆ 32-bit Arm® Cortex®-M0+ processor core
▆ Up to 48 MHz operating frequency
▆ Single-cycle multiplication
▆ Integrated Nested Vectored Interrupt Controller (NVIC)
▆ 24-bit SysTick timer
The Cortex®-M0+ processor is a very low gate count, highly energy efficient processor that is
intended for microcontroller and deeply embedded applications that require an area optimized,
low-power processor. The processor is based on the ARMv6-M architecture and supports Thumb®
instruction sets, single-cycle I/O ports, hardware multiplier and low latency interrupt respond time.

### On-Chip Memory

▆ Up to 128 KB on-chip Flash memory for instruction/data and option byte storage
▆ 16 KB on-chip SRAM
▆ Supports multiple boot modes
The Arm® Cortex®-M0+ processor accesses and debug accesses share the single external interface
to external AHB peripherals. The processor access takes priority over debug access. The maximum
address range of the Cortex®-M0+ is 4 GB since it has a 32-bit bus address width. Additionally,
a pre-defined memory map is provided by the Cortex®-M0+ processor to reduce the software
complexity of repeated implementation by different device vendors. However, some regions are used
by the Arm® Cortex®-M0+ system peripherals. Refer to the Arm® Cortex®-M0+ Technical Reference
Manual for more information. Figure 2 shows the memory map of the HT32F52342/52352 series of
devices, including code, SRAM, peripheral, and other pre-defined regions.

### Flash Memory Controller – FMC
▆ Flash accelerator for maximum efficiency
▆ 32-Bit word programming with In System Programming (ISP) and In Application Programming (IAP)
▆ Flash protection capability to prevent illegal access
The Flash Memory Controller, FMC, provides all the necessary functions and pre-fetch buffer for
the embedded on-chip Flash Memory. Since the access speed of the Flash Memory is slower than the
CPU, a wide access interface with a pre-fetch buffer and cache are provided for the Flash Memory
in order to reduce the CPU waiting time which will cause CPU instruction execution delays. Flash
Memory word program/page erase functions are also provided.

### Reset Control Unit – RSTCU

▆ Supply supervisor:
● Power On Reset / Power Down Reset – POR/PDR
● Brown-out Detector – BOD
● Programmable Low Voltage Detector – LVD
The Reset Control Unit, RSTCU, has three kinds of reset, a power on reset, a system reset and an
APB unit reset. The power on reset, known as a cold reset, resets the full system during power up. A
system reset resets the processor core and peripheral IP components with the exception of the SW-DP
controller. The resets can be triggered by an external signal, internal events and the reset generators.

### External Interrupt/Event Controller – EXTI
▆ Up to 16 EXTI lines with configurable trigger source and type
▆ All GPIO pins can be selected as EXTI trigger source
▆ Source trigger type includes high level, low level, negative edge, positive edge, or both edges
▆ Individual interrupt enable, wakeup enable and status bits for each EXTI line
▆ Software interrupt trigger mode for each EXTI line
▆ Integrated deglitch filter for short pulse blocking
The External Interrupt/Event Controller, EXTI, comprises 16 edge detectors which can generate
a wake-up event or interrupt requests independently. Each EXTI line can also be masked
independently.

### I/O Ports – GPIO
▆ Up to 51 GPIOs
▆ Port A, B, C, D are mapped as 16 external interrupts – EXTI
▆ Almost all I/O pins have a configurable output driving current.
There are up to 51 General Purpose I/O pins, GPIO, named from PA0~PA15, PB0 ~ PB8, PB10 ~
PB15. PC0 ~ PC15 and PD0~PD3 for the implementation of logic input/output functions. Each of
the GPIO ports has a series of related control and configuration registers to maximise flexibility and
to meet the requirements of a wide range of applications.
The GPIO ports are pin-shared with other alternative functions to obtain maximum functional
flexibility on the package pins. The GPIO pins can be used as alternative functional pins by
configuring the corresponding registers regardless of the input or output pins. The external interrupts
on the GPIO pins of the device have related control and configuration registers in the External
Interrupt Control Unit, EXTI.

### General-Purpose Timer – GPTM
▆ 16-bit up, down, up/down auto-reload counter
▆ Up to 4 independent channels for each timer
▆ 16-bit programmable prescaler that allows division of the prescaler clock source by any factor
between 1 and 65536 to generate the counter clock frequency
▆ Input Capture function
▆ Compare Match Output
▆ PWM waveform generation with Edge-aligned and Center-aligned Counting Modes
▆ Single Pulse Mode Output
▆ Encoder interface controller with two inputs using quadrature decoder
The General Purpose Timer Module, GPTM, consists of one 16-bit up/down-counter, four 16-bit
Capture/Compare Registers (CCRs), one 16-bit Counter Reload Register (CRR) and several control
/ status registers. They can be used for a variety of purposes including general time measurement,
input signal pulse width measurement, output waveform generation such as single pulse generation,
or PWM output generation. The GPTM supports an Encoder Interface using a decoder with two
inputs.

### Universal Serial Bus Device Controller – USB

▆ Complies with USB 2.0 full-speed (12 Mbps) specification
▆ On-chip USB full-speed transceiver
▆ 1 control endpoint (EP0) for control transfer
▆ 3 single-buffered endpoints for bulk and interrupt transfer
▆ 4 double-buffered endpoints for bulk, interrupt and isochronous transfer
▆ 1,024 bytes EP_SRAM used as the endpoint data buffers
The USB device controller is compliant with the USB 2.0 full-speed specification. There is one
control endpoint known as Endpoint 0 and seven configurable endpoints. A 1024-byte EP_SRAM
is used as the endpoint buffer. Each endpoint buffer size is programmable using corresponding
registers, which provides maximum flexibility for various applications. The integrated USB
full-speed transceiver helps to minimise the overall system complexity and cost. The USB functional
block also contains the resume and suspend feature to meet the requirements of low-power
consumption.
