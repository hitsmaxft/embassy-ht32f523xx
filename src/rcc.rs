//! Reset and Clock Control (RCC/CKCU) driver
//!
//! HT32 uses CKCU (Clock Control Unit) instead of RCC, but we maintain RCC naming for consistency

use crate::pac::Ckcu;
use crate::time::Hertz;

// Use defmt logging when available
#[cfg(feature = "defmt")]
use defmt::{debug, error, info, warn};

#[cfg(not(feature = "defmt"))]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! error {
    ($($arg:tt)*) => {};
}

/// Clock configuration
pub struct Config {
    /// System clock frequency
    pub sys_clk: Option<Hertz>,
    /// AHB clock frequency
    pub ahb_clk: Option<Hertz>,
    /// APB clock frequency
    pub apb_clk: Option<Hertz>,
    /// Use external crystal oscillator
    pub use_hse: bool,
    /// HSE frequency (if used)
    pub hse_freq: Option<Hertz>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            sys_clk: Some(Hertz::mhz(48)),  // Default to 96MHz for USB compatibility (96MHz/2 = 48MHz USB)
            ahb_clk: None,  // Same as sys_clk by default
            apb_clk: None,  // Same as sys_clk by default
            use_hse: false, // Use HSI by default
            hse_freq: None,
        }
    }
}

/// Frozen clock frequencies
#[derive(Clone, Copy, Debug)]
pub struct Clocks {
    pub sys_clk: Hertz,
    pub ahb_clk: Hertz,
    pub apb_clk: Hertz,
    pub hse_clk: Option<Hertz>,
}

impl Clocks {
    /// Get the system clock frequency
    pub fn sys_clk(&self) -> Hertz {
        self.sys_clk
    }

    /// Get the AHB clock frequency
    pub fn ahb_clk(&self) -> Hertz {
        self.ahb_clk
    }

    /// Get the APB clock frequency
    pub fn apb_clk(&self) -> Hertz {
        self.apb_clk
    }
}

static mut CLOCKS: Option<Clocks> = None;

/// Initialize the clock system
pub fn init(config: Config) -> Clocks {
    let ckcu = unsafe { &*Ckcu::ptr() };

    // Configure system clock based on config
    let sys_freq = config.sys_clk.unwrap_or(Hertz::mhz(8)); // Default HSI freq

    let clocks = if config.use_hse && config.hse_freq.is_some() {
        configure_hse_clock(ckcu, config.hse_freq.unwrap(), sys_freq)
    } else {
        configure_hsi_clock(ckcu, sys_freq)
    };

    // Store clocks globally for later access
    unsafe {
        CLOCKS = Some(clocks);
    }

    // Enable GPIO clocks by default
    enable_gpio_clocks(ckcu);

    // Configure USB clock divider if needed
    configure_usb_clock(ckcu, clocks.sys_clk);

    clocks
}

/// Get the current clock configuration
pub fn get_clocks() -> Clocks {
    unsafe { CLOCKS.unwrap_or_else(|| {
        // Return default HSI clocks if not initialized
        Clocks {
            sys_clk: Hertz::mhz(8),
            ahb_clk: Hertz::mhz(8),
            apb_clk: Hertz::mhz(8),
            hse_clk: None,
        }
    })}
}

fn configure_hsi_clock(ckcu: &crate::pac::ckcu::RegisterBlock, target_freq: Hertz) -> Clocks {
    // Enable HSI (High Speed Internal oscillator) first
    ckcu.gccr().modify(|_, w| w.hsien().set_bit());

    // Wait for HSI to be ready
    while !ckcu.gcsr().read().hsirdy().bit_is_set() {}

    // Configure PLL if target frequency is higher than HSI
    let sys_clk = if target_freq.to_hz() > 8_000_000 {
        configure_pll_from_hsi(ckcu, target_freq)
    } else {
        // Use HSI directly - SW field: 0=HSI, 1=HSE, 2=PLL
        ckcu.gccr().modify(|_, w| w.sw().variant(0));
        Hertz::mhz(8) // HSI frequency
    };

    // Configure AHB and APB prescalers
    configure_bus_clocks(ckcu, sys_clk)
}

fn configure_hse_clock(ckcu: &crate::pac::ckcu::RegisterBlock, hse_freq: Hertz, target_freq: Hertz) -> Clocks {
    // Enable HSE (High Speed External oscillator)
    ckcu.gccr().modify(|_, w| w.hseen().set_bit());

    // Wait for HSE to be ready
    while !ckcu.gcsr().read().hserdy().bit_is_set() {}

    // Configure PLL from HSE if needed
    let sys_clk = if target_freq.to_hz() > hse_freq.to_hz() {
        configure_pll_from_hse(ckcu, hse_freq, target_freq)
    } else {
        // Use HSE directly
        ckcu.gccr().modify(|_, w| w.sw().variant(1));
        hse_freq
    };

    configure_bus_clocks(ckcu, sys_clk)
}

