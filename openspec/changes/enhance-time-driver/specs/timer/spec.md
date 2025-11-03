## ADDED Requirements

### Requirement: Enterprise Clock Management System
The system SHALL provide comprehensive microcontroller clock management with hardware failure detection and recovery capabilities supporting at least HSI/HSE/PLL/LSI/LSE clock sources.

#### Scenario: Hardware Clock Failure Detection
- **WHEN** an external crystal oscillator (HSE) becomes unstable or fails
- **THEN** the hardware monitoring system SHALL automatically detect the failure through CKMEN
- **AND** trigger an NMI interrupt within 10 clock cycles
- **AND** automatically switch system clock to the internal HSI oscillator
- **AND** provide error reporting to the application layer

#### Scenario: PLL Precise Frequency Configuration
- **WHEN** configuring system frequency through PLL multiplication
- **THEN** the system SHALL support configurable input divider (INDIV) and feedback divider (FBDIV)
- **AND** provide frequency accuracy to within ±0.1% of target frequency
- **AND** validate PLL lock status before switching system clock
- **AND** support input frequencies from 1MHz to 16MHz and output frequencies up to 144MHz

#### Scenario: Multiple Clock Source Management
- **WHEN** applications request different clock configurations
- **THEN** the system SHALL support 5 independent clock sources (HSI/HSE/PLL/LSI/LSE)
- **AND** provide safe switching between clock sources without disrupting time references
- **AND** maintain time continuity during clock source transitions

### Requirement: Enhanced Embassy Time Driver Implementation
The system SHALL implement the Embassy Rust async framework `Driver` trait with enterprise-grade features including sub-microsecond timestamp precision and comprehensive queue management for concurrent timer operations.

#### Scenario: Concurrent Timer Operations
- **WHEN** applications create multiple timers (up to 16 concurrent alarms)
- **THEN** the system SHALL provide reliable scheduling with ≤1μs precision
- **AND** maintain independent wake-up times for each timer
- **AND** wake appropriate task when timer expires using Embassy's Waker mechanism

#### Scenario: 64-Bit Timestamp Accuracy
- **WHEN** applications query current time through `now()` function
- **THEN** the system SHALL return accurate 64-bit timestamps in microseconds
- **AND** maintain monotonic time (never goes backwards)
- **AND** support up to 584,554 years of continuous operation without overflow
- **AND** provide ±0.1% accuracy against crystal reference

#### Scenario: Timer Overflow Management
- **WHEN** hardware timer counters approach 32-bit overflow points
- **THEN** the system SHALL detect overflow conditions using enhanced half-cycle algorithm
- **AND** update software period counters atomically without losing time
- **AND** handle race conditions safely between interrupt and foreground execution

### Requirement: Advanced Timer Hardware Utilization
The system SHALL migrate from basic GPTM to BFTM timer architecture to utilize superior hardware capabilities including 32-bit resolution, multiple compare channels, and optimized interrupt patterns.

#### Scenario: BFTM Timer Configuration
- **WHEN** initializing the time driver
- **THEN** the system SHALL configure BFTM0 for 1MHz tick frequency
- **AND** set up compare channels for alarm wake-up management
- **AND** configure appropriate prescaler based on system clock frequency
- **AND** enable match interrupts (MIEN) for precise timing events

#### Scenario: Compare Channel Management
- **WHEN** scheduling wake-up events with `schedule_wake()` function
- **THEN** the system SHALL program appropriate BFTM compare registers
- **AND** enable hardware interrupts only when needed to reduce interrupt overhead
- **AND** handle both near-term (<128ms) and long-term (>128ms) wake-up scheduling efficiently
- **AND** ensure no wake-up events are lost due to hardware timing constraints

#### Scenario: Performance Optimized Interrupt Handling
- **WHEN** timer interrupts occur at typical 1MHz or 327Hz rates
- **THEN** the system SHALL respond with interrupt latency <50μs
- **AND** clear interrupt flags efficiently without hardware race conditions
- **AND** update compare values for the next scheduled event without blocking
- **AND** wake appropriate Embassy tasks through Rust's Waker mechanism

