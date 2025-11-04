//! GPIO (General Purpose Input/Output) driver
//!
//! This module provides GPIO functionality similar to embassy-stm32, adapted for HT32 architecture.

use core::marker::PhantomData;

// Helper macro for GPIO operations
macro_rules! gpio_impl {
    ($port:expr, $pin:expr, $op:ident) => {
        unsafe {
            match $port {
                'A' => {
                    let gpio = &*Gpioa::ptr();
                    gpio_op!(gpio, $pin, $op)
                }
                'B' => {
                    let gpio = &*Gpiob::ptr();
                    gpio_op!(gpio, $pin, $op)
                }
                'C' => {
                    let gpio = &*Gpioc::ptr();
                    gpio_op!(gpio, $pin, $op)
                }
                'D' => {
                    let gpio = &*Gpiod::ptr();
                    gpio_op!(gpio, $pin, $op)
                }
                _ => panic!("Invalid GPIO port"),
            }
        }
    };
}

macro_rules! gpio_op {
    ($gpio:expr, $pin:expr, set_output) => {
        $gpio.dircr().modify(|r, w| {
            let mut val = r.bits();
            val |= 1 << $pin;
            w.bits(val)
        })
    };
    ($gpio:expr, $pin:expr, set_input) => {
        $gpio.dircr().modify(|r, w| {
            let mut val = r.bits();
            val &= !(1 << $pin);
            w.bits(val)
        })
    };
    ($gpio:expr, $pin:expr, set_high) => {
        $gpio.srr().write(|w| w.bits(1 << $pin))
    };
    ($gpio:expr, $pin:expr, set_low) => {
        $gpio.rr().write(|w| w.bits(1 << $pin))
    };
    ($gpio:expr, $pin:expr, read_output) => {
        $gpio.doutr().read().bits() & (1 << $pin) != 0
    };
    ($gpio:expr, $pin:expr, read_input) => {
        $gpio.dinr().read().bits() & (1 << $pin) != 0
    };
    ($gpio:expr, $pin:expr, enable_pullup) => {{
        $gpio.pur().modify(|r, w| w.bits(r.bits() | (1 << $pin)));
        $gpio.pdr().modify(|r, w| w.bits(r.bits() & !(1 << $pin)));
    }};
    ($gpio:expr, $pin:expr, enable_pulldown) => {{
        $gpio.pdr().modify(|r, w| w.bits(r.bits() | (1 << $pin)));
        $gpio.pur().modify(|r, w| w.bits(r.bits() & !(1 << $pin)));
    }};
    ($gpio:expr, $pin:expr, disable_pull) => {{
        $gpio.pur().modify(|r, w| w.bits(r.bits() & !(1 << $pin)));
        $gpio.pdr().modify(|r, w| w.bits(r.bits() & !(1 << $pin)));
    }};
}
use embedded_hal::digital::{ErrorType, InputPin, OutputPin, StatefulOutputPin};
use crate::pac::{Gpioa, Gpiob, Gpioc, Gpiod, Afio};
use crate::exti::{ExtiChannel, Edge};

/// GPIO error type
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct GpioError;

impl embedded_hal::digital::Error for GpioError {
    fn kind(&self) -> embedded_hal::digital::ErrorKind {
        embedded_hal::digital::ErrorKind::Other
    }
}

/// GPIO pin levels
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Level {
    Low,
    High,
}

impl From<Level> for bool {
    fn from(level: Level) -> bool {
        match level {
            Level::Low => false,
            Level::High => true,
        }
    }
}

impl From<bool> for Level {
    fn from(value: bool) -> Self {
        if value {
            Level::High
        } else {
            Level::Low
        }
    }
}

/// GPIO pull configuration
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Pull {
    None,
    Up,
    Down,
}

/// GPIO output speed
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Speed {
    Low,
    Medium,
    High,
    VeryHigh,
}

/// GPIO pin modes
pub mod mode {

    pub struct Input;
    pub struct Output;
    pub struct Analog;

    pub struct AlternateFunction<const N: u8>;