fn configure_pll_from_hsi(ckcu: &crate::pac::ckcu::RegisterBlock, target_freq: Hertz) -> Hertz {
    // HSI = 8MHz as input to PLL
    let hsi_freq = 8_000_000u32;
    let target = target_freq.to_hz();

    // HT32F523xx PLL formula: PLL_Output = Input_Freq * ((PFBD + 2) / (2^POTD))
    // PFBD: 4-bit feedback divider (0-15, representing 2-17 multiplier)
    // POTD: 2-bit output divider (0-3, representing 2^0 to 2^3 = 1,2,4,8 divider)

    let (pfbd, potd) = calculate_pll_params_ht32(hsi_freq, target);

    // Configure PLL
    ckcu.pllcfgr().modify(|_, w| unsafe {
        w.pfbd().bits(pfbd)    // Feedback divider (4 bits)
         .potd().bits(potd)    // Output divider (2 bits)
    });

    // Enable PLL
    ckcu.gccr().modify(|_, w| w.pllen().set_bit());

    // Wait for PLL to be ready
    while !ckcu.gcsr().read().pllrdy().bit_is_set() {}

    // Switch to PLL as system clock
    ckcu.gccr().modify(|_, w| w.sw().variant(2));

    // Calculate actual frequency: Input * ((PFBD + 2) / (2^POTD))
    let actual_freq = hsi_freq * (pfbd as u32 + 2) / (1u32 << potd as u32);
    Hertz::hz(actual_freq)
}

fn configure_pll_from_hse(ckcu: &crate::pac::ckcu::RegisterBlock, hse_freq: Hertz, target_freq: Hertz) -> Hertz {
    // Similar to HSI but using HSE as input
    let hse_hz = hse_freq.to_hz();
    let target = target_freq.to_hz();

    let (pfbd, potd) = calculate_pll_params_ht32(hse_hz, target);

    // Configure PLL with HSE as source
    ckcu.pllcfgr().modify(|_, w| unsafe {
        w.pfbd().bits(pfbd)    // Feedback divider (4 bits)
         .potd().bits(potd)    // Output divider (2 bits)
    });

    // Enable PLL
    ckcu.gccr().modify(|_, w| w.pllen().set_bit());

    // Wait for PLL to be ready
    while !ckcu.gcsr().read().pllrdy().bit_is_set() {}

    // Switch to PLL as system clock
    ckcu.gccr().modify(|_, w| w.sw().variant(2));

    // Calculate actual frequency: Input * ((PFBD + 2) / (2^POTD))
    let actual_freq = hse_hz * (pfbd as u32 + 2) / (1u32 << potd as u32);
    Hertz::hz(actual_freq)
}

fn calculate_pll_params_ht32(input_freq: u32, target_freq: u32) -> (u8, u8) {
    // HT32F523xx PLL calculation: Output = Input * ((PFBD + 2) / (2^POTD))
    // PFBD: 0-15 (representing multiplier 2-17)
    // POTD: 0-3 (representing divider 1,2,4,8)

    // USB-COMPATIBLE PRIORITY: For USB operation, we prefer specific frequencies
    // that can divide cleanly to 48MHz USB clock: 48MHz, 72MHz, 96MHz, 144MHz
    const USB_COMPATIBLE_FREQS: &[u32] = &[48_000_000, 72_000_000, 96_000_000, 144_000_000];

    // First, try to hit an exact USB-compatible frequency
    for &usb_freq in USB_COMPATIBLE_FREQS {
        for potd in 0..=3u8 {
            let divisor = 1u32 << potd;
            for pfbd in 0..=15u8 {
                let multiplier = pfbd as u32 + 2;
                let output_freq = input_freq * multiplier / divisor;

                if output_freq == usb_freq {
                    // Ensure VCO frequency is within bounds (relaxed for USB compatibility)
                    let vco_freq = input_freq * multiplier;
                    if vco_freq >= 48_000_000 && vco_freq <= 200_000_000 {
                        info!("🔧 PLL_USB_COMPAT: Found exact USB-compatible {}MHz (PFBD={}, POTD={}, VCO={}MHz)",
                               output_freq / 1_000_000, pfbd, potd, vco_freq / 1_000_000);
                        return (pfbd, potd);
                    }
                }
            }
        }
    }

    // Fallback to original algorithm if no exact USB-compatible frequency found
    let mut best_error = u32::MAX;
    let mut best_pfbd = 6; // Default: 8MHz * ((6+2)/1) = 64MHz, but limited by max freq
    let mut best_potd = 1; // Default: divide by 2 -> 32MHz

    // Try all combinations within reasonable bounds
    for potd in 0..=3u8 {
        let divisor = 1u32 << potd;
        for pfbd in 0..=15u8 {
            let multiplier = pfbd as u32 + 2;
            let output_freq = input_freq * multiplier / divisor;

            // Ensure we don't exceed maximum system clock (usually 60MHz for HT32F523xx)
            if output_freq > 60_000_000 {
                continue;
            }

            // Ensure VCO frequency is within bounds (typically 120-200MHz before final division)
            let vco_freq = input_freq * multiplier;
            if vco_freq < 120_000_000 || vco_freq > 200_000_000 {
                continue;
            }

            let error = if output_freq > target_freq {
                output_freq - target_freq
            } else {
                target_freq - output_freq
            };

            if error < best_error {
                best_error = error;
                best_pfbd = pfbd;
                best_potd = potd;
            }

            // Exact match found
            if error == 0 {
                break;
            }
        }
    }

    (best_pfbd, best_potd)
}

