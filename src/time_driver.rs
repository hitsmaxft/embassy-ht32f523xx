//! Embassy-time driver implementation for HT32F523x2
//! Simple implementation using 16-bit timer with basic overflow tracking
//!

use core::cell::Cell;
use core::cell::RefCell;

use critical_section::CriticalSection;
use embassy_sync::blocking_mutex::Mutex;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_time_driver::Driver;
use embassy_time_queue_utils::Queue;

// Embassy-time tick frequency (1MHz = 1μs tick resolution)
const TICK_HZ: u32 = 1_000_000;

// High word maintained by the GPTM0 update interrupt. Access is serialized by
// critical sections because Cortex-M0+ has no native AtomicU64.
static mut OVERFLOW_COUNT: u32 = 0;

struct AlarmState {
    timestamp: Cell<u64>,
}

unsafe impl Send for AlarmState {}

impl AlarmState {
    const fn new() -> Self {
        Self {
            timestamp: Cell::new(u64::MAX),
        }
    }
}

pub(crate) struct TimeDriver {
    alarm: Mutex<CriticalSectionRawMutex, AlarmState>,
    queue: Mutex<CriticalSectionRawMutex, RefCell<Queue>>,
}

embassy_time_driver::time_driver_impl!(static DRIVER: TimeDriver = TimeDriver {
    alarm: Mutex::const_new(CriticalSectionRawMutex::new(), AlarmState::new()),
    queue: Mutex::new(RefCell::new(Queue::new()))
});

impl TimeDriver {
    fn init(&'static self, _cs: CriticalSection) {
        let timer = unsafe { &*crate::pac::Gptm0::ptr() };

        // Enable timer clock
        let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
        ckcu.apbccr1().modify(|_, w| w.gptm0en().set_bit());

        // Disable timer first
        timer.gptm_ctr().modify(|_, w| w.tme().clear_bit());
        timer.gptm_cntr().write(|w| unsafe { w.bits(0) });

        // RCC configures GPTM0PCLK to CK_AHB (/1), so derive the prescaler
        // from the frozen clock tree instead of assuming 48 MHz.
        let timer_freq = crate::rcc::get_clocks().apb_clk().to_hz();
        assert!(timer_freq >= TICK_HZ && timer_freq.is_multiple_of(TICK_HZ));

        // Calculate prescaler for TICK_HZ frequency (1MHz = 1us tick)
        let psc = (timer_freq / TICK_HZ) - 1;

        // Set prescaler
        timer.gptm_pscr().write(|w| unsafe { w.bits(psc) });

        // Set to maximum period (16-bit timer)
        timer.gptm_crr().write(|w| unsafe { w.bits(0xFFFF) });

        // Normal internal-clock, up-counting mode. MDCFR.TSE controls trigger
        // input filtering; it is not a counter-direction selector.
        timer.gptm_cntcfr().write(|w| unsafe { w.bits(0) });
        timer.gptm_mdcfr().write(|w| unsafe { w.bits(0) });

        // Configure channel 1 as a no-pin-change output compare channel. The
        // Holtek FWLib writes the compare value to CHxCCR; CHxACR is only the
        // asymmetric-PWM companion register.
        timer.gptm_ch1ocfr().write(|w| unsafe { w.bits(0) });
        timer.gptm_ch1ccr().write(|w| unsafe { w.bits(0) });
        timer.gptm_chctr().modify(|_, w| w.ch1e().set_bit());

        // PSCR and CRR are preloaded registers. A software update event is
        // required to copy them into their active shadow registers. Without
        // this, GPTM0 continues with the reset /1 prescaler even though PSCR
        // reads back the requested value.
        timer.gptm_evgr().write(|w| w.uevg().set_bit());

        // The software update sets UEVIF. Clear it before enabling the update
        // interrupt, then configure the sole Embassy alarm channel.
        timer.gptm_intsr().write(|w| unsafe { w.bits(0) });
        cortex_m::asm::dsb();
        timer.gptm_dictr().write(|w| {
            w.uevie()
                .set_bit() // Update Event (overflow) Interrupt Enable
                .ch0ccie()
                .clear_bit()
                .ch1ccie()
                .clear_bit() // Channel 1 Interrupt Enable (enabled when alarm set)
        });

        // Initialize static variables
        unsafe {
            OVERFLOW_COUNT = 0;
        }

        // Start timer
        timer.gptm_ctr().modify(|_, w| w.tme().set_bit());

        // Enable GPTM0 interrupt in NVIC
        unsafe { cortex_m::peripheral::NVIC::unmask(crate::pac::Interrupt::GPTM0) };
    }

