//! Reset and Clock Control (RCC/CKCU) driver
//!
//! HT32 uses CKCU (Clock Control Unit) instead of RCC, but we maintain RCC naming for consistency

use crate::pac::Ckcu;
use crate::time::Hertz;

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
            sys_clk: Some(Hertz::mhz(48)),  // Default to 48MHz
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
    });

    // Enable AFIO clock (AFIO is on APB bus)
    ckcu.apbccr0().modify(|_, w| {
        w.afioen().set_bit() // Enable AFIO
    });
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