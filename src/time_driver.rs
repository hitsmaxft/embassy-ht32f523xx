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

// Simple counters for tracking overflows (using static mut for Cortex-M0+ compatibility)
static mut OVERFLOW_COUNT: u32 = 0;
static mut LAST_COUNTER: u16 = 0;

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

        // Get system clock frequency - use a reasonable default
        let timer_freq = 48_000_000; // 48MHz system clock

        // Calculate prescaler for TICK_HZ frequency (1MHz = 1us tick)
        let psc = (timer_freq / TICK_HZ) - 1;

        // Set prescaler
        timer.gptm_pscr().write(|w| unsafe { w.bits(psc) });

        // Set to maximum period (16-bit timer)
        timer.gptm_crr().write(|w| unsafe { w.bits(0xFFFF) });

        // Configure for up-counting mode
        timer.gptm_mdcfr().modify(|_, w| w.tse().set_bit());

        // Set compare channel 0 for half-overflow (0x8000)
        timer.gptm_ch0acr().write(|w| unsafe { w.bits(0x8000) });

        // Set compare channel 1 for alarm (will be set dynamically)
        timer.gptm_ch1acr().write(|w| unsafe { w.bits(0x0000) });

        // Enable interrupts: overflow, channel 0 (half-overflow), channel 1 (alarm)
        timer.gptm_dictr().modify(|_, w| {
            w.uevie().set_bit()    // Update Event (overflow) Interrupt Enable
             .ch0ccie().set_bit()    // Channel 0 Interrupt Enable
             .ch1ccie().clear_bit()  // Channel 1 Interrupt Enable (enabled when alarm set)
        });

        // Clear any pending interrupts
        timer.gptm_intsr().write(|w| {
            w.uevif().set_bit()    // Clear Update Event flag
             .ch0ccif().set_bit()    // Clear Channel 0 flag
             .ch1ccif().set_bit()    // Clear Channel 1 flag
        });

        // Initialize static variables
        unsafe {
            OVERFLOW_COUNT = 0;
            LAST_COUNTER = 0;
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
            timer.gptm_ch1acr().write(|w| unsafe { w.bits(timestamp as u32) });

            // Enable it if it'll happen soon. Otherwise, period tracking will enable it.
            // Use the same threshold as STM32: 0xc000 ticks (about 49ms at 1MHz)
            let diff = timestamp - t;
            if diff < 0xc000 {
                timer.gptm_dictr().modify(|_, w| w.ch1ccie().set_bit());
            } else {
                timer.gptm_dictr().modify(|_, w| w.ch1ccie().clear_bit());
            }

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

    // Trigger alarm processing - called from interrupt handler
    fn trigger_alarm(&self, cs: CriticalSection) {
        // Clear current alarm
        self.alarm.borrow(cs).timestamp.set(u64::MAX);

        // Process expired timers and set next alarm using STM32 pattern
        let mut next = self.queue.borrow(cs).borrow_mut().next_expiration(self.now());
        while !self.set_alarm(next) {
            next = self.queue.borrow(cs).borrow_mut().next_expiration(self.now());
        }
    }

    // Enable alarm if it's approaching within the current period
    fn enable_nearby_alarms(&self, now: u64) {
        critical_section::with(|cs| {
            let alarm = self.alarm.borrow(cs);
            let alarm_time = alarm.timestamp.get();

            if alarm_time != u64::MAX {
                // Use same threshold as STM32: 0xc000 ticks
                if alarm_time < now + 0xc000 {
                    // Alarm is approaching, enable it
                    let timer = unsafe { &*crate::pac::Gptm0::ptr() };
                    timer.gptm_dictr().modify(|_, w| w.ch1ccie().set_bit());
                }
            }
        })
    }

    // Check for expired alarms and trigger them
    fn check_expired_alarms(&self, now: u64) {
        critical_section::with(|cs| {
            let alarm = self.alarm.borrow(cs);
            let alarm_time = alarm.timestamp.get();

            // DEBUG: Check if we have any alarm set
            if alarm_time != u64::MAX {
                // DEBUG: We have an alarm, check if it expired
                if alarm_time <= now {
                    // Alarm has expired, trigger alarm processing
                    self.trigger_alarm(cs);
                } else {
                    // Check if alarm is approaching and should be enabled
                    self.enable_nearby_alarms(now);
                }
            }
        })
    }
}

impl Driver for TimeDriver {
    fn now(&self) -> u64 {
        let timer = unsafe { &*crate::pac::Gptm0::ptr() };

        // Get current counter value
        let counter = timer.gptm_cntr().read().bits() as u16;

        // Use critical section to safely update static variables
        let now = critical_section::with(|_| {
            unsafe {
                // Check if we've had an overflow (counter wrapped around)
                if counter < LAST_COUNTER {
                    // Counter wrapped around, increment overflow count
                    OVERFLOW_COUNT += 1;
                }

                // Update last counter
                LAST_COUNTER = counter;

                // Calculate timestamp: (overflow_count * 65536) + counter
                ((OVERFLOW_COUNT as u64) << 16) | (counter as u64)
            }
        });

        // Check for expired alarms EVERY time now() is called
        // This provides a polling-based fallback if interrupts don't work
        self.check_expired_alarms(now);

        now
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

/// Handle GPTM0 interrupt - called from interrupt handler
pub fn handle_gptm0_interrupt() {
    let timer = unsafe { &*crate::pac::Gptm0::ptr() };

    critical_section::with(|_| {
        // Read interrupt status
        let intsr = timer.gptm_intsr().read();

        // Clear all interrupt flags immediately
        timer.gptm_intsr().write(|w| {
            w.uevif().set_bit()    // Clear Update Event flag
             .ch0ccif().set_bit()    // Clear Channel 0 flag
             .ch1ccif().set_bit()    // Clear Channel 1 flag
        });

        // Handle update event (overflow) interrupt
        if intsr.uevif().bit() {
            // Timer overflow occurred - this may affect period tracking
            // Our overflow detection in now() will handle this
        }

        // Handle channel 0 (half-overflow) interrupt
        if intsr.ch0ccif().bit() {
            // Half-overflow occurred - may affect period tracking
        }

        // Handle channel 1 (alarm) interrupt
        if intsr.ch1ccif().bit() {
            // Alarm interrupt - trigger alarm processing
            critical_section::with(|cs| {
                DRIVER.trigger_alarm(cs);
            });
        }
    })
}