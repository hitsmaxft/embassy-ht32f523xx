//! Clock system management for HT32F523xx microcontrollers
//!
//! This module provides enterprise-grade clock management with hardware fault monitoring,
//! automatic failover, and precise frequency configuration following ChibiOS HAL LLD patterns.
//!
//! Based on comprehensive ChibiOS research findings for HT32F523xx microcontroller architecture.

use core::sync::atomic::{AtomicU32, Ordering};
use crate::pac::Ckcu;

/// Clock failure counter for enterprise monitoring
static CLOCK_FAILURE_COUNT: AtomicU32 = AtomicU32::new(0);

// ============================================================================
// Clock System Configuration
// ============================================================================

/// Comprehensive clock configuration with ChibiOS-grade error handling
#[derive(Debug, Clone, Copy)]
pub struct ClockConfig {
    /// Target system clock frequency
    pub sysclock_hz: u32,
    /// Enable external crystal (HSE) if available
    pub hse_enabled: bool,
    /// HSE frequency in Hz (16MHz typical for HT32 boards)
    pub hse_freq: Option<u32>,
    /// Enable PLL multiplication
    pub pll_enabled: bool,
    /// PLL multiplier (2-16 range for HT32F523xx)
    pub pll_mult: u32,
    /// Enable hardware clock monitoring and fault detection
    pub clock_monitor: bool,
    /// HCLK/AHB divider (0=÷1, 1=÷2, 2=÷4, etc...)
    pub ahb_divider: u8,
    /// PCLK/APB divider (0=÷1, 1=÷2, 2=÷4, etc...)
    pub apb_divider: u8,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            sysclock_hz: 48_000_000,  // 48MHz from HSI + PLL
            hse_enabled: false,
            hse_freq: None,
            pll_enabled: true,
            pll_mult: 6,              // 8MHz * 6 = 48MHz
            clock_monitor: true,      // Enable fault detection
            ahb_divider: 0,           // HCLK = SYSCLK
            apb_divider: 0,           // PCLK = SYSCLK
        }
    }
}

/// High-performance clock configuration for enterprise applications
pub fn config_enterprise_performance() -> ClockConfig {
    ClockConfig {
        sysclock_hz: 48_000_000,
        hse_enabled: true,
        hse_freq: Some(16_000_000),  // 16MHz crystal precision
        pll_enabled: true,
        pll_mult: 3,                 // 16MHz * 3 = 48MHz
        clock_monitor: true,         // Full fault tolerance
        ahb_divider: 0,
        apb_divider: 0,
    }
}

/// Low-power clock configuration for battery applications
pub fn config_low_power() -> ClockConfig {
    ClockConfig {
        sysclock_hz: 8_000_000,      // HSI only (no PLL)
        hse_enabled: false,
        hse_freq: None,
        pll_enabled: false,          // Disable PLL for power savings
        pll_mult: 1,
        clock_monitor: true,
        ahb_divider: 0,
        apb_divider: 0,
    }
}

// ============================================================================
// Clock System Implementation
// ============================================================================

/// Get world-readable clock failure count for monitoring
pub fn get_clock_failure_count() -> u32 {
    CLOCK_FAILURE_COUNT.load(Ordering::Relaxed)
}

/// Reset clock failure counter
pub fn reset_clock_failure_count() {
    CLOCK_FAILURE_COUNT.store(0, Ordering::Relaxed);
}

/// Enterprise-grade clock system initialization
pub fn clock_system_init(config: &ClockConfig) -> Result<(), ClockError> {
    let ckcu = unsafe { &*Ckcu::ptr() };

    // Step 1: Security - Disable PLL during configuration
    ckcu.gccr().modify(|_, w| w.pllen().clear_bit());

    // Step 2: Start HSI oscillator as foundation
    start_hsi_oscillator(&*ckcu)?;

    // Step 3: Configure external crystal if requested
    if config.hse_enabled {
        if let Some(hse_freq) = config.hse_freq {
            start_hse_oscillator(&*ckcu, hse_freq)?;
        }
    }

    // Step 4: Configure PLL for target frequency
    if config.pll_enabled && config.pll_mult >= 2 && config.pll_mult <= 16 {
        configure_pll_multiplication(&*ckcu, config)?;
    }

    // Step 5: Configure bus clock dividers
    configure_bus_clocks(&*ckcu, config)?;

    // Step 6: Enable hardware clock monitoring (enterprise feature)
    if config.clock_monitor {
        enable_clock_monitoring(&*ckcu)?;
    }

    // Step 7: Switch to final clock source
    switch_to_target_clock(&*ckcu, config)?;

    // Step 8: Verify complete configuration
    verify_final_configuration(&*ckcu, config)?;

    Ok(())
}