### Requirement: Enterprise Performance Monitoring and Diagnostics
The system SHALL provide comprehensive performance metrics, statistical analysis, and self-diagnostic capabilities suitable for production monitoring and validation of time-critical applications.

#### Scenario: System Performance Metrics Collection
- **WHEN** the time driver operates under production load
- **THEN** the system SHALL automatically collect performance statistics including:
  - Total calls to `now()` function with average/maximum latency measurements
  - Successful vs failed `schedule_wake()` operations
  - Interrupt frequency and average processing time in nanoseconds
  - Active alarm count and success rate for wake-up events
  - Hardware fault events including clock failures detected

#### Scenario: Self-Diagnostic Validation
- **WHEN** system starts up or periodically during operation
- **THEN** the system SHALL perform comprehensive self-diagnostics including:
  - Monotonicity tests to ensure timestamps never decrease
  - Overflow detection algorithm correctness verification
  - Hardware register read/write consistency validation
  - Timer accuracy measurement against known reference intervals
- **AND** report any diagnostic failures through appropriate error channels

#### Scenario: Long-Term Stability Validation
- **WHEN** system operates continuously for extended periods (hours to days)
- **THEN** the system SHALL monitor and report long-term timing accuracy with <100ppm frequency drift
- **AND** detect and log any unusual time measurement patterns or inconsistencies
- **AND** validate interrupt response times remain consistent over time periods
- **AND** ensure no accumulated timing errors exceed specification limits

### Requirement: Production-Ready Error Handling and Recovery
The system SHALL implement enterprise-grade error detection, reporting, and recovery mechanisms that maintain operation even under adverse conditions while providing detailed failure information.

#### Scenario: Graceful Degradation on Resource Exhaustion
- **WHEN** the maximum number of concurrent alarms is exceeded (16 alarms)
- **THEN** the system SHALL return appropriate error status to applications rather than failing silently
- **AND** provide guidance on alarm count limits exceeded
- **AND** maintain operation for existing alarms without disruption
- **AND** allow recovery when alarm resources become available

#### Scenario: Time Reference Validation Under Stress
- **WHEN** system experiences extreme interrupt load or hardware stress conditions
- **THEN** the system SHALL validate time reference continuity every 100ms minimum
- **AND** detect any timing inconsistencies >1μs that exceed specification
- **AND** attempt automatic recovery through hardware reset or fall back to degraded mode
- **AND** provide detailed diagnostic information for debugging complex timing issues

## MODIFIED Requirements

### Requirement: Enhanced Embassy Framework Driver Trait Implementation
The system SHALL provide significantly improved time driver implementation using BFTM instead of GPTM with hardware interrupt-driven alarm management, 32-bit counter resolution, and sophisticated overflow handling algorithms.

#### Scenario: Enhanced Time Reference Calculation
- **WHEN** applications call `now()` to get current time in microseconds
- **THEN** the system SHALL use enhanced half-cycle algorithm (2^31 instead of 2^15 periods)
- **AND** provide more precise time resolution with reduced interrupt frequency
- **AND** handle hardware counter reads with proper memory barriers and race condition protection
- **AND** return consistently accurate timestamps across overflow boundaries

#### Scenario: Improved Wake-up Event Scheduling
- **WHEN** applications schedule wake-up events through `schedule_wake(timestamp, waker)`
- **THEN** the system SHALL update hardware compare registers for interrupt-driven wake-up instead of polling
- **AND** support longer time intervals without frequent hardware interrupts
- **AND** provide more reliable event delivery with reduced CPU usage
- **AND** enable sophisticated alarm queue management with multiple concurrent events

### Requirement: Optimized Interrupt Service Routine Implementation
The system SHALL provide sophisticated interrupt service routine optimized for minimal latency with comprehensive state management and performance measurement integration replacing basic interrupt handler.

#### Scenario: Optimized Timer Interrupt Processing
- **WHEN** hardware timer triggers interrupts at scheduled intervals
- **THEN** the system SHALL process interrupts with latency under 50μs measured from hardware trigger to ISR entry
- **AND** efficiently clear interrupt flags to prevent race conditions with subsequent interrupts
- **AND** determine the next scheduled wake-up time with minimal calculation overhead
- **AND** update hardware compare registers for the subsequent scheduled event