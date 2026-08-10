//! Compatibility clock-management facade for HT32F52342/52.
//!
//! The actual clock tree is configured by [`crate::rcc`]. This module keeps
//! the older time-system API, but validates it against the 48 MHz CKCU and
//! reports the clock source from CKST rather than the GCCR request bits.

use core::sync::atomic::{AtomicU32, Ordering};

use crate::pac::Ckcu;
use crate::time::Hertz;

const HSI_HZ: u32 = 8_000_000;
const ESK32_HSE_HZ: u32 = 8_000_000;
const MAX_SYSCLK_HZ: u32 = 48_000_000;

static CLOCK_FAILURE_COUNT: AtomicU32 = AtomicU32::new(0);

/// Legacy clock configuration used by the enhanced time API.
#[derive(Debug, Clone, Copy)]
pub struct ClockConfig {
    pub sysclock_hz: u32,
    pub hse_enabled: bool,
    /// HSE frequency. The ESK32-30501 crystal is 8 MHz; the MCU accepts 4..16 MHz.
    pub hse_freq: Option<u32>,
    pub pll_enabled: bool,
    /// Direct PLL feedback multiplier for the legacy configurations in this API.
    pub pll_mult: u32,
    /// Enable the HSE clock monitor. This is valid only when HSE is running.
    pub clock_monitor: bool,
    /// AHBPRE encoding: 0=/1, 1=/2, 2=/4, 3=/8, 4=/16, 5=/32.
    pub ahb_divider: u8,
    /// Kept for API compatibility. HT32 has per-peripheral PCLK prescalers,
    /// not one global APB divider, so this must be zero.
    pub apb_divider: u8,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            sysclock_hz: MAX_SYSCLK_HZ,
            hse_enabled: cfg!(feature = "usb"),
            hse_freq: cfg!(feature = "usb").then_some(ESK32_HSE_HZ),
            pll_enabled: true,
            pll_mult: 6,
            clock_monitor: cfg!(feature = "usb"),
            ahb_divider: 0,
            apb_divider: 0,
        }
    }
}

/// Accurate high-performance configuration for ESK32-30501.
pub fn config_enterprise_performance() -> ClockConfig {
    ClockConfig {
        sysclock_hz: MAX_SYSCLK_HZ,
        hse_enabled: true,
        hse_freq: Some(ESK32_HSE_HZ),
        pll_enabled: true,
        pll_mult: 6,
        clock_monitor: true,
        ahb_divider: 0,
        apb_divider: 0,
    }
}

pub fn config_low_power() -> ClockConfig {
    ClockConfig {
        sysclock_hz: HSI_HZ,
        hse_enabled: false,
        hse_freq: None,
        pll_enabled: false,
        pll_mult: 1,
        clock_monitor: false,
        ahb_divider: 0,
        apb_divider: 0,
    }
}

pub fn get_clock_failure_count() -> u32 {
    CLOCK_FAILURE_COUNT.load(Ordering::Relaxed)
}

pub fn reset_clock_failure_count() {
    CLOCK_FAILURE_COUNT.store(0, Ordering::Relaxed);
}

pub fn clock_system_init(config: &ClockConfig) -> Result<(), ClockError> {
    validate_config(config)?;

    let ahb_divisor = 1u32 << config.ahb_divider;
    let ahb_hz = config.sysclock_hz / ahb_divisor;
    crate::rcc::init(crate::rcc::Config {
        sys_clk: Some(Hertz::hz(config.sysclock_hz)),
        ahb_clk: Some(Hertz::hz(ahb_hz)),
        apb_clk: Some(Hertz::hz(ahb_hz)),
        use_hse: config.hse_enabled,
        hse_freq: config.hse_freq.map(Hertz::hz),
    });

    let ckcu = unsafe { &*Ckcu::ptr() };
    ckcu.gcir()
        .modify(|_, w| w.cksie().bit(config.clock_monitor));
    ckcu.gccr()
        .modify(|_, w| w.ckmen().bit(config.clock_monitor));
    verify_final_configuration(config)
}

fn validate_config(config: &ClockConfig) -> Result<(), ClockError> {
    if config.sysclock_hz == 0 || config.sysclock_hz > MAX_SYSCLK_HZ {
        return Err(ClockError::FrequencyOutOfRange);
    }
    if cfg!(feature = "usb")
        && (!config.hse_enabled || !config.pll_enabled || config.sysclock_hz != MAX_SYSCLK_HZ)
    {
        return Err(ClockError::InvalidClockSource);
    }
    if config.ahb_divider > 5 || config.apb_divider != 0 {
        return Err(ClockError::InvalidBusDivider);
    }

    let input_hz = if config.hse_enabled {
        let hse = config.hse_freq.ok_or(ClockError::InvalidClockSource)?;
        if !(4_000_000..=16_000_000).contains(&hse) {
            return Err(ClockError::FrequencyOutOfRange);
        }
        hse
    } else {
        if config.hse_freq.is_some() || config.clock_monitor {
            return Err(ClockError::InvalidClockSource);
        }
        HSI_HZ
    };

    if config.pll_enabled {
        if !(1..=16).contains(&config.pll_mult)
            || input_hz.saturating_mul(config.pll_mult) != config.sysclock_hz
        {
            return Err(ClockError::ConfigurationMismatch);
        }
    } else if config.pll_mult != 1 || input_hz != config.sysclock_hz {
        return Err(ClockError::ConfigurationMismatch);
    }

    Ok(())
}