fn configure_bus_clocks(_ckcu: &crate::pac::ckcu::RegisterBlock, sys_clk: Hertz) -> Clocks {
    // For HT32, AHB and APB are typically the same as system clock
    // This can be modified based on specific requirements

    // Configure AHB prescaler (if needed)
    // ckcu.ahbcfgr.modify(|_, w| w.ahbpre().div1());

    // Configure APB prescaler (if needed)
    // ckcu.apbcfgr.modify(|_, w| w.apbpre().div1());

    Clocks {
        sys_clk,
        ahb_clk: sys_clk, // Same as system clock
        apb_clk: sys_clk, // Same as system clock
        hse_clk: None,    // TODO: Track HSE frequency if used
    }
}

fn enable_gpio_clocks(ckcu: &crate::pac::ckcu::RegisterBlock) {
    // Enable GPIO clocks (GPIO are on AHB bus)
    ckcu.ahbccr().modify(|_, w| {
        w.paen().set_bit()  // Enable GPIOA
         .pben().set_bit()  // Enable GPIOB
         .pcen().set_bit()  // Enable GPIOC
         .pden().set_bit()  // Enable GPIOD
         .usben().set_bit() // Enable USB peripheral clock
    });

    // Enable AFIO clock (AFIO is on APB bus)
    ckcu.apbccr0().modify(|_, w| {
        w.afioen().set_bit() // Enable AFIO
    });

    // Enable timer clocks (Timers are on APB bus)
    ckcu.apbccr1().modify(|_, w| {
        w.gptm0en().set_bit() // Enable GPTM0 for embassy-time
         .gptm1en().set_bit() // Enable GPTM1
    });
}

/// Configure USB clock divider to ensure exactly 48MHz USB clock
///
/// CRITICAL: USB requires EXACTLY 48MHz (±0.25%) for proper operation
/// HT32 uses PLL output divided by USB prescaler: USB_CLK = PLL_CLK / USB_PRESCALER
///
/// Recommended configuration based on HT32 documentation:
/// - PLL: 144MHz (8MHz HSE * 18 or equivalent from HSI)
/// - USB_PRESCALER: 3 (144MHz / 3 = 48MHz)
fn configure_usb_clock(ckcu: &crate::pac::ckcu::RegisterBlock, sys_clk: Hertz) {
    // USB requires EXACTLY 48MHz clock (±0.25% tolerance)
    const USB_TARGET_FREQ: u32 = 48_000_000;

    let sys_freq = sys_clk.to_hz();

    // Calculate and validate USB clock configuration
    // We need: SYS_FREQ / USB_PRESCALER = 48MHz
    // USBPRE values: 0=1:1, 1=1.5:1, 2=2:1, 3=2.5:1
    let (usbpre_val, actual_usb_freq) = if sys_freq == 144_000_000 {
        // Ideal case: 144MHz / 3 = 48MHz USB
        (3, USB_TARGET_FREQ)
    } else if sys_freq == 96_000_000 {
        // Alternative: 96MHz / 2 = 48MHz USB
        (2, USB_TARGET_FREQ)
    } else if sys_freq == 72_000_000 {
        // Alternative: 72MHz / 1.5 = 48MHz USB
        (1, USB_TARGET_FREQ)
    } else if sys_freq == USB_TARGET_FREQ {
        // Direct: 48MHz / 1 = 48MHz USB
        (0, USB_TARGET_FREQ)
    } else {
        // WARNING: Unsupported frequency for USB!
        // Try to get closest possible, but enumeration will likely fail
        warn!("⚠️  USB_CLOCK_WARN: System clock {}MHz cannot provide exact 48MHz USB clock", sys_freq / 1_000_000);
        warn!("⚠️  USB_CLOCK_WARN: USB enumeration may fail - consider using 48MHz, 72MHz, 96MHz, or 144MHz system clock");

        // Fallback: get as close as possible
        if sys_freq > 144_000_000 {
            (3, sys_freq / 3)
        } else if sys_freq > 96_000_000 {
            (2, sys_freq / 2)
        } else if sys_freq > 48_000_000 {
            (1, sys_freq * 2 / 3) // Approximate 1.5:1
        } else {
            (0, sys_freq) // Direct, but likely not 48MHz
        }
    };

    // Configure USB prescaler (USBPRE bits 22:23 in GCFGR)
    // USBPRE values: 0=1:1, 1=1.5:1, 2=2:1, 3=2.5:1
    ckcu.gcfgr().modify(|_, w| unsafe {
        w.usbpre().bits(usbpre_val)
    });

    // Log USB clock configuration for debugging
    if actual_usb_freq == USB_TARGET_FREQ {
        info!("🔧 USB_CLOCK: Configured exact 48MHz USB clock (sys: {}MHz, prescaler: {})",
              sys_freq / 1_000_000, usbpre_val);
    } else {
        error!("❌ USB_CLOCK_ERROR: USB clock = {}MHz (target: 48MHz) - enumeration may fail!",
               actual_usb_freq / 1_000_000);
        error!("❌ USB_CLOCK_ERROR: System clock {}MHz with prescaler {} cannot produce 48MHz USB",
               sys_freq / 1_000_000, usbpre_val);
    }
}

