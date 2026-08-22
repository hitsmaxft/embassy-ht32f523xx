//! Interrupt handling for HT32F523xx
//!
//! This module provides interrupt handling utilities and waker management
//! for Embassy async drivers.

use core::marker::PhantomData;
use core::task::Poll;
use embassy_sync::waitqueue::AtomicWaker;

pub use crate::pac::Interrupt;

/// Default interrupt handler placeholder
#[unsafe(no_mangle)]
pub extern "C" fn DefaultHandler() -> ! {
    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

/// Trait for interrupt handlers
pub trait InterruptHandler<T> {
    /// Handle the interrupt
    fn on_interrupt(&mut self);
}

/// Interrupt binding type
pub struct Binding<T, H> {
    _phantom: PhantomData<(T, H)>,
}

impl<T, H> Binding<T, H> {
    pub fn new() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

/// GPIO External Interrupt types
pub struct Exti0_1;
pub struct Exti2_3;
pub struct Exti4_15;

/// Timer interrupt types
pub struct Gptm0;
pub struct Gptm1;

/// UART interrupt types
pub struct Usart0;
pub struct Usart1;

/// USB interrupt type
pub struct UsbInterrupt;

/// Interrupt waker utility
pub struct InterruptWaker {
    waker: AtomicWaker,
}

impl InterruptWaker {
    pub const fn new() -> Self {
        Self {
            waker: AtomicWaker::new(),
        }
    }

    pub fn wake(&self) {
        self.waker.wake();
    }

    pub fn wait(&self) -> impl core::future::Future<Output = ()> + '_ {
        // Use embassy's waitqueue API correctly
        core::future::poll_fn(move |cx| {
            self.waker.register(cx.waker());
            Poll::Pending
        })
    }
}

/// Global interrupt wakers for each interrupt type
static GPTM0_WAKER: InterruptWaker = InterruptWaker::new();
static GPTM1_WAKER: InterruptWaker = InterruptWaker::new();
static USART0_WAKER: InterruptWaker = InterruptWaker::new();
static USART1_WAKER: InterruptWaker = InterruptWaker::new();
static USB_WAKER: InterruptWaker = InterruptWaker::new();
static EXTI0_1_WAKER: InterruptWaker = InterruptWaker::new();
static EXTI2_3_WAKER: InterruptWaker = InterruptWaker::new();
static EXTI4_15_WAKER: InterruptWaker = InterruptWaker::new();

/// Get the waker for a specific interrupt
pub fn get_waker(interrupt: Interrupt) -> &'static InterruptWaker {
    match interrupt {
        Interrupt::GPTM0 => &GPTM0_WAKER,
        Interrupt::GPTM1 => &GPTM1_WAKER,
        Interrupt::USART0 => &USART0_WAKER,
        Interrupt::USART1 => &USART1_WAKER,
        Interrupt::USB => &USB_WAKER,
        Interrupt::EXTI0_1 => &EXTI0_1_WAKER,
        Interrupt::EXTI2_3 => &EXTI2_3_WAKER,
        Interrupt::EXTI4_15 => &EXTI4_15_WAKER,
        _ => panic!("Unsupported interrupt"),
    }
}

/// Initialize the interrupt system with proper NVIC priority configuration
pub fn init() {
    // Enable NVIC for key interrupts using the existing approach
    // Note: NVIC priorities will use default values for now
    // The Signal mechanism from Phase 1 should handle the deadlock
    unsafe {
        cortex_m::peripheral::NVIC::unmask(Interrupt::GPTM0);
        cortex_m::peripheral::NVIC::unmask(Interrupt::GPTM1);
        cortex_m::peripheral::NVIC::unmask(Interrupt::BFTM0);
        cortex_m::peripheral::NVIC::unmask(Interrupt::BFTM1);
        cortex_m::peripheral::NVIC::unmask(Interrupt::USB);
        cortex_m::peripheral::NVIC::unmask(Interrupt::USART0);
        cortex_m::peripheral::NVIC::unmask(Interrupt::USART1);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI0_1);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI2_3);
        cortex_m::peripheral::NVIC::unmask(Interrupt::EXTI4_15);
    }
}

// GPTM0 interrupt handler for embassy-time driver
#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn GPTM0() {
    crate::time_driver::get_driver().on_interrupt();
}

#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn BFTM0() {
    crate::timer::on_interrupt::<crate::timer::Timer0>();
}

#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn BFTM1() {
    crate::timer::on_interrupt::<crate::timer::Timer1>();
}

// EXTI interrupt handlers for GPIO async operations
#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EXTI0_1() {
    crate::exti::on_interrupt(0x0003);
}

#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EXTI2_3() {
    crate::exti::on_interrupt(0x000c);
}

#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn EXTI4_15() {
    crate::exti::on_interrupt(0xfff0);
}

#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn USART0() {
    crate::uart::on_interrupt::<crate::uart::Usart0>();
}

#[cfg(feature = "rt")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn USART1() {
    crate::uart::on_interrupt::<crate::uart::Usart1>();
}

#[cfg(all(feature = "rt", feature = "usb"))]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn USB() {
    // 🚨 安全地调用 USB 驱动的事件处理器
    unsafe { crate::usb::on_usb_interrupt() };
}