fn verify_final_configuration(config: &ClockConfig) -> Result<(), ClockError> {
    if get_system_clock_frequency()? != config.sysclock_hz {
        return Err(ClockError::ConfigurationMismatch);
    }

    let status = current_source_status();
    let expected = if config.pll_enabled {
        1
    } else if config.hse_enabled {
        2
    } else {
        3
    };
    if !clock_status_matches(status, expected) {
        return Err(ClockError::ConfigurationMismatch);
    }
    Ok(())
}

fn current_source_status() -> u8 {
    unsafe { &*Ckcu::ptr() }.ckst().read().ckswst().bits()
}

fn clock_status_matches(status: u8, source: u8) -> bool {
    if source == 1 {
        status & 0b110 == 0
    } else {
        status == source
    }
}

pub fn get_system_clock_frequency() -> Result<u32, ClockError> {
    let frequency = crate::rcc::get_clocks().sys_clk().to_hz();
    if frequency == 0 || frequency > MAX_SYSCLK_HZ {
        Err(ClockError::FrequencyOutOfRange)
    } else {
        Ok(frequency)
    }
}

pub fn get_bus_clock_frequency() -> Result<u32, ClockError> {
    let frequency = crate::rcc::get_clocks().ahb_clk().to_hz();
    if frequency == 0 || frequency > MAX_SYSCLK_HZ {
        Err(ClockError::FrequencyOutOfRange)
    } else {
        Ok(frequency)
    }
}

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
            Self::ClockStartupTimeout(source) => write!(f, "{source} clock startup timeout"),
            Self::ClockSourceNotReady => write!(f, "target clock source is not ready"),
            Self::ClockSwitchTimeout => write!(f, "clock source switch timeout"),
            Self::FrequencyOutOfRange => write!(f, "clock frequency is outside the 48 MHz limit"),
            Self::InvalidBusDivider => write!(f, "invalid bus clock divider"),
            Self::ConfigurationMismatch => write!(f, "clock configuration mismatch"),
            Self::UnknownClockSource => write!(f, "unknown clock source"),
            Self::InvalidClockSource => write!(f, "invalid clock source configuration"),
        }
    }
}

impl core::error::Error for ClockError {}

pub struct ClockSystemSummary {
    pub configured_frequency: u32,
    pub actual_frequency: Result<u32, ClockError>,
    pub source: &'static str,
    pub failure_count: u32,
    pub bus_settings: (u8, u8),
}

pub fn get_clock_system_summary() -> ClockSystemSummary {
    let ckcu = unsafe { &*Ckcu::ptr() };
    let ahb = ckcu.ahbcfgr().read().ahbpre().bits();
    let source = match current_source_status() {
        0 | 1 => "PLL",
        2 => "HSE",
        3 => "HSI",
        6 => "LSE",
        7 => "LSI",
        _ => "Unknown",
    };
    let actual_frequency = get_system_clock_frequency();

    ClockSystemSummary {
        configured_frequency: actual_frequency.unwrap_or(0),
        actual_frequency,
        source,
        failure_count: get_clock_failure_count(),
        bus_settings: (ahb, 0),
    }
}

pub fn diagnostic_check() -> Result<(), ClockError> {
    let frequency = get_system_clock_frequency()?;
    if !(1_000_000..=MAX_SYSCLK_HZ).contains(&frequency) {
        return Err(ClockError::FrequencyOutOfRange);
    }
    Ok(())
}

/// HSE clock-failure NMI fallback.
#[inline(always)]
pub extern "C" fn handle_clock_failure() {
    let ckcu = unsafe { &*Ckcu::ptr() };
    if !ckcu.gcir().read().cksf().bit_is_set() {
        return;
    }

    let failures = CLOCK_FAILURE_COUNT.load(Ordering::Relaxed);
    CLOCK_FAILURE_COUNT.store(failures.saturating_add(1), Ordering::Relaxed);
    ckcu.gccr().modify(|_, w| w.ckmen().clear_bit());
    // CKSF is W1C. A direct write clears it and disables further CKSIE NMIs.
    unsafe { ckcu.gcir().write(|w| w.bits(1)) };
    ckcu.gccr().modify(|_, w| w.sw().variant(3));

    // CKST, not GCCR.SW, confirms the HSI fallback is active. Bound the wait
    // because this function executes in NMI context.
    for _ in 0..100_000 {
        if clock_status_matches(current_source_status(), 3) {
            ckcu.gccr().modify(|_, w| w.pllen().clear_bit());
            return;
        }
        core::hint::spin_loop();
    }
}

pub(crate) fn init_flow_init() -> Result<(), ClockError> {
    clock_system_init(&ClockConfig::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn board_performance_config_uses_8mhz_hse() {
        let config = config_enterprise_performance();
        assert_eq!(config.hse_freq, Some(8_000_000));
        assert_eq!(config.pll_mult, 6);
        assert_eq!(validate_config(&config), Ok(()));
    }

    #[test]
    fn rejects_clock_above_device_limit() {
        let mut config = ClockConfig::default();
        config.sysclock_hz = 144_000_000;
        assert_eq!(
            validate_config(&config),
            Err(ClockError::FrequencyOutOfRange)
        );
    }
}
