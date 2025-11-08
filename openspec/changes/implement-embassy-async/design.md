# Design: Embassy Async Integration Architecture

## Context
Embassy-ht32 currently has incomplete async functionality. The Embassy async runtime requires specific integration patterns that differ from current custom implementations. Based on analysis of embassy-stm32 reference implementation, we need to adopt standard Embassy patterns for critical sections, executor wake mechanisms, time drivers, and async peripheral support.

## Goals / Non-Goals
**Goals:**
- Provide complete Embassy async/await support for HT32F523xx
- Enable true interrupt-driven async operations (not polling)
- Ensure compatibility with embedded_hal_async ecosystem
- Follow Embassy standard patterns proven in embassy-stm32
- Maintain backward compatibility for existing synchronous code

**Non-Goals:**
- Create custom async runtime (use Embassy standard)
- Implement non-standard async patterns
- Modify Embassy core libraries
- Support async features beyond Embassy's current capabilities

## Decisions

### 1. Critical Section Implementation
**Decision**: Use Embassy's critical-section crate with standard Cortex-M acquire/release patterns

The current implementation in `interrupt.rs` already follows the correct pattern:
```rust
#[unsafe(no_mangle)]
pub unsafe extern "C" fn _critical_section_1_0_acquire() -> u32 {
    let primask: u32;
    core::arch::asm!("mrs {0}, PRIMASK", "cpsid i", out(reg) primask);
    primask
}
```

**Alternatives considered:**
- Custom critical section implementation (rejected - Embassy standard works)
- Platform-specific optimizations (rejected - would break Embassy compatibility)

### 2. Executor Wake Mechanism
**Decision**: Use embassy-executor with `arch-cortex-m` feature for PendSV-based task wake mechanism

Enable Cortex-M support through embassy-executor features:
```toml
embassy-executor = {
    version = "0.9.0",
    features = [
        "arch-cortex-m",
        "executor-interrupt",  # or "executor-thread"
    ]
}
```

The `__pender` function in `embassy-executor/src/arch/cortex_m.rs` will handle NVIV interrupt automatically when the feature is enabled.

**Alternatives considered:**
- Custom task scheduling (rejected - Embassy has proven implementation)
- Software interrupt alternatives (rejected - PendSV is Cortex-M standard)

### 3. Time Driver Integration
**Decision**: Replace custom alarm handling with standard embassy-time integration

Current issue: custom `trigger_alarm()` should be replaced with `embassy_time::alarm::on_interrupt()`:
```rust
// Instead of custom trigger_alarm
if intsr.ch1ccif().bit() {
    embassy_time::alarm::on_interrupt();  // Embassy standard
}
```

**Alternatives considered:**
- Keep custom alarm handling (rejected - not Embassy compatible)
- Hybrid approach (rejected - complexity for no benefit)

### 4. Async Peripheral Pattern
**Decision**: Implement AtomicWaker-based interrupt-driven async peripherals

Standard Embassy pattern for async peripherals:
```rust
static GPIO_WAKER: AtomicWaker = AtomicWaker::new();

pub async fn wait_for_rising_edge(&mut self) -> Result<(), Error> {
    core::future::poll_fn(|cx| {
        self.waker.register(cx.waker());
        if self.get_interrupt_flag() {
            self.clear_interrupt_flag();
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    }).await
}
```

**Alternatives considered:**
- Polling-based async (rejected - defeats async purpose)
- DMA-only approach (rejected - not suitable for all peripherals)

### 5. Implementation Strategy
**Decision**: 5-step progressive implementation with testing at each stage

Dependency chain: Critical Section → Executor → Time Driver → Interrupt Peripherals → Async Traits

Each step must be validated before proceeding to next, with comprehensive test coverage.

## Risks / Trade-offs

### Risk: Embassy Version Compatibility
**Mitigation**: Pin to tested Embassy versions and validate integration thoroughly

### Risk: HT32-Specific Hardware Differences
**Mitigation**: Adapt embassy-stm32 patterns to HT32 register layout while maintaining Embassy integration points

### Risk: Performance Degradation
**Mitigation**: Embassy's interrupt-driven approach should improve performance over polling; validate with benchmarks

### Trade-off: Implementation Complexity vs Reliability
**Decision**: Prioritize Embassy-standard patterns for maximum reliability, even if more complex initially

## Migration Plan

### Step 1: Validate Foundation
- Test current critical section implementation
- Ensure no breaking changes to existing synchronous code

### Step 2: Add Executor Support
- Add embassy-executor-cortex-m dependency
- Implement PendSV handler (minimal changes)
- Test basic async functionality

### Step 3: Fix Time Driver
- Replace custom alarm handling with Embassy standard
- Maintain backward compatibility for Timer usage
- Extensive time precision testing

### Step 4: Implement Async Peripherals
- Add interrupt-driven GPIO and UART
- Implement embedded_hal_async traits
- Comprehensive peripheral testing

### Step 5: Complete Integration
- Update all examples and documentation
- Performance validation
- RMK keyboard integration testing

### Rollback Strategy
Each step can be rolled back independently:
- Critical section: Keep current implementation (already working)
- Executor: Remove embassy-executor-cortex-m dependency
- Time driver: Revert to custom alarm handling
- Peripherals: Keep polling async as fallback

## Open Questions

1. **Embassy Executor Version**: Which version of embassy-executor-cortex-m provides best Cortex-M0+ support?
2. **Timer Hardware**: Should we use GPTM0 for time driver or dedicate a different timer for better isolation?
3. **Memory Constraints**: How to optimize async peripheral memory usage for 8KB RAM devices?
4. **Performance Targets**: What are specific performance goals for interrupt latency and power consumption?

## Success Criteria

- All Embassy async examples work without modification
- Timer accuracy within ±1% across full range
- Button response time under 100μs with interrupt-driven async
- UART async throughput comparable to blocking implementation
- Power consumption reduction measurable with async idle
- Full embedded_hal_async compatibility validation
- 24+ hour stability testing completed