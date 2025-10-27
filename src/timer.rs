//! Timer driver for HT32 GPTM (General Purpose Timer Module)

use crate::pac::Gptm1;

use embassy_time::Duration;
use embassy_sync::waitqueue::AtomicWaker;
use core::marker::PhantomData;

/// Timer instance trait
pub trait Instance {
    /// Get the timer register block
    fn regs() -> &'static crate::pac::gptm0::RegisterBlock;

    /// Get the timer interrupt waker
    fn waker() -> &'static AtomicWaker;
}

/// Timer 0
pub struct Timer0 {
    _private: (),
}

impl Timer0 {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl Instance for Timer0 {
    fn regs() -> &'static crate::pac::gptm0::RegisterBlock {
        unsafe { &*crate::pac::Gptm0::ptr() }
    }

    fn waker() -> &'static AtomicWaker {
        static WAKER: AtomicWaker = AtomicWaker::new();
        &WAKER
    }
}

/// Timer 1
pub struct Timer1 {
    _private: (),
}

impl Timer1 {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl Instance for Timer1 {
    fn regs() -> &'static crate::pac::gptm0::RegisterBlock {
        unsafe { &*Gptm1::ptr() }
    }

    fn waker() -> &'static AtomicWaker {
        static WAKER: AtomicWaker = AtomicWaker::new();
        &WAKER
    }
}

// Note: HT32F523x2 only has GPTM0 and GPTM1 available
// Additional timer instances would be added here for other HT32 variants

/// Generic timer driver
pub struct Timer<T: Instance> {
    _instance: PhantomData<T>,
}

impl<T: Instance> Timer<T> {
    /// Create a new timer instance
    pub fn new() -> Self {
        // Initialize the timer hardware
        let regs = T::regs();

        // Basic timer setup
        regs.gptm_ctr().modify(|_, w| w.tme().clear_bit()); // Disable timer
        regs.gptm_mdcfr().modify(|_, w| w.tse().bit(true)); // Up counting mode

        Self {
            _instance: PhantomData,
        }
    }

    /// Start a one-shot timer for the given duration
    pub async fn sleep(&mut self, duration: Duration) {
        let _regs = T::regs();
        let _waker = T::waker();

        // Calculate timer parameters based on system clock
        let clock_freq = crate::rcc::get_clocks().apb_clk().to_hz();
        let ticks = (duration.as_micros() as u64 * clock_freq as u64) / 1_000_000;

        if ticks > u32::MAX as u64 {
            // Duration too long, split into multiple waits
            // For simplicity, just wait for maximum duration
            self.wait_ticks(u32::MAX).await;
            return;
        }

        self.wait_ticks(ticks as u32).await;
    }

    async fn wait_ticks(&mut self, ticks: u32) {
        let regs = T::regs();
        let waker = T::waker();

        // Set up timer for one-shot operation
        regs.gptm_ctr().modify(|_, w| w.tme().clear_bit()); // Disable timer
        regs.gptm_cntr().reset(); // Reset counter
        regs.gptm_crr().write(|w| unsafe { w.bits(ticks) }); // Set compare value

        // Enable compare interrupt
        regs.gptm_evgr().write(|w| w.ch0ccg().set_bit()); // Clear interrupt flag
        regs.gptm_dictr().modify(|_, w| w.ch0ccie().set_bit()); // Enable interrupt

        // Start timer
        regs.gptm_ctr().modify(|_, w| w.tme().set_bit());

        // Wait for interrupt
        core::future::poll_fn(|cx| {
            waker.register(cx.waker());

            // Check if timer has elapsed
            if regs.gptm_intsr().read().ch0ccif().bit_is_set() {
                // Clear interrupt flag
                regs.gptm_evgr().write(|w| w.ch0ccg().set_bit());
                // Disable timer
                regs.gptm_ctr().modify(|_, w| w.tme().clear_bit());
                core::task::Poll::Ready(())
            } else {
                core::task::Poll::Pending
            }
        }).await;
    }

    /// Get the current timer counter value
    pub fn get_counter(&self) -> u32 {
        T::regs().gptm_cntr().read().bits()
    }

    /// Set the timer prescaler
    pub fn set_prescaler(&mut self, prescaler: u16) {
        T::regs().gptm_pscr().write(|w| unsafe { w.bits(prescaler as u32) });
    }

    /// Set the timer frequency
    pub fn set_frequency(&mut self, freq: crate::time::Hertz) {
        let clock_freq = crate::rcc::get_clocks().apb_clk().to_hz();
        let prescaler = (clock_freq / freq.to_hz()) - 1;
        self.set_prescaler(prescaler as u16);
    }
}

// Interrupt handlers would go here
// These need to be implemented for each timer instance

/// Initialize embassy-time using a hardware timer
pub fn init_embassy_time() {
    // This would typically use SysTick or a dedicated timer for embassy-time
    // For now, this is a placeholder
}

/// PWM channel configuration
pub enum Channel {
    Ch0,
    Ch1,
    Ch2,
    Ch3,
}

/// PWM driver
pub struct Pwm<T: Instance> {
    _instance: PhantomData<T>,
}

impl<T: Instance> Pwm<T> {
    /// Create a new PWM instance
    pub fn new() -> Self {
        let regs = T::regs();

        // Configure timer for PWM mode
        regs.gptm_mdcfr().modify(|_, w| w.tse().bit(true)); // Up counting

        Self {
            _instance: PhantomData,
        }
    }

    /// Set PWM duty cycle for a channel
    pub fn set_duty_cycle(&mut self, channel: Channel, duty: u16, max: u16) {
        let regs = T::regs();
        let duty_ticks = (duty as u32 * regs.gptm_crr().read().bits()) / max as u32;

        match channel {
            Channel::Ch0 => regs.gptm_ch0ccr().write(|w| unsafe { w.bits(duty_ticks) }),
            Channel::Ch1 => regs.gptm_ch1ccr().write(|w| unsafe { w.bits(duty_ticks) }),
            Channel::Ch2 => regs.gptm_ch2ccr().write(|w| unsafe { w.bits(duty_ticks) }),
            Channel::Ch3 => regs.gptm_ch3ccr().write(|w| unsafe { w.bits(duty_ticks) }),
        }
    }

    /// Enable PWM output for a channel
    pub fn enable_channel(&mut self, channel: Channel) {
        let regs = T::regs();

        match channel {
            Channel::Ch0 => regs.gptm_chctr().modify(|_, w| w.ch0e().set_bit()),
            Channel::Ch1 => regs.gptm_chctr().modify(|_, w| w.ch1e().set_bit()),
            Channel::Ch2 => regs.gptm_chctr().modify(|_, w| w.ch2e().set_bit()),
            Channel::Ch3 => regs.gptm_chctr().modify(|_, w| w.ch3e().set_bit()),
        }
    }
}