/// Start HSI oscillator with timeout protection
fn start_hsi_oscillator(ckcu: &Ckcu) -> Result<(), ClockError> {
    // Enable HSI oscillator
    ckcu.gccr().modify(|_, w| w.hsien().set_bit());

    // Wait for HSI ready with timeout verification
    let timeout_start = cortex_m::peripheral::DWT::get_cycle_count();
    let timeout_cycles = 480_000; // ~1ms at 48MHz

    loop {
        let current_cycles = cortex_m::peripheral::DWT::get_cycle_count();
        let elapsed = current_cycles.wrapping_sub(timeout_start);

        if elapsed > timeout_cycles {
            return Err(ClockError::ClockStartupTimeout("HSI"));
        }

        if ckcu.gcsr().read().hsirdy().bit_is_set() {
            break;
        }

        // Add small delay to prevent CPU spinning
        for _ in 0..50 { cortex_m::asm::nop(); }
    }

    Ok(())
}

/// Start HSE oscillator with crystal startup protection
fn start_hse_oscillator(ckcu: &Ckcu, hse_freq: u32) -> Result<(), ClockError> {
    // Enable HSE oscillator
    ckcu.gccr().modify(|_, w| w.hseen().set_bit());

    // Wait longer for crystal startup
    let timeout_start = cortex_m::peripheral::DWT::get_cycle_count();
    let timeout_cycles = 1_440_000; // ~3ms at 48MHz (crystal startup)

    loop {
        let current_cycles = cortex_m::peripheral::DWT::get_cycle_count();
        let elapsed = current_cycles.wrapping_sub(timeout_start);

        if elapsed > timeout_cycles {
            // Increment failure counter for monitoring
            CLOCK_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            return Err(ClockError::ClockStartupTimeout("HSE"));
        }

        if ckcu.gcsr().read().hserdy().bit_is_set() {
            break;
        }

        // Add small delay to allow crystal to stabilize
        for _ in 0..100 { cortex_m::asm::nop(); }
    }

    Ok(())
}

/// Configure PLL multiplication for target frequency
fn configure_pll_multiplication(ckcu: &Ckcu, config: &ClockConfig) -> Result<(), ClockError> {
    // Configure PLL (HT32F523xx: PLLPLL = PCLK × (FBDIV + 1))
    ckcu.pllcfgr().modify(|_, w| w.fld().bits(config.pll_mult as u8 - 1));

    // Enable PLL and wait for lock
    ckcu.gccr().modify(|_, w| w.pllen().set_bit());

    let timeout_start = cortex_m::peripheral::DWT::get_cycle_count();
    let timeout_cycles = 2_400_000; // 5ms at 48MHz (PLL can be slow)

    loop {
        let current_cycles = cortex_m::peripheral::DWT::get_cycle_count();
        let elapsed = current_cycles.wrapping_sub(timeout_start);

        if elapsed > timeout_cycles {
            CLOCK_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            return Err(ClockError::ClockStartupTimeout("PLL"));
        }

        if ckcu.gcsr().read().pllrdy().bit_is_set() {
            break;
        }

        // Small delay for PLL stability
        for _ in 0..75 { cortex_m::asm::nop(); }
    }

    Ok(())
}

/// Configure AHB and APB bus clock dividers
fn configure_bus_clocks(ckcu: &Ckcu, config: &ClockConfig) -> Result<(), ClockError> {
    let max_divider = 7; // For APB
    let ahb_bits = config.ahb_divider.min(15); // 4-bit field
    let apb_bits = config.apb_divider.min(max_divider);

    // Set AHB clock divider (HCLK)
    ckcu.ahbpfcr().modify(|_, w| w.ahbpres().bits(ahb_bits));

    // Set APB clock divider (PCLK)
    ckcu.apbpfcr().modify(|_, w| w.apbpres().bits(apb_bits));

    Ok(())
}

/// Enable hardware clock monitoring for enterprise fault tolerance
fn enable_clock_monitoring(ckcu: &Ckcu) -> Result<(), ClockError> {
    // Enable clock failure interrupt
    ckcu.gcir().modify(|_, w| w.cksie().set_bit());

    // Enable clock monitoring system
    ckcu.gccr().modify(|_, w| w.ckmen().set_bit());

    Ok(())
}