    fn set_alarm(&self, timestamp: u64) -> bool {
        critical_section::with(|cs| {
            let alarm = self.alarm.borrow(cs);
            alarm.timestamp.set(timestamp);

            let t = self.now();
            if timestamp <= t {
                // If alarm timestamp has passed the alarm will not fire.
                // Disarm the alarm and return `false` to indicate that.
                alarm.timestamp.set(u64::MAX);

                // Disable channel 1 interrupt
                let timer = unsafe { &*crate::pac::Gptm0::ptr() };
                timer.gptm_dictr().modify(|_, w| w.ch1ccie().clear_bit());
                return false;
            }

            // Write the compare value regardless of whether we enable it now
            // This way, when we enable it later, the right value is already set
            let timer = unsafe { &*crate::pac::Gptm0::ptr() };
            timer
                .gptm_ch1ccr()
                .write(|w| unsafe { w.bits(timestamp as u32) });

            // Keep the low-word compare armed. For alarms more than one
            // 16-bit period away it may fire early once per wrap; the ISR
            // checks the full 64-bit timestamp and only wakes at expiry.
            timer.gptm_dictr().modify(|_, w| w.ch1ccie().set_bit());

            // Reevaluate if the alarm timestamp is still in the future
            let t = self.now();
            if timestamp <= t {
                // Race condition: alarm timestamp has passed since we set it
                alarm.timestamp.set(u64::MAX);
                timer.gptm_dictr().modify(|_, w| w.ch1ccie().clear_bit());
                return false;
            }

            true
        })
    }

    // Trigger alarm processing - called from on_interrupt()
    fn trigger_alarm(&self, cs: CriticalSection) {
        // Clear current alarm
        self.alarm.borrow(cs).timestamp.set(u64::MAX);

        // Process expired timers and set next alarm using STM32 pattern
        let mut next = self
            .queue
            .borrow(cs)
            .borrow_mut()
            .next_expiration(self.now());
        while !self.set_alarm(next) {
            next = self
                .queue
                .borrow(cs)
                .borrow_mut()
                .next_expiration(self.now());
        }
    }

    // Standard Embassy interrupt handler - called from GPTM0 interrupt
    // Optimized: Minimize critical section scope to prevent deadlock with USB interrupts
    pub fn on_interrupt(&self) {
        let timer = unsafe { &*crate::pac::Gptm0::ptr() };

        // Read interrupt status outside critical section
        let intsr = timer.gptm_intsr().read();

        // W0C: zero only the flags observed in this snapshot and write one to
        // every other bit, so an event arriving after the read is preserved.
        timer
            .gptm_intsr()
            .write(|w| unsafe { w.bits(!intsr.bits()) });
        // Match Holtek TM_ClearFlag(): ensure W0C reaches the peripheral
        // before exception return, otherwise NVIC may immediately re-enter.
        cortex_m::asm::dsb();

        // Handle update event (overflow) interrupt - no critical section needed
        if intsr.uevif().bit() {
            critical_section::with(|_| unsafe {
                OVERFLOW_COUNT = OVERFLOW_COUNT.wrapping_add(1);
            });
        }

        // A compare can occur in an earlier 16-bit period. Only publish the
        // alarm when its full timestamp has actually expired. Also check at
        // overflow to close the compare/overflow boundary race.
        if intsr.ch1ccif().bit() || intsr.uevif().bit() {
            critical_section::with(|cs| {
                let timestamp = self.alarm.borrow(cs).timestamp.get();
                if timestamp != u64::MAX && timestamp <= self.now() {
                    let timer = unsafe { &*crate::pac::Gptm0::ptr() };
                    timer.gptm_dictr().modify(|_, w| w.ch1ccie().clear_bit());
                    self.trigger_alarm(cs);
                }
            });
        }
    }
}

impl Driver for TimeDriver {
    fn now(&self) -> u64 {
        let timer = unsafe { &*crate::pac::Gptm0::ptr() };

        critical_section::with(|_| {
            let high = unsafe { OVERFLOW_COUNT };
            let counter = timer.gptm_cntr().read().bits() as u16;
            let overflow_pending = timer.gptm_intsr().read().uevif().bit_is_set();

            // If the counter wrapped before its ISR ran, include that pending
            // period in this sample without mutating the ISR-owned high word.
            let high = high.wrapping_add(overflow_pending as u32);
            ((high as u64) << 16) | counter as u64
        })
    }

    fn schedule_wake(&self, at: u64, waker: &core::task::Waker) {
        critical_section::with(|cs| {
            let mut queue = self.queue.borrow(cs).borrow_mut();

            if queue.schedule_wake(at, waker) {
                // Process the queue immediately to set the next alarm
                // This is the key insight from the provided example
                let mut next = queue.next_expiration(self.now());
                while !self.set_alarm(next) {
                    next = queue.next_expiration(self.now());
                }
            }
        })
    }
}

pub(crate) fn init(cs: CriticalSection) {
    DRIVER.init(cs)
}

/// Get the time driver instance - used by interrupt handler
pub(crate) fn get_driver() -> &'static TimeDriver {
    &DRIVER
}
