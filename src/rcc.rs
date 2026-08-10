//! Reset and Clock Control (RCC/CKCU) driver
//!
//! HT32 uses CKCU (Clock Control Unit) instead of RCC, but we maintain RCC naming for consistency

#[cfg(feature = "usb")]
use crate::pac::Pwrcu;
use crate::pac::{Ckcu, Fmc};
use crate::time::Hertz;

// Use defmt logging when available
#[cfg(feature = "defmt")]
use defmt::info;

#[cfg(not(feature = "defmt"))]
macro_rules! info {
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
            sys_clk: Some(Hertz::mhz(48)),
            ahb_clk: None,
            apb_clk: None,
            // USB full-speed cannot use the ±2% HSI as its 48 MHz source.
            // The supported HT32F52352 board carries an 8 MHz crystal.
            use_hse: cfg!(feature = "usb"),
            hse_freq: if cfg!(feature = "usb") {
                Some(Hertz::mhz(8))
            } else {
                None
            },
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

    #[cfg(feature = "usb")]
    enable_usb_backup_domain(ckcu);

    // Configure system clock based on config
    let sys_freq = config.sys_clk.unwrap_or(Hertz::mhz(8)); // Default HSI freq

    #[cfg(feature = "usb")]
    {
        assert!(config.use_hse, "USB requires the external HSE oscillator");
        assert_eq!(
            sys_freq.to_hz(),
            48_000_000,
            "USB requires a 48 MHz PLL system clock"
        );
    }

    let clocks = if config.use_hse && config.hse_freq.is_some() {
        configure_hse_clock(
            ckcu,
            config.hse_freq.unwrap(),
            sys_freq,
            config.ahb_clk,
            config.apb_clk,
        )
    } else {
        configure_hsi_clock(ckcu, sys_freq, config.ahb_clk, config.apb_clk)
    };

    // Store clocks globally for later access
    unsafe {
        CLOCKS = Some(clocks);
    }

    // Enable GPIO clocks by default
    enable_gpio_clocks(ckcu);

    // Configure USB clock divider if needed
    #[cfg(feature = "usb")]
    configure_usb_clock(ckcu, clocks.sys_clk);

    clocks
}

/// Get the current clock configuration
pub fn get_clocks() -> Clocks {
    unsafe {
        CLOCKS.unwrap_or_else(|| {
            // Return default HSI clocks if not initialized
            Clocks {
                sys_clk: Hertz::mhz(8),
                ahb_clk: Hertz::mhz(8),
                apb_clk: Hertz::mhz(8),
                hse_clk: None,
            }
        })
    }
}

fn configure_hsi_clock(
    ckcu: &crate::pac::ckcu::RegisterBlock,
    target_freq: Hertz,
    ahb_clk: Option<Hertz>,
    apb_clk: Option<Hertz>,
) -> Clocks {
    // Enable HSI (High Speed Internal oscillator) first
    ckcu.gccr().modify(|_, w| w.hsien().set_bit());

    // Wait for HSI to be ready
    wait_until(
        || ckcu.gcsr().read().hsirdy().bit_is_set(),
        "HSI oscillator did not become ready",
    );

    // Configure PLL if target frequency is higher than HSI
    let sys_clk = if target_freq.to_hz() > 8_000_000 {
        configure_pll_from_hsi(ckcu, target_freq)
    } else {
        assert_eq!(target_freq.to_hz(), 8_000_000, "direct HSI clock is 8 MHz");
        configure_flash_wait_states(8_000_000);
        // GCCR.SW: PLL=1, HSE=2, HSI=3.
        ckcu.gccr().modify(|_, w| w.sw().variant(3));
        wait_until(
            || system_clock_is(ckcu, 3),
            "failed to switch system clock to HSI",
        );
        Hertz::mhz(8) // HSI frequency
    };

    configure_bus_clocks(ckcu, sys_clk, ahb_clk, apb_clk, None)
}