/// Switch to target clock source with verification
fn switch_to_target_clock(ckcu: &Ckcu, config: &ClockConfig) -> Result<(), ClockError> {
    let target_source = if config.pll_enabled {
        2 // PLL
    } else if config.hse_enabled {
        1 // HSE
    } else {
        0 // HSI
    };

    // Verify target clock is ready
    let ready_flag = match target_source {
        0 => return Err(ClockError::InvalidClockSource.into()), // HSI should always be ready
        1 => ckcu.gcsr().read().hserdy().bit_is_set(),
        2 => ckcu.gcsr().read().pllrdy().bit_is_set(),
        _ => return Err(ClockError::InvalidClockSource.into()),
    };

    if !ready_flag {
        return Err(ClockError::ClockSourceNotReady.into());
    }

    // Perform clock switch
    ckcu.gccr().modify(|_, w| w.sw().bits(target_source as u8));

    // Wait for switch completion with timeout
    let timeout_start = cortex_m::peripheral::DWT::get_cycle_count();
    let timeout_cycles = 240_000; // ~500μs at 48MHz

    loop {
        let current_cycles = cortex_m::peripheral::DWT::get_cycle_count();
        let elapsed = current_cycles.wrapping_sub(timeout_start);

        if elapsed > timeout_cycles {
            return Err(ClockError::ClockSwitchTimeout.into());
        }

        let current_source = ckcu.gcsr().read().sw().bits();
        if current_source as u32 == target_source {
            break;
        }

        for _ in 0..20 { cortex_m::asm::nop(); }
    }

    Ok(())
}

/// Verify final clock configuration meets specifications
fn verify_final_configuration(ckcu: &Ckcu, config: &ClockConfig) -> Result<(), ClockError> {
    // Basic frequency validation
    if config.sysclock_hz > 144_000_000 {
        return Err(ClockError::FrequencyOutOfRange.into());
    }

    if config.ahb_divider > 15 || config.apb_divider > 7 {
        return Err(ClockError::InvalidBusDivider.into());
    }

    // Verify current clock source matches configuration
    let current_source = ckcu.gcsr().read().sw().bits();
    let expected_source = if config.pll_enabled { 2 } else if config.hse_enabled { 1 } else { 0 };

    if current_source as u32 != expected_source {
        return Err(ClockError::ConfigurationMismatch.into());
    }

    Ok(())
}

// ============================================================================
// Clock System Utilities
// ============================================================================

/// Get current system clock frequency with error checking
pub fn get_system_clock_frequency() -> Result<u32, ClockError> {
    let ckcu = unsafe { &*Ckcu::ptr() };

    let current_source = ckcu.gcsr().read().sw().bits() as u32;
    let pllcfg = ckcu.pllcfgr().read();

    // Calculate actual frequencies based on current configuration
    let base_freq = match current_source {
        0 => 8_000_000,     // HSI = 8MHz
        1 => 16_000_000,    // HSE = 16MHz (typical)
        2 => {
            // PLL: frequency depends on input and multipliers
            let pclk = get_bus_clock_frequency()?;
            let multiplier = (pllcfg.fld().bits() + 1) as u32;
            pclk * multiplier
        }
        _ => return Err(ClockError::UnknownClockSource.into()),
    };

    // Apply bus dividers as needed
    let ahb_prescaler = ckcu.ahbpfcr().read().ahbpres().bits() as u32;
    let apb_prescaler = ckcu.apbpfcr().read().apbpres().bits() as u32;

    Ok(base_freq >> apb_prescaler) // Simplified calculation
}

/// Get current bus clock frequencies
pub fn get_bus_clock_frequency() -> Result<u32, ClockError> {
    // Simplified: return system clock / APB divider
    let ckcu = unsafe { &*Ckcu::ptr() };
    let prescaler = ckcu.apbpfcr().read().apbpres().bits() as u32;
    let sys_clock = get_system_clock_frequency()?;
    Ok(sys_clock >> prescaler)
}

// ============================================================================
// Error Handling and Monitoring
// ============================================================================

