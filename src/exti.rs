//! External Interrupt (EXTI) support for HT32F523xx GPIO pins
//!
//! This module provides external interrupt functionality for GPIO pins,
//! similar to embassy-stm32 EXTI implementation.
//!
//! Note: This is a simplified implementation that focuses on basic functionality.

use crate::pac::{Exti, Afio};
use crate::interrupt::{self};
use crate::pac::Interrupt;

/// EXTI trigger edge configuration
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Edge {
    /// Rising edge trigger
    Rising,
    /// Falling edge trigger
    Falling,
    /// Both rising and falling edges
    RisingFalling,
}

/// EXTI line number (0-15, corresponding to pin numbers)
pub type ExtiLine = u8;

/// EXTI channel - maps GPIO pins to interrupt lines
pub struct ExtiChannel {
    line: ExtiLine,
}

impl ExtiChannel {
    /// Create a new EXTI channel for the given GPIO pin
    pub fn new(pin: u8) -> Option<Self> {
        if pin <= 15 {
            Some(Self { line: pin })
        } else {
            None
        }
    }

    /// Enable the EXTI line with the specified trigger edge
    pub fn enable_interrupt(&self, edge: Edge) {
        // For now, this is a simplified implementation
        // The actual HT32F523xx EXTI register layout needs to be determined
        // from the reference manual or PAC documentation

        let _edge_config = self.get_edge_config(edge);

        // TODO: Implement proper EXTI configuration once PAC register layout is known
        // For now, we'll rely on the NVIC interrupt being enabled

        // Clear any pending interrupt
        self.clear_pending();
    }

    /// Get edge configuration value for HT32 EXTI
    fn get_edge_config(&self, edge: Edge) -> u8 {
        match edge {
            Edge::Rising => 1,    // Rising edge
            Edge::Falling => 2,   // Falling edge
            Edge::RisingFalling => 3, // Both edges
        }
    }

    /// Disable the EXTI line
    pub fn disable_interrupt(&self) {
        // TODO: Implement proper EXTI disable once PAC register layout is known
        // Clear any pending interrupt
        self.clear_pending();
    }

    /// Check if interrupt is pending
    pub fn is_pending(&self) -> bool {
        let exti = unsafe { &*Exti::ptr() };
        (exti.edgeflgr().read().bits() & (1 << self.line)) != 0
    }

    /// Clear pending interrupt
    pub fn clear_pending(&self) {
        let exti = unsafe { &*Exti::ptr() };
        exti.edgeflgr().write(|w| unsafe {
            w.bits(1 << self.line) // Write 1 to clear
        });
    }

    /// Wait for interrupt
    pub async fn wait(&self) {
        let interrupt = self.get_interrupt();
        let waker = interrupt::get_waker(interrupt);

        // Enable interrupt
        self.enable_interrupt(Edge::RisingFalling); // Default to both edges

        // Wait for interrupt
        waker.wait().await;

        // Clear the interrupt flag
        self.clear_pending();
    }

    /// Get the corresponding NVIC interrupt for this EXTI line
    fn get_interrupt(&self) -> Interrupt {
        match self.line {
            0..=1 => Interrupt::EXTI0_1,
            2..=3 => Interrupt::EXTI2_3,
            4..=15 => Interrupt::EXTI4_15,
            _ => panic!("Invalid EXTI line"),
        }
    }
}

/// Configure EXTI source selection (which GPIO port drives which EXTI line)
pub fn configure_exti_source(line: ExtiLine, port: char) {
    let afio = unsafe { &*Afio::ptr() };

    let source_value = match port {
        'A' => 0b00,
        'B' => 0b01,
        'C' => 0b10,
        'D' => 0b11,
        _ => panic!("Invalid GPIO port"),
    };

    // HT32 EXTI source selection is done through AFIO EXTI configuration registers
    match line {
        0..=3 => {
            // EXTI0-3 are controlled by ESSR0 register
            let shift = line * 4;
            afio.essr0().modify(|r, w| unsafe {
                let mut val = r.bits();
                val &= !(0b11 << shift); // Clear the field
                val |= (source_value as u32) << shift; // Set new value
                w.bits(val)
            });
        }
        4..=7 => {
            // EXTI4-7 are controlled by ESSR1 register
            let shift = (line - 4) * 4;
            afio.essr1().modify(|r, w| unsafe {
                let mut val = r.bits();
                val &= !(0b11 << shift); // Clear the field
                val |= (source_value as u32) << shift; // Set new value
                w.bits(val)
            });
        }
        8..=15 => {
            // EXTI8-15 are controlled by ESSR1 register
            let shift = (line - 8) * 4;
            afio.essr1().modify(|r, w| unsafe {
                let mut val = r.bits();
                val &= !(0b11 << shift); // Clear the field
                val |= (source_value as u32) << shift; // Set new value
                w.bits(val)
            });
        }
        _ => panic!("Invalid EXTI line"),
    }
}

/// Initialize EXTI system
pub fn init() {
    // Enable EXTI and AFIO clocks (already done in RCC init)
    // Clear all pending interrupts
    let exti = unsafe { &*Exti::ptr() };
    exti.edgeflgr().write(|w| unsafe { w.bits(0xFFFF) }); // Clear all flags
}