fn configure_hse_clock(
    ckcu: &crate::pac::ckcu::RegisterBlock,
    hse_freq: Hertz,
    target_freq: Hertz,
    ahb_clk: Option<Hertz>,
    apb_clk: Option<Hertz>,
) -> Clocks {
    // Enable HSE (High Speed External oscillator)
    ckcu.gccr().modify(|_, w| w.hseen().set_bit());

    // Wait for HSE to be ready
    wait_until(
        || ckcu.gcsr().read().hserdy().bit_is_set(),
        "HSE oscillator did not become ready",
    );

    // Configure PLL from HSE if needed
    let sys_clk = if target_freq.to_hz() > hse_freq.to_hz() {
        configure_pll_from_hse(ckcu, hse_freq, target_freq)
    } else {
        // Use HSE directly
        assert_eq!(target_freq.to_hz(), hse_freq.to_hz());
        configure_flash_wait_states(hse_freq.to_hz());
        ckcu.gccr().modify(|_, w| w.sw().variant(2));
        wait_until(
            || system_clock_is(ckcu, 2),
            "failed to switch system clock to HSE",
        );
        hse_freq
    };

    configure_bus_clocks(ckcu, sys_clk, ahb_clk, apb_clk, Some(hse_freq))
}

fn configure_pll_from_hsi(ckcu: &crate::pac::ckcu::RegisterBlock, target_freq: Hertz) -> Hertz {
    // HSI = 8MHz as input to PLL
    let hsi_freq = 8_000_000u32;
    let target = target_freq.to_hz();

    // HT32F523xx PLL formula: PLL_Output = Input_Freq * PFBD / (2^POTD)
    // PFBD: 4-bit feedback divider (0 encodes 16, 1..15 encode themselves)
    // POTD: 2-bit output divider (0-3, representing 2^0 to 2^3 = 1,2,4,8 divider)

    let (pfbd, potd) = calculate_pll_params_ht32(hsi_freq, target);

    // GCFGR.PLLSRC: 1 = HSI, 0 = HSE.
    ckcu.gcfgr().modify(|_, w| w.pllsrc().set_bit());

    // Configure PLL
    ckcu.pllcfgr().modify(|_, w| unsafe {
        w.pfbd()
            .bits(pfbd) // Feedback divider (4 bits)
            .potd()
            .bits(potd) // Output divider (2 bits)
    });

    // Enable PLL
    ckcu.gccr().modify(|_, w| w.pllen().set_bit());

    // Wait for PLL to be ready
    wait_until(
        || ckcu.gcsr().read().pllrdy().bit_is_set(),
        "PLL did not lock",
    );

    let multiplier = if pfbd == 0 { 16 } else { pfbd as u32 };
    let actual_freq = hsi_freq * multiplier / (1u32 << potd as u32);
    configure_flash_wait_states(actual_freq);

    // Switch to PLL as system clock
    ckcu.gccr().modify(|_, w| w.sw().variant(1));
    wait_until(
        || system_clock_is(ckcu, 1),
        "failed to switch system clock to PLL",
    );

    Hertz::hz(actual_freq)
}

fn configure_pll_from_hse(
    ckcu: &crate::pac::ckcu::RegisterBlock,
    hse_freq: Hertz,
    target_freq: Hertz,
) -> Hertz {
    // Similar to HSI but using HSE as input
    let hse_hz = hse_freq.to_hz();
    let target = target_freq.to_hz();

    let (pfbd, potd) = calculate_pll_params_ht32(hse_hz, target);

    // GCFGR.PLLSRC: 1 = HSI, 0 = HSE.
    ckcu.gcfgr().modify(|_, w| w.pllsrc().clear_bit());

    // Configure PLL with HSE as source
    ckcu.pllcfgr().modify(|_, w| unsafe {
        w.pfbd()
            .bits(pfbd) // Feedback divider (4 bits)
            .potd()
            .bits(potd) // Output divider (2 bits)
    });

    // Enable PLL
    ckcu.gccr().modify(|_, w| w.pllen().set_bit());

    // Wait for PLL to be ready
    wait_until(
        || ckcu.gcsr().read().pllrdy().bit_is_set(),
        "PLL did not lock",
    );

    let multiplier = if pfbd == 0 { 16 } else { pfbd as u32 };
    let actual_freq = hse_hz * multiplier / (1u32 << potd as u32);
    configure_flash_wait_states(actual_freq);

    // Switch to PLL as system clock
    ckcu.gccr().modify(|_, w| w.sw().variant(1));
    wait_until(
        || system_clock_is(ckcu, 1),
        "failed to switch system clock to PLL",
    );

    Hertz::hz(actual_freq)
}