    // Type aliases for common AF modes
    pub type AF0 = AlternateFunction<0>;
    pub type AF1 = AlternateFunction<1>;
    pub type AF2 = AlternateFunction<2>;
    pub type AF3 = AlternateFunction<3>;
    pub type AF4 = AlternateFunction<4>;
    pub type AF5 = AlternateFunction<5>;
    pub type AF6 = AlternateFunction<6>;
    pub type AF7 = AlternateFunction<7>;
}

/// GPIO pin
pub struct Pin<const PORT: char, const PIN: u8, MODE> {
    _mode: PhantomData<MODE>,
}

// Type aliases for specific pins - GPIOA
pub type PA0 = Pin<'A', 0, mode::Input>;
pub type PA1 = Pin<'A', 1, mode::Input>;
pub type PA2 = Pin<'A', 2, mode::Input>;
pub type PA3 = Pin<'A', 3, mode::Input>;
pub type PA4 = Pin<'A', 4, mode::Input>;
pub type PA5 = Pin<'A', 5, mode::Input>;
pub type PA6 = Pin<'A', 6, mode::Input>;
pub type PA7 = Pin<'A', 7, mode::Input>;
pub type PA8 = Pin<'A', 8, mode::Input>;
pub type PA9 = Pin<'A', 9, mode::Input>;
pub type PA10 = Pin<'A', 10, mode::Input>;
pub type PA11 = Pin<'A', 11, mode::Input>;
pub type PA12 = Pin<'A', 12, mode::Input>;
pub type PA13 = Pin<'A', 13, mode::Input>;
pub type PA14 = Pin<'A', 14, mode::Input>;
pub type PA15 = Pin<'A', 15, mode::Input>;

// GPIOB pins
pub type PB0 = Pin<'B', 0, mode::Input>;
pub type PB1 = Pin<'B', 1, mode::Input>;
pub type PB2 = Pin<'B', 2, mode::Input>;
pub type PB3 = Pin<'B', 3, mode::Input>;
pub type PB4 = Pin<'B', 4, mode::Input>;
pub type PB5 = Pin<'B', 5, mode::Input>;
pub type PB6 = Pin<'B', 6, mode::Input>;
pub type PB7 = Pin<'B', 7, mode::Input>;
pub type PB8 = Pin<'B', 8, mode::Input>;
pub type PB9 = Pin<'B', 9, mode::Input>;
pub type PB10 = Pin<'B', 10, mode::Input>;
pub type PB11 = Pin<'B', 11, mode::Input>;
pub type PB12 = Pin<'B', 12, mode::Input>;
pub type PB13 = Pin<'B', 13, mode::Input>;
pub type PB14 = Pin<'B', 14, mode::Input>;
pub type PB15 = Pin<'B', 15, mode::Input>;

// GPIOC pins
pub type PC0 = Pin<'C', 0, mode::Input>;
pub type PC1 = Pin<'C', 1, mode::Input>;
pub type PC2 = Pin<'C', 2, mode::Input>;
pub type PC3 = Pin<'C', 3, mode::Input>;
pub type PC4 = Pin<'C', 4, mode::Input>;
pub type PC5 = Pin<'C', 5, mode::Input>;
pub type PC6 = Pin<'C', 6, mode::Input>;
pub type PC7 = Pin<'C', 7, mode::Input>;
pub type PC8 = Pin<'C', 8, mode::Input>;
pub type PC9 = Pin<'C', 9, mode::Input>;
pub type PC10 = Pin<'C', 10, mode::Input>;
pub type PC11 = Pin<'C', 11, mode::Input>;
pub type PC12 = Pin<'C', 12, mode::Input>;
pub type PC13 = Pin<'C', 13, mode::Input>;
pub type PC14 = Pin<'C', 14, mode::Input>;
pub type PC15 = Pin<'C', 15, mode::Input>;

// GPIOD pins
pub type PD0 = Pin<'D', 0, mode::Input>;
pub type PD1 = Pin<'D', 1, mode::Input>;
pub type PD2 = Pin<'D', 2, mode::Input>;
pub type PD3 = Pin<'D', 3, mode::Input>;
pub type PD4 = Pin<'D', 4, mode::Input>;
pub type PD5 = Pin<'D', 5, mode::Input>;
pub type PD6 = Pin<'D', 6, mode::Input>;
pub type PD7 = Pin<'D', 7, mode::Input>;
pub type PD8 = Pin<'D', 8, mode::Input>;
pub type PD9 = Pin<'D', 9, mode::Input>;
pub type PD10 = Pin<'D', 10, mode::Input>;
pub type PD11 = Pin<'D', 11, mode::Input>;
pub type PD12 = Pin<'D', 12, mode::Input>;
pub type PD13 = Pin<'D', 13, mode::Input>;
pub type PD14 = Pin<'D', 14, mode::Input>;
pub type PD15 = Pin<'D', 15, mode::Input>;

