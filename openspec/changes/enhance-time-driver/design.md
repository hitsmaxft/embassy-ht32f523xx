## Context
The HT32F523xx microcontroller requires a sophisticated time driver implementation that matches Embassy framework standards while leveraging the chip's full hardware capabilities. Current implementation uses basic GPTM (General Purpose Timer Module) with minimal interrupt management and limited clock configuration. Research reveals that ChibiOS HT32 implementation provides enterprise-grade time management with hardware clock monitoring, fault tolerance, and precise PLL configuration.

Key research findings:
- **ChibiOS Architecture**: Uses 4-layer HAL (Hardware Abstraction Layer) with LLD (Low-Level Driver) providing most functionality
- **Embassy Requirements**: Must implement `Driver` trait with `now()` and `schedule_wake()` methods, handle 64-bit timestamps from 32-bit hardware counters
- **HT32 Hardware**: BFTM (Basic Timer Module) provides optimal 32-bit resolution vs GPTM's 16-bit, supports compare channels for precise alarm management
- **Performance Targets**: <50μs interrupt latency, ±0.1% frequency accuracy, tens of thousands of years without overflow

## Goals / Non-Goals
**Goals:**
- Implement complete Enterprise-grade time driver matching Embassy framework standards
- Provide hardware fault tolerance with automatic clock failure detection and recovery
- Achieve sub-microsecond time precision with validated frequency accuracy
- Support comprehensive performance monitoring and diagnostic capabilities

**Non-Goals:**
- Multi-core time synchronization (single-core focus)
- External RTC integration (internal timers only)
- Dynamic voltage/frequency scaling (static configuration)
- Network Time Protocol support

## Decisions

### Timer Selection: BFTM vs GPTM
- **Decision**: Migrate from GPTM to BFTM for time driver core
- **Rationale**: BFTM offers 32-bit counters (vs GPTM's 16-bit), dedicated compare channels, simpler interrupt handling, better alignment with Embassy's 32-bit overflow algorithms
- **Implementation**: Replace `src/time_driver.rs` GPTM logic with BFTM register management, maintain identical `Driver` trait interface

### Clock Architecture: ChibiOS 4-Layer Pattern
- **Decision**: Adopt ChibiOS HAL LLD architecture with hardware abstraction layers
- **Rationale**: Provides proven enterprise patterns, clear separation of concerns, production-proven stability
- **Implementation**: Create new `src/time/clocks.rs` module with clear layer boundaries

### Overflow Management: Enhanced Half-Cycle Algorithm
- **Decision**: Implement 2^31 half-cycle overflow prevention (vs current 2^15 approach)
- **Rationale**: Provides ~64 second periods vs ~32ms, reducing interrupt frequency by 1000x, improved race condition handling
- **Implementation**: Double-check overflow detection in both directions, compare channel at 0x8000_0000 threshold

### Fault Tolerance: Hardware Clock Monitoring
- **Decision**: Enable HT32 hardware clock failure detection (CKMEN) with NMI handler
- **Rationale**: Provides enterprise-grade reliability, automatic fallback to safe HSI clock, transparent to applications
- **Implementation**: Configure GCCR.CKMEN, implement NMI_Handler for fault detection, automatic failover logic

### Performance Monitoring: Comprehensive Metrics
- **Decision**: Add detailed performance counters, interrupt latency measurement, self-diagnostics
- **Rationale**: Enables production monitoring, debugging capabilities, long-term stability validation
- **Implementation**: Atomic counters for all timing events, cycle-accurate latency measurement, automated validation tests

## Risks / Trade-offs

**Hardware-Specific Changes**
- **Risk**: New implementation may have undiscovered HT32-specific quirks
- **Mitigation**: Extensive validation testing with physical hardware, fallback to GPTM if BFTM issues discovered

**Interrupt Latency Impact**
- **Risk**: More complex interrupt handlers could increase critical path latency
- **Mitigation**: Inline interrupt functions, minimal critical sections, performance measurement instrumentation

**Memory Usage Increase**
- **Risk**: Additional features consume more Flash/RAM resources
- **Mitigation**: Configurable feature flags, optional debugging features, memory-optimized data structures

**Breaking API Changes**
- **Risk**: Applications may depend on current simple timing behavior
- **Mitigation**: Maintain exact `Driver` trait interface, provide compatibility shim if needed, extensive testing against existing code

## Migration Plan

### Phase 1: Foundation (Week 1-2)
- Create new clock management module `src/time/clocks.rs` with ChibiOS patterns
- Implement BFTM hardware abstraction layer
- Develop basic fault tolerance framework

### Phase 2: Driver Integration (Week 3)
- Replace existing `time_driver.rs` with BFTM-based implementation
- Implement enhanced 64-bit timestamp algorithms
- Add performance monitoring infrastructure

### Phase 3: Testing & Validation (Week 4)
- Comprehensive hardware testing with measurement equipment
- Long-term stability validation (24+ hour runs)
- Performance benchmarking against previous implementation

### Rollback Strategy
Keep original GPTM implementation in `src/time/legacy_time_driver.rs` with feature flag `legacy-time-driver` to enable instant rollback if critical issues discovered.

## Open Questions

1. **BFTM Channel Mapping**: Confirm exact BFTM compare channel numbering for alarm management
2. **NMI Vector Priority**: Verify NMI handler interaction with existing interrupt priorities
3. **Clock Source Validation**: Need measurement equipment to validate ±0.1% frequency accuracy claims
4. **Production Configurations**: Determine most common production clock/PLL configurations for default optimization

## Implementation Notes

Based on comprehensive research analysis from `/Users/bhe/projects/keyboard/embassy-ht32/deps/agent_rearch_time_driver/`, the upgrade addresses critical gaps identified in VI.0 vs V2.0 comparison:

- **Headers**: ChibiOS provides complete register definitions with detailed bit positions
- **Performance**: Enterprise-grade architecture increases code size by 140% but provides massive reliability gains
- **Features**: Hardware monitoring, fault tolerance, performance metrics transform basic timer into enterprise solution
- **Testing**: Comprehensive validation framework ensures production readiness

Key validation metrics from research to validate success:
- Frequency accuracy: ±0.1% measured against precision frequency counter
- Interrupt latency: <50μs measured with oscilloscope
- Long-term stability: <100ppm drift over 24-hour period
- Fault tolerance: 100% successful automatic HSI fallback during clock failure simulations