fn calculate_pll_params_ht32(input_freq: u32, target_freq: u32) -> (u8, u8) {
    // HT32F523xx PLL calculation: Output = Input * PFBD / (2^POTD)
    // PFBD: 0 encodes multiplier 16, 1..15 encode themselves.
    // POTD: 0-3 (representing divider 1,2,4,8)
    let mut best_error = u32::MAX;
    let mut best_pfbd = 1;
    let mut best_potd = 0;

    for potd in 0..=3u8 {
        let divisor = 1u32 << potd;
        for multiplier in 1..=16u32 {
            let pfbd = if multiplier == 16 {
                0
            } else {
                multiplier as u8
            };
            let output_freq = input_freq * multiplier / divisor;

            if output_freq > 48_000_000 {
                continue;
            }

            // HT32F523xx VCO runs at twice the pre-POTD PLL output.
            let vco_freq = input_freq * multiplier * 2;
            if !(48_000_000..=96_000_000).contains(&vco_freq) {
                continue;
            }

            let error = output_freq.abs_diff(target_freq);

            if error < best_error {
                best_error = error;
                best_pfbd = pfbd;
                best_potd = potd;
            }

            if error == 0 {
                return (pfbd, potd);
            }
        }
    }

    (best_pfbd, best_potd)
}

fn configure_bus_clocks(
    ckcu: &crate::pac::ckcu::RegisterBlock,
    sys_clk: Hertz,
    requested_ahb: Option<Hertz>,
    requested_apb: Option<Hertz>,
    hse_clk: Option<Hertz>,
) -> Clocks {
    let requested_ahb = requested_ahb.unwrap_or(sys_clk).to_hz();
    let sys_hz = sys_clk.to_hz();
    assert!(requested_ahb != 0 && sys_hz.is_multiple_of(requested_ahb));
    let divisor = sys_hz / requested_ahb;
    let prescaler = match divisor {
        1 => 0,
        2 => 1,
        4 => 2,
        8 => 3,
        16 => 4,
        32 => 5,
        _ => panic!("AHB clock only supports system clock divisors 1, 2, 4, 8, 16, or 32"),
    };
    ckcu.ahbcfgr()
        .modify(|_, w| unsafe { w.ahbpre().bits(prescaler) });

    // HT32F523xx has no independent APB prescaler: CK_APB follows CK_AHB.
    let ahb_clk = Hertz::hz(sys_hz / divisor);
    let apb_clk = requested_apb.unwrap_or(ahb_clk);
    assert_eq!(
        apb_clk.to_hz(),
        ahb_clk.to_hz(),
        "APB clock must equal AHB clock on HT32F523xx"
    );

    Clocks {
        sys_clk,
        ahb_clk,
        apb_clk,
        hse_clk,
    }
}

fn wait_until(mut ready: impl FnMut() -> bool, failure: &'static str) {
    const CLOCK_STARTUP_TIMEOUT: u32 = 10_000_000;
    for _ in 0..CLOCK_STARTUP_TIMEOUT {
        if ready() {
            return;
        }
        core::hint::spin_loop();
    }
    panic!("{failure}");
}

/// GCCR.SW is only a clock-source request. CKST.CKSWST reports the source
/// actually driving CK_SYS after the hardware switching delay.
fn system_clock_is(ckcu: &crate::pac::ckcu::RegisterBlock, source: u8) -> bool {
    let status = ckcu.ckst().read().ckswst().bits();
    if source == 1 {
        // Both 000 and 001 select/report CK_PLL (documented as 00x).
        status & 0b110 == 0
    } else {
        status == source
    }
}

fn configure_flash_wait_states(frequency: u32) {
    let fmc = unsafe { &*Fmc::ptr() };
    // CFCR.WAIT is encoded as wait cycles + 1: 001 means zero wait states,
    // 010 means one. All other encodings are reserved on HT32F52342/52.
    let wait_encoding = if frequency > 24_000_000 { 2 } else { 1 };
    let prefetch_enabled = fmc.cfcr().read().pfbe().bit_is_set();

    // The user manual requires prefetch to be disabled while WAIT changes.
    if prefetch_enabled {
        fmc.cfcr().modify(|_, w| w.pfbe().clear_bit());
    }
    fmc.cfcr()
        .modify(|_, w| unsafe { w.wait().bits(wait_encoding) });
    if prefetch_enabled {
        fmc.cfcr().modify(|_, w| w.pfbe().set_bit());
    }
}

#[cfg(feature = "usb")]
fn enable_usb_backup_domain(ckcu: &crate::pac::ckcu::RegisterBlock) {
    // The HT32 USB block depends on the backup power domain. This sequence is
    // part of Holtek's low-level initialization and must precede USB clocking.
    ckcu.lpcr().modify(|_, w| w.bkiso().set_bit());
    ckcu.apbccr1().modify(|_, w| w.bkpren().set_bit());

    let pwrcu = unsafe { &*Pwrcu::ptr() };
    wait_until(
        || pwrcu.pwrcu_baktest().read().baktest().bits() == 0x27,
        "USB backup power domain did not become ready",
    );
}

fn enable_gpio_clocks(ckcu: &crate::pac::ckcu::RegisterBlock) {
    // Enable GPIO clocks (GPIO are on AHB bus)
    ckcu.ahbccr().modify(|_, w| {
        w.paen()
            .set_bit() // Enable GPIOA
            .pben()
            .set_bit() // Enable GPIOB
            .pcen()
            .set_bit() // Enable GPIOC
            .pden()
            .set_bit() // Enable GPIOD
    });

    #[cfg(feature = "usb")]
    ckcu.ahbccr().modify(|_, w| w.usben().set_bit());

    // Enable AFIO clock (AFIO is on APB bus)
    ckcu.apbccr0().modify(|_, w| {
        w.afioen().set_bit() // Enable AFIO
    });

    // Enable timer clocks (Timers are on APB bus)
    ckcu.apbccr1().modify(|_, w| {
        w.gptm0en()
            .set_bit() // Enable GPTM0 for embassy-time
            .gptm1en()
            .set_bit() // Enable GPTM1
    });
}

/// Configure USB clock divider to ensure exactly 48MHz USB clock
///
/// CRITICAL: USB requires EXACTLY 48MHz (±0.25%) for proper operation
/// HT32 uses PLL output divided by USB prescaler: USB_CLK = PLL_CLK / USB_PRESCALER
///
/// Supported board configuration: 8 MHz HSE, 48 MHz PLL, USBPRE=0.
fn configure_usb_clock(ckcu: &crate::pac::ckcu::RegisterBlock, sys_clk: Hertz) {
    // USB requires EXACTLY 48MHz clock (±0.25% tolerance)
    const USB_TARGET_FREQ: u32 = 48_000_000;

    assert_eq!(sys_clk.to_hz(), USB_TARGET_FREQ);
    assert!(
        ckcu.gcsr().read().pllrdy().bit_is_set(),
        "USB clock source PLL is not locked"
    );
    assert_eq!(
        ckcu.ckst().read().ckswst().bits() & 0b110,
        0,
        "USB requires PLL as the active system clock"
    );

    // USBPRE=0 selects CK_PLL without division: 48 MHz / 1 = 48 MHz.
    ckcu.gcfgr().modify(|_, w| unsafe { w.usbpre().bits(0) });
    assert_eq!(ckcu.gcfgr().read().usbpre().bits(), 0);
    info!("USB clock configured from 48 MHz PLL");
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
