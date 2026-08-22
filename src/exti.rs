//! External interrupt support for HT32F523xx GPIO pins.

use core::cell::Cell;
use core::task::Poll;

use critical_section::Mutex;
use embassy_sync::waitqueue::AtomicWaker;

use crate::pac::Interrupt;
use crate::pac::{Afio, Exti};

/// EXTI trigger edge configuration.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Edge {
    /// Rising edge trigger.
    Rising,
    /// Falling edge trigger.
    Falling,
    /// Both rising and falling edges.
    RisingFalling,
}

/// EXTI line number (0-15, corresponding to pin numbers).
pub type ExtiLine = u8;

static FIRED: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));
static WAKERS: [AtomicWaker; 16] = [const { AtomicWaker::new() }; 16];

/// EXTI channel - maps GPIO pins to interrupt lines.
pub struct ExtiChannel {
    line: ExtiLine,
}

impl ExtiChannel {
    /// Create a new EXTI channel for the given GPIO pin.
    pub fn new(pin: u8) -> Option<Self> {
        (pin <= 15).then_some(Self { line: pin })
    }

    /// Enable the EXTI line with the specified trigger edge.
    pub fn enable_interrupt(&self, edge: Edge) {
        let exti = unsafe { &*Exti::ptr() };
        let mask = 1u32 << self.line;

        // Holtek implements one full 32-bit CFGR register per channel. SRCTYPE
        // occupies bits 30:28 and DBEN bit 31; leave debounce disabled here.
        let source_type = match edge {
            Edge::Falling => 0x2,
            Edge::Rising => 0x3,
            Edge::RisingFalling => 0x4,
        };

        self.clear_pending();
        clear_fired(mask);
        unsafe {
            (Exti::ptr() as *mut u32)
                .add(self.line as usize)
                .write_volatile(source_type << 28);
        }
        exti.cr().modify(|r, w| unsafe { w.bits(r.bits() | mask) });
    }

    /// Disable the EXTI line.
    pub fn disable_interrupt(&self) {
        let exti = unsafe { &*Exti::ptr() };
        let mask = 1u32 << self.line;
        exti.cr().modify(|r, w| unsafe { w.bits(r.bits() & !mask) });
        self.clear_pending();
        clear_fired(mask);
    }

    /// Check if an edge is pending in hardware.
    pub fn is_pending(&self) -> bool {
        let exti = unsafe { &*Exti::ptr() };
        exti.edgeflgr().read().bits() & (1 << self.line) != 0
    }

    /// Clear both the edge flag and its positive/negative edge status.
    pub fn clear_pending(&self) {
        clear_pending_mask(1u32 << self.line);
    }

    /// Wait for either edge.
    pub async fn wait(&self) {
        self.wait_for_edge(Edge::RisingFalling).await;
    }

    /// Wait for the selected edge.
    pub async fn wait_for_edge(&self, edge: Edge) {
        let mask = 1u32 << self.line;
        self.enable_interrupt(edge);

        core::future::poll_fn(|cx| {
            WAKERS[self.line as usize].register(cx.waker());

            // Register before checking FIRED so an interrupt racing this poll
            // is either observed here or wakes the newly registered task.
            if take_fired(mask) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;

        self.disable_interrupt();
    }

    /// Get the corresponding grouped NVIC interrupt.
    pub fn get_interrupt(&self) -> Interrupt {
        match self.line {
            0..=1 => Interrupt::EXTI0_1,
            2..=3 => Interrupt::EXTI2_3,
            4..=15 => Interrupt::EXTI4_15,
            _ => unreachable!(),
        }
    }
}

/// Configure which GPIO port drives an EXTI line.
pub fn configure_exti_source(line: ExtiLine, port: char) {
    assert!(line <= 15, "invalid EXTI line");
    let afio = unsafe { &*Afio::ptr() };
    let source = match port {
        'A' => 0,
        'B' => 1,
        'C' => 2,
        'D' => 3,
        _ => panic!("invalid GPIO port"),
    };

    // ESSR0 covers pins 0..7 and ESSR1 pins 8..15, with one 4-bit field per
    // pin. This is the same layout used by Holtek AFIO_EXTISourceConfig().
    let shift = u32::from(line % 8) * 4;
    let update = |bits: u32| (bits & !(0xf << shift)) | (source << shift);
    if line < 8 {
        afio.essr0()
            .modify(|r, w| unsafe { w.bits(update(r.bits())) });
    } else {
        afio.essr1()
            .modify(|r, w| unsafe { w.bits(update(r.bits())) });
    }
}

fn clear_pending_mask(mask: u32) {
    let exti = unsafe { &*Exti::ptr() };
    // EDGEFLGR and EDGESR are W1C. Clearing both matches Holtek
    // EXTI_ClearEdgeFlag().
    exti.edgeflgr().write(|w| unsafe { w.bits(mask) });
    exti.edgesr().write(|w| unsafe { w.bits(mask) });
    cortex_m::asm::dsb();
}

fn clear_fired(mask: u32) {
    critical_section::with(|cs| {
        let fired = FIRED.borrow(cs);
        fired.set(fired.get() & !mask);
    });
}

fn take_fired(mask: u32) -> bool {
    critical_section::with(|cs| {
        let fired = FIRED.borrow(cs);
        let bits = fired.get();
        fired.set(bits & !mask);
        bits & mask != 0
    })
}

/// Dispatch a grouped EXTI interrupt to the per-line waiters.
pub(crate) fn on_interrupt(group_mask: u32) {
    let exti = unsafe { &*Exti::ptr() };
    // A software-set command stays asserted until explicitly disabled. Treat
    // it as a pending source and drop it before returning from the ISR, as the
    // Holtek EXTI_SWIntCmd(DISABLE) path does.
    let software = exti.sscr().read().bits() & group_mask;
    if software != 0 {
        exti.sscr()
            .modify(|r, w| unsafe { w.bits(r.bits() & !software) });
    }
    let pending = (exti.edgeflgr().read().bits() | software) & exti.cr().read().bits() & group_mask;
    if pending == 0 {
        return;
    }

    // One-shot arm: stop another edge from replacing the edge-status register
    // before the waiting task has observed this one.
    exti.cr()
        .modify(|r, w| unsafe { w.bits(r.bits() & !pending) });
    clear_pending_mask(pending);
    critical_section::with(|cs| {
        let fired = FIRED.borrow(cs);
        fired.set(fired.get() | pending);
    });

    for line in 0..16 {
        if pending & (1 << line) != 0 {
            WAKERS[line].wake();
        }
    }
}

/// Initialize the EXTI peripheral to a known disabled state.
pub fn init() {
    let exti = unsafe { &*Exti::ptr() };
    exti.cr().write(|w| unsafe { w.bits(0) });
    clear_pending_mask(0xffff);
    critical_section::with(|cs| FIRED.borrow(cs).set(0));
}
