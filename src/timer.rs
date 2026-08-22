//! Asynchronous timers backed by HT32 Basic Function Timers (BFTM).
//!
//! This follows the ChibiOS HT32 GPT backend: BFTM runs directly from the AHB
//! clock, `CMP` contains the raw interval, `SR.MIF` is cleared by writing zero,
//! and one-shot operation uses `CR = CEN | OSM | MIEN`.

use core::cell::Cell;
use core::marker::PhantomData;
use core::task::Poll;

use critical_section::Mutex;
use embassy_sync::waitqueue::AtomicWaker;
use embassy_time::Duration;

use crate::pac::{Bftm0, Bftm1, Interrupt};

const CR_MIEN: u32 = 1 << 0;
const CR_OSM: u32 = 1 << 1;
const CR_CEN: u32 = 1 << 2;
const SR_MIF: u32 = 1 << 0;

#[doc(hidden)]
pub struct TimerSignal {
    fired: Mutex<Cell<bool>>,
    waker: AtomicWaker,
}

impl TimerSignal {
    const fn new() -> Self {
        Self {
            fired: Mutex::new(Cell::new(false)),
            waker: AtomicWaker::new(),
        }
    }

    fn clear(&self) {
        critical_section::with(|cs| self.fired.borrow(cs).set(false));
    }

    fn fire(&self) {
        critical_section::with(|cs| self.fired.borrow(cs).set(true));
        self.waker.wake();
    }

    fn take(&self) -> bool {
        critical_section::with(|cs| {
            let fired = self.fired.borrow(cs);
            let value = fired.get();
            fired.set(false);
            value
        })
    }
}

static BFTM0_SIGNAL: TimerSignal = TimerSignal::new();
static BFTM1_SIGNAL: TimerSignal = TimerSignal::new();

mod sealed {
    pub trait Sealed {}
}

/// BFTM instance used by [`Timer`].
pub trait Instance: sealed::Sealed {
    fn regs() -> &'static crate::pac::bftm0::RegisterBlock;
    fn enable_clock();
    fn interrupt() -> Interrupt;
    fn signal() -> &'static TimerSignal;
}

/// Ownership token for BFTM0.
pub struct Timer0 {
    _private: (),
}

impl Timer0 {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl sealed::Sealed for Timer0 {}

impl Instance for Timer0 {
    fn regs() -> &'static crate::pac::bftm0::RegisterBlock {
        unsafe { &*Bftm0::ptr() }
    }

    fn enable_clock() {
        let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
        ckcu.apbccr1().modify(|_, w| w.bftm0en().set_bit());
    }

    fn interrupt() -> Interrupt {
        Interrupt::BFTM0
    }

    fn signal() -> &'static TimerSignal {
        &BFTM0_SIGNAL
    }
}

/// Ownership token for BFTM1.
pub struct Timer1 {
    _private: (),
}

impl Timer1 {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl sealed::Sealed for Timer1 {}

impl Instance for Timer1 {
    fn regs() -> &'static crate::pac::bftm0::RegisterBlock {
        unsafe { &*Bftm1::ptr() }
    }

    fn enable_clock() {
        let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
        ckcu.apbccr1().modify(|_, w| w.bftm1en().set_bit());
    }

    fn interrupt() -> Interrupt {
        Interrupt::BFTM1
    }

    fn signal() -> &'static TimerSignal {
        &BFTM1_SIGNAL
    }
}

/// One-shot asynchronous BFTM timer.
pub struct Timer<T: Instance> {
    _instance: PhantomData<T>,
}

impl<T: Instance> Timer<T> {
    /// Create a timer from its unique peripheral token.
    pub fn new(_instance: T) -> Self {
        T::enable_clock();
        let regs = T::regs();
        regs.cr().write(|w| unsafe { w.bits(0) });
        regs.sr().write(|w| unsafe { w.bits(0) });
        T::signal().clear();
        unsafe { cortex_m::peripheral::NVIC::unmask(T::interrupt()) };

        Self {
            _instance: PhantomData,
        }
    }

    /// Sleep for at least `duration`.
    ///
    /// BFTM has a 32-bit compare at the undivided AHB clock. Long durations
    /// are split into multiple one-shot intervals.
    pub async fn sleep(&mut self, duration: Duration) {
        let frequency = u64::from(crate::rcc::get_clocks().ahb_clk().to_hz());
        let time_ticks = duration.as_ticks();
        let timer_ticks = time_ticks
            .saturating_mul(frequency)
            .saturating_add(embassy_time::TICK_HZ - 1)
            / embassy_time::TICK_HZ;

        let mut remaining = timer_ticks;
        while remaining != 0 {
            let chunk = remaining.min(u64::from(u32::MAX)) as u32;
            self.wait_ticks(chunk).await;
            remaining -= u64::from(chunk);
        }
    }

    async fn wait_ticks(&mut self, ticks: u32) {
        if ticks == 0 {
            return;
        }

        let regs = T::regs();
        let signal = T::signal();
        let mut started = false;
        let _stop_on_cancel = StopGuard::<T>(PhantomData);

        core::future::poll_fn(|cx| {
            signal.waker.register(cx.waker());

            if !started {
                signal.clear();
                regs.cr().write(|w| unsafe { w.bits(0) });
                regs.sr().write(|w| unsafe { w.bits(0) });
                regs.cntr().write(|w| unsafe { w.bits(0) });
                regs.cmpr().write(|w| unsafe { w.bits(ticks) });
                regs.cr()
                    .write(|w| unsafe { w.bits(CR_CEN | CR_OSM | CR_MIEN) });
                started = true;
            }

            if signal.take() || regs.sr().read().bits() & SR_MIF != 0 {
                stop::<T>();
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;
    }

    /// Read the raw BFTM counter, in AHB clock cycles.
    pub fn counter(&self) -> u32 {
        T::regs().cntr().read().bits()
    }

    /// Stop a running interval.
    pub fn stop(&mut self) {
        stop::<T>();
        T::signal().clear();
    }
}

struct StopGuard<T: Instance>(PhantomData<T>);

impl<T: Instance> Drop for StopGuard<T> {
    fn drop(&mut self) {
        stop::<T>();
    }
}

fn stop<T: Instance>() {
    let regs = T::regs();
    regs.cr().write(|w| unsafe { w.bits(0) });
    // BFTM SR.MIF is write-zero-to-clear, matching Holtek FWLib and ChibiOS.
    regs.sr().write(|w| unsafe { w.bits(0) });
    cortex_m::asm::dsb();
}

/// Handle a BFTM interrupt.
pub(crate) fn on_interrupt<T: Instance>() {
    let regs = T::regs();
    if regs.sr().read().bits() & SR_MIF == 0 {
        return;
    }

    stop::<T>();
    T::signal().fire();
}