/// Type-erased GPIO pin that can be any pin on any port
/// This allows storing different pins in collections like arrays
pub struct AnyPin {
    port: char,
    pin: u8,
    _mode: PhantomData<mode::Input>,
}

impl AnyPin {
    /// Create a new AnyPin from port and pin number
    pub fn new(port: char, pin: u8) -> Self {
        Self {
            port,
            pin,
            _mode: PhantomData,
        }
    }

    /// Get the port character
    pub fn port(&self) -> char {
        self.port
    }

    /// Get the pin number
    pub fn pin(&self) -> u8 {
        self.pin
    }
}

// Implement embedded-hal traits for AnyPin
impl embedded_hal::digital::ErrorType for AnyPin {
    type Error = GpioError;
}

impl embedded_hal::digital::OutputPin for AnyPin {
    fn set_low(&mut self) -> Result<(), Self::Error> {
        gpio_impl!(self.port, self.pin, set_low);
        Ok(())
    }

    fn set_high(&mut self) -> Result<(), Self::Error> {
        gpio_impl!(self.port, self.pin, set_high);
        Ok(())
    }
}

impl embedded_hal::digital::StatefulOutputPin for AnyPin {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(gpio_impl!(self.port, self.pin, read_output))
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!gpio_impl!(self.port, self.pin, read_output))
    }
}

impl embedded_hal::digital::InputPin for AnyPin {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(gpio_impl!(self.port, self.pin, read_input))
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!gpio_impl!(self.port, self.pin, read_input))
    }
}

// Implement embedded-hal-async traits for AnyPin
impl embedded_hal_async::digital::Wait for AnyPin {
    async fn wait_for_high(&mut self) -> Result<(), Self::Error> {
        // Simple polling implementation - in a real implementation this would use interrupts
        while self.is_low()? {
            embassy_time::Timer::after(embassy_time::Duration::from_micros(10)).await;
        }
        Ok(())
    }

    async fn wait_for_low(&mut self) -> Result<(), Self::Error> {
        // Simple polling implementation - in a real implementation this would use interrupts
        while self.is_high()? {
            embassy_time::Timer::after(embassy_time::Duration::from_micros(10)).await;
        }
        Ok(())
    }

    async fn wait_for_rising_edge(&mut self) -> Result<(), Self::Error> {
        self.wait_for_low().await?;
        self.wait_for_high().await
    }

    async fn wait_for_falling_edge(&mut self) -> Result<(), Self::Error> {
        self.wait_for_high().await?;
        self.wait_for_low().await
    }

    async fn wait_for_any_edge(&mut self) -> Result<(), Self::Error> {
        let initial_state = self.is_high()?;
        loop {
            if self.is_high()? != initial_state {
                return Ok(());
            }
            embassy_time::Timer::after(embassy_time::Duration::from_micros(10)).await;
        }
    }
}

impl<const PORT: char, const PIN: u8, MODE> Pin<PORT, PIN, MODE> {
    /// Create a new pin instance (primarily for BSP usage)
    pub fn new() -> Pin<PORT, PIN, mode::Input> {
        Pin { _mode: PhantomData }
    }

    /// Convert this pin to a type-erased AnyPin
    /// This allows storing different pins in arrays or other collections
    pub fn degrade(self) -> AnyPin {
        AnyPin::new(PORT, PIN)
    }

    /// Convert pin to output mode
    pub fn into_push_pull_output(self, level: Level, speed: Speed) -> Pin<PORT, PIN, mode::Output> {
        self.into_push_pull_output_with_config(level, speed, Pull::None)
    }