/// RCC peripheral handle
pub struct Rcc {
    _private: (),
}

impl Rcc {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    /// Enable peripheral clock
    pub fn enable_peripheral(&self, peripheral: Peripheral) {
        let ckcu = unsafe { &*Ckcu::ptr() };

        match peripheral {
            Peripheral::GPIOA => ckcu.ahbccr().modify(|_, w| w.paen().set_bit()),
            Peripheral::GPIOB => ckcu.ahbccr().modify(|_, w| w.pben().set_bit()),
            Peripheral::GPIOC => ckcu.ahbccr().modify(|_, w| w.pcen().set_bit()),
            Peripheral::GPIOD => ckcu.ahbccr().modify(|_, w| w.pden().set_bit()),
            Peripheral::AFIO => ckcu.apbccr0().modify(|_, w| w.afioen().set_bit()),
            Peripheral::USART0 => ckcu.apbccr0().modify(|_, w| w.usr0en().set_bit()),
            Peripheral::USART1 => ckcu.apbccr0().modify(|_, w| w.usr1en().set_bit()),
            Peripheral::TIM0 => ckcu.apbccr1().modify(|_, w| w.gptm0en().set_bit()),
            Peripheral::TIM1 => ckcu.apbccr1().modify(|_, w| w.gptm1en().set_bit()),
            Peripheral::USB => ckcu.ahbccr().modify(|_, w| w.usben().set_bit()),
        }
    }

    /// Disable peripheral clock
    pub fn disable_peripheral(&self, peripheral: Peripheral) {
        let ckcu = unsafe { &*Ckcu::ptr() };

        match peripheral {
            Peripheral::GPIOA => ckcu.ahbccr().modify(|_, w| w.paen().clear_bit()),
            Peripheral::GPIOB => ckcu.ahbccr().modify(|_, w| w.pben().clear_bit()),
            Peripheral::GPIOC => ckcu.ahbccr().modify(|_, w| w.pcen().clear_bit()),
            Peripheral::GPIOD => ckcu.ahbccr().modify(|_, w| w.pden().clear_bit()),
            Peripheral::AFIO => ckcu.apbccr0().modify(|_, w| w.afioen().clear_bit()),
            Peripheral::USART0 => ckcu.apbccr0().modify(|_, w| w.usr0en().clear_bit()),
            Peripheral::USART1 => ckcu.apbccr0().modify(|_, w| w.usr1en().clear_bit()),
            Peripheral::TIM0 => ckcu.apbccr1().modify(|_, w| w.gptm0en().clear_bit()),
            Peripheral::TIM1 => ckcu.apbccr1().modify(|_, w| w.gptm1en().clear_bit()),
            Peripheral::USB => ckcu.ahbccr().modify(|_, w| w.usben().clear_bit()),
        }
    }

    /// Get current clock frequencies
    pub fn clocks(&self) -> Clocks {
        get_clocks()
    }
}

/// Peripheral enumeration for clock control
#[derive(Debug, Copy, Clone)]
pub enum Peripheral {
    GPIOA,
    GPIOB,
    GPIOC,
    GPIOD,
    AFIO,
    USART0,
    USART1,
    TIM0,
    TIM1,
    USB,
}

/// Extension trait for RCC
pub trait RccExt {
    fn configure(self, config: Config) -> Clocks;
}

impl RccExt for Ckcu {
    fn configure(self, config: Config) -> Clocks {
        init(config)
    }
}