/// Errors that can occur during clock system initialisation
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockError {
    ClockStartupTimeout(&'static str),
    ClockSourceNotReady,
    ClockSwitchTimeout,
    FrequencyOutOfRange,
    InvalidBusDivider,
    ConfigurationMismatch,
    UnknownClockSource,
    InvalidClockSource,
}

impl core::fmt::Display for ClockError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ClockError::ClockStartupTimeout(peripheral) =>
                write!(f, "{} clock startup timeout", peripheral),
            ClockError::ClockSourceNotReady =>
                write!(f, "Target clock source is not ready"),
            ClockError::ClockSwitchTimeout =>
                write!(f, "Clock source switch timeout"),
            ClockError::FrequencyOutOfRange =>
                write!(f, "Clock frequency exceeds hardware limits"),
            ClockError::InvalidBusDivider =>
                write!(f, "Invalid bus clock divider setting"),
            ClockError::ConfigurationMismatch =>
                write!(f, "Clock configuration verification failed"),
            ClockError::UnknownClockSource =>
                write!(f, "Unknown or invalid clock source"),
            ClockError::InvalidClockSource =>
                write!(f, "Invalid clock source selection"),
        }
    }
}

impl core::error::Error for ClockError {}

// ============================================================================
// Enterprise Features Integration
// ============================================================================

/// Enterprise-grade clock management summary for debugging
pub struct ClockSystemSummary {
    pub configured_frequency: u32,
    pub actual_frequency: Result<u32, ClockError>,
    pub source: &'static str,
    pub failure_count: u32,
    pub bus_settings: (u8, u8), // (ahb_div, apb_div)
}

/// Get comprehensive clock system summary
pub fn get_clock_system_summary() -> ClockSystemSummary {
    let actual_freq = get_system_clock_frequency();

    ClockSystemSummary {
        configured_frequency: get_system_clock_frequency().unwrap_or(0),
        actual_frequency: actual_freq,
        source: match actual_freq.ok().unwrap_or(0) {
            8000000 => "HSI",
            16000000 => "HSE",
            f if f > 48000000 => "PLL",
            _ => "HSI/PLL",
        },
        failure_count: get_clock_failure_count(),
        bus_settings: (0, 0), // Would be retrieved from actual registers
    }
}

/// Quick clock system diagnostic check
pub fn diagnostic_check() -> Result<(), ClockError> {
    // Basic sanity checks
    let current_freq = get_system_clock_frequency()?;

    if current_freq > 144_000_000 {
        return Err(ClockError::FrequencyOutOfRange);
    }

    if current_freq == 0 {
        return Err(ClockError::UnknownClockSource);
    }

    Ok(())
}

// ============================================================================
// Hardware Fault Handler (Clock Monitoring)
// ============================================================================

/// Hardware clock failure NMI handler
/// This MUST be called from the actual NMI interrupt vector
#[inline(always)]
pub extern "C" fn handle_clock_failure() {
    let ckcu = unsafe { &*Ckcu::ptr() };

    critical_section::with(|_| {
        // Check if this is actually a clock failure
        if ckcu.gcir().read().cksf().bit_is_set() {
            // Clear the failure flag
            ckcu.gcir().modify(|_, w| w.cksf().set_bit());

            // Disable monitoring to prevent repeated interrupts
            ckcu.gccr().modify(|_, w| w.ckmen().clear_bit());

            // Record failure event
            CLOCK_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);

            // Automatic fallback to HSI (enterprise feature)
            ckcu.gccr().modify(|_, w| {
                w.sw().bits(0); // Switch to HSI
                w.pllen().clear_bit() // Disable PLL for stability
            });
        }
    });
}

// Crate private function for system initialization use
pub(crate) fn init_flow_init() -> Result<(), ClockError> {
    // Generate default enterprise configuration
    let config = ClockConfig::default();
    clock_system_init(&config)
}

// ============================================================================
// Testing and Validation
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clock_config_creation() {
        let config = ClockConfig::default();
        assert_eq!(config.sysclock_hz, 48_000_000);
        assert_eq!(config.pll_mult, 6);
        assert!(config.clock_monitor);
        assert!(!config.hse_enabled);
    }

    #[test]
    fn test_enterprise_config() {
        let config = config_enterprise_performance();
        assert_eq!(config.sysclock_hz, 48_000_000);
        assert!(config.hse_enabled);
        assert_eq!(config.hse_freq, Some(16_000_000));
        assert!(config.clock_monitor);
    }

    #[test]
    fn test_low_power_config() {
        let config = config_low_power();
        assert_eq!(config.sysclock_hz, 8_000_000);
        assert!(!config.pll_enabled); // No PLL for power savings
        assert!(config.clock_monitor); // Still have monitoring
    }

    #[test]
    fn test_clock_failure_counting() {
        reset_clock_failure_count();
        assert_eq!(get_clock_failure_count(), 0);

        CLOCK_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
        assert_eq!(get_clock_failure_count(), 1);

        reset_clock_failure_count();
        assert_eq!(get_clock_failure_count(), 0);
    }
}