    /// Convert pin to output mode with pull configuration
    pub fn into_push_pull_output_with_config(
        self,
        level: Level,
        _speed: Speed,
        pull: Pull
    ) -> Pin<PORT, PIN, mode::Output> {
        // Set initial output level
        if level == Level::High {
            gpio_impl!(PORT, PIN, set_high);
        } else {
            gpio_impl!(PORT, PIN, set_low);
        }

        // Configure pin as output
        gpio_impl!(PORT, PIN, set_output);

        // Configure pull-up/pull-down if needed
        configure_pull::<PORT, PIN>(pull);

        Pin { _mode: PhantomData }
    }

    /// Convert pin to input mode
    pub fn into_floating_input(self) -> Pin<PORT, PIN, mode::Input> {
        self.into_input_with_pull(Pull::None)
    }

    /// Convert pin to input mode with pull configuration
    pub fn into_input_with_pull(self, pull: Pull) -> Pin<PORT, PIN, mode::Input> {
        // Configure pin as input
        gpio_impl!(PORT, PIN, set_input);

        // Configure pull-up/pull-down
        configure_pull::<PORT, PIN>(pull);

        Pin { _mode: PhantomData }
    }

    /// Convert pin to alternate function mode
    pub fn into_alternate_function<const AF: u8>(self) -> Pin<PORT, PIN, mode::AlternateFunction<AF>> {
        // For HT32, alternate function is configured through AFIO only
        // Set pin as output for most AF functions
        gpio_impl!(PORT, PIN, set_output);

        // Configure alternate function in AFIO
        unsafe {
            configure_alternate_function::<PORT, PIN, AF>();
        }

        Pin { _mode: PhantomData }
    }
}

impl<const PORT: char, const PIN: u8> Pin<PORT, PIN, mode::Input> {
    /// Enable external interrupt on this pin
    pub fn enable_interrupt(&self, edge: Edge) -> Option<ExtiChannel> {
        if PIN <= 15 {
            // Configure EXTI source to this port
            crate::exti::configure_exti_source(PIN, PORT);

            // Create and configure EXTI channel
            if let Some(exti) = ExtiChannel::new(PIN) {
                exti.enable_interrupt(edge);
                Some(exti)
            } else {
                None
            }
        } else {
            None
        }
    }

    /// Wait for external interrupt on this pin
    pub async fn wait_for_interrupt(&self, edge: Edge) {
        if let Some(exti) = self.enable_interrupt(edge) {
            exti.wait().await;
        }
    }
}

/// GPIO Output pin
pub type Output<'d> = Pin<'A', 0, mode::Output>; // Simplified for now

/// GPIO Input pin
pub type Input<'d> = Pin<'A', 0, mode::Input>; // Simplified for now

// Implement embedded-hal traits
impl<const PORT: char, const PIN: u8> ErrorType for Pin<PORT, PIN, mode::Output> {
    type Error = GpioError;
}

impl<const PORT: char, const PIN: u8> OutputPin for Pin<PORT, PIN, mode::Output> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        gpio_impl!(PORT, PIN, set_high);
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        gpio_impl!(PORT, PIN, set_low);
        Ok(())
    }
}

impl<const PORT: char, const PIN: u8> StatefulOutputPin for Pin<PORT, PIN, mode::Output> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(gpio_impl!(PORT, PIN, read_output))
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_set_high()?)
    }
}

impl<const PORT: char, const PIN: u8> ErrorType for Pin<PORT, PIN, mode::Input> {
    type Error = GpioError;
}

impl<const PORT: char, const PIN: u8> InputPin for Pin<PORT, PIN, mode::Input> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(gpio_impl!(PORT, PIN, read_input))
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(!self.is_high()?)
    }
}

fn configure_pull<const PORT: char, const PIN: u8>(pull: Pull) {
    // HT32 pull configuration is done through PxPUR and PxPDR registers
    match pull {
        Pull::None => {
            gpio_impl!(PORT, PIN, disable_pull);
        }
        Pull::Up => {
            gpio_impl!(PORT, PIN, enable_pullup);
        }
        Pull::Down => {
            gpio_impl!(PORT, PIN, enable_pulldown);
        }
    }
}


unsafe fn configure_alternate_function<const PORT: char, const PIN: u8, const AF: u8>() {
    // Configure AFIO for alternate function
    let afio = unsafe { &*Afio::ptr() };

    // HT32 uses different AFIO registers for each GPIO port
    // Each port has two registers: low (pins 0-7) and high (pins 8-15)
    match PORT {
        'A' => {
            if PIN < 8 {
                afio.gpacfglr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << (PIN * 4));  // Clear AF bits (4 bits per pin)
                    val |= (AF as u32) << (PIN * 4); // Set AF value
                    unsafe { w.bits(val) }
                });
            } else {
                afio.gpacfghr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << ((PIN - 8) * 4));  // Clear AF bits
                    val |= (AF as u32) << ((PIN - 8) * 4); // Set AF value
                    unsafe { w.bits(val) }
                });
            }
        }
        'B' => {
            if PIN < 8 {
                afio.gpbcfglr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << (PIN * 4));
                    val |= (AF as u32) << (PIN * 4);
                    unsafe { w.bits(val) }
                });
            } else {
                afio.gpbcfghr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << ((PIN - 8) * 4));
                    val |= (AF as u32) << ((PIN - 8) * 4);
                    unsafe { w.bits(val) }
                });
            }
        }
        'C' => {
            if PIN < 8 {
                afio.gpccfglr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << (PIN * 4));
                    val |= (AF as u32) << (PIN * 4);
                    unsafe { w.bits(val) }
                });
            } else {
                afio.gpccfghr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << ((PIN - 8) * 4));
                    val |= (AF as u32) << ((PIN - 8) * 4);
                    unsafe { w.bits(val) }
                });
            }
        }
        'D' => {
            if PIN < 8 {
                afio.gpdcfglr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << (PIN * 4));
                    val |= (AF as u32) << (PIN * 4);
                    unsafe { w.bits(val) }
                });
            } else {
                afio.gpdcfghr().modify(|r, w| {
                    let mut val = r.bits();
                    val &= !(0b1111 << ((PIN - 8) * 4));
                    val |= (AF as u32) << ((PIN - 8) * 4);
                    unsafe { w.bits(val) }
                });
            }
        }
        _ => panic!("Invalid GPIO port for AF configuration"),
    }
}

/// GPIO port abstractions
pub struct PortA {
    _private: (),
}

pub struct PortB {
    _private: (),
}

pub struct PortC {
    _private: (),
}

pub struct PortD {
    _private: (),
}

impl PortA {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    pub fn pa0(&mut self) -> PA0 { Pin { _mode: PhantomData } }
    pub fn pa1(&mut self) -> PA1 { Pin { _mode: PhantomData } }
    pub fn pa2(&mut self) -> PA2 { Pin { _mode: PhantomData } }
    pub fn pa3(&mut self) -> PA3 { Pin { _mode: PhantomData } }
    pub fn pa4(&mut self) -> PA4 { Pin { _mode: PhantomData } }
    pub fn pa5(&mut self) -> PA5 { Pin { _mode: PhantomData } }
    pub fn pa6(&mut self) -> PA6 { Pin { _mode: PhantomData } }
    pub fn pa7(&mut self) -> PA7 { Pin { _mode: PhantomData } }
    pub fn pa8(&mut self) -> PA8 { Pin { _mode: PhantomData } }
    pub fn pa9(&mut self) -> PA9 { Pin { _mode: PhantomData } }
    pub fn pa10(&mut self) -> PA10 { Pin { _mode: PhantomData } }
    pub fn pa11(&mut self) -> PA11 { Pin { _mode: PhantomData } }
    pub fn pa12(&mut self) -> PA12 { Pin { _mode: PhantomData } }
    pub fn pa13(&mut self) -> PA13 { Pin { _mode: PhantomData } }
    pub fn pa14(&mut self) -> PA14 { Pin { _mode: PhantomData } }
    pub fn pa15(&mut self) -> PA15 { Pin { _mode: PhantomData } }
}

impl PortB {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    pub fn pb0(&mut self) -> PB0 { Pin { _mode: PhantomData } }
    pub fn pb1(&mut self) -> PB1 { Pin { _mode: PhantomData } }
    pub fn pb2(&mut self) -> PB2 { Pin { _mode: PhantomData } }
    pub fn pb3(&mut self) -> PB3 { Pin { _mode: PhantomData } }
    pub fn pb4(&mut self) -> PB4 { Pin { _mode: PhantomData } }
    pub fn pb5(&mut self) -> PB5 { Pin { _mode: PhantomData } }
    pub fn pb6(&mut self) -> PB6 { Pin { _mode: PhantomData } }
    pub fn pb7(&mut self) -> PB7 { Pin { _mode: PhantomData } }
    pub fn pb8(&mut self) -> PB8 { Pin { _mode: PhantomData } }
    pub fn pb9(&mut self) -> PB9 { Pin { _mode: PhantomData } }
    pub fn pb10(&mut self) -> PB10 { Pin { _mode: PhantomData } }
    pub fn pb11(&mut self) -> PB11 { Pin { _mode: PhantomData } }
    pub fn pb12(&mut self) -> PB12 { Pin { _mode: PhantomData } }
    pub fn pb13(&mut self) -> PB13 { Pin { _mode: PhantomData } }
    pub fn pb14(&mut self) -> PB14 { Pin { _mode: PhantomData } }
    pub fn pb15(&mut self) -> PB15 { Pin { _mode: PhantomData } }
}

impl PortC {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    pub fn pc0(&mut self) -> PC0 { Pin { _mode: PhantomData } }
    pub fn pc1(&mut self) -> PC1 { Pin { _mode: PhantomData } }
    pub fn pc2(&mut self) -> PC2 { Pin { _mode: PhantomData } }
    pub fn pc3(&mut self) -> PC3 { Pin { _mode: PhantomData } }
    pub fn pc4(&mut self) -> PC4 { Pin { _mode: PhantomData } }
    pub fn pc5(&mut self) -> PC5 { Pin { _mode: PhantomData } }
    pub fn pc6(&mut self) -> PC6 { Pin { _mode: PhantomData } }
    pub fn pc7(&mut self) -> PC7 { Pin { _mode: PhantomData } }
    pub fn pc8(&mut self) -> PC8 { Pin { _mode: PhantomData } }
    pub fn pc9(&mut self) -> PC9 { Pin { _mode: PhantomData } }
    pub fn pc10(&mut self) -> PC10 { Pin { _mode: PhantomData } }
    pub fn pc11(&mut self) -> PC11 { Pin { _mode: PhantomData } }
    pub fn pc12(&mut self) -> PC12 { Pin { _mode: PhantomData } }
    pub fn pc13(&mut self) -> PC13 { Pin { _mode: PhantomData } }
    pub fn pc14(&mut self) -> PC14 { Pin { _mode: PhantomData } }
    pub fn pc15(&mut self) -> PC15 { Pin { _mode: PhantomData } }
}

impl PortD {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }

    pub fn pd0(&mut self) -> PD0 { Pin { _mode: PhantomData } }
    pub fn pd1(&mut self) -> PD1 { Pin { _mode: PhantomData } }
    pub fn pd2(&mut self) -> PD2 { Pin { _mode: PhantomData } }
    pub fn pd3(&mut self) -> PD3 { Pin { _mode: PhantomData } }
    pub fn pd4(&mut self) -> PD4 { Pin { _mode: PhantomData } }
    pub fn pd5(&mut self) -> PD5 { Pin { _mode: PhantomData } }
    pub fn pd6(&mut self) -> PD6 { Pin { _mode: PhantomData } }
    pub fn pd7(&mut self) -> PD7 { Pin { _mode: PhantomData } }
    pub fn pd8(&mut self) -> PD8 { Pin { _mode: PhantomData } }
    pub fn pd9(&mut self) -> PD9 { Pin { _mode: PhantomData } }
    pub fn pd10(&mut self) -> PD10 { Pin { _mode: PhantomData } }
    pub fn pd11(&mut self) -> PD11 { Pin { _mode: PhantomData } }
    pub fn pd12(&mut self) -> PD12 { Pin { _mode: PhantomData } }
    pub fn pd13(&mut self) -> PD13 { Pin { _mode: PhantomData } }
    pub fn pd14(&mut self) -> PD14 { Pin { _mode: PhantomData } }
    pub fn pd15(&mut self) -> PD15 { Pin { _mode: PhantomData } }
}

/// Extension trait for GPIO port setup
pub trait GpioExt {
    type Parts;
    fn split(self) -> Self::Parts;
}

// Extension implementations would go here for splitting ports into individual pins