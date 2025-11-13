#![no_std]
#![no_main]

use defmt::*;
use embassy_time::Timer;
use embassy_ht32f523xx::{self, embassy_time::Duration as HalDuration, pac};
use embassy_ht32f523xx as hal;

use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    info!("🔍 Starting USB Basic Hardware Check");
    info!("🎯 Goal: Verify USB peripheral is accessible and properly clocked");

    // Initialize Embassy HAL
    let p = hal::init(hal::Config::default());
    info!("✅ Embassy HAL initialized");

    // Test basic USB peripheral access
    let usb = unsafe { &*pac::Usb::ptr() };

    // Read USB CSR register to check if peripheral is accessible
    let csr = usb.csr().read();
    info!("🔌 USB_CSR: Initial state = {:#010x}", csr.bits());

    // Check if USB peripheral clock is enabled
    let ckcu = unsafe { &*pac::Ckcu::ptr() };
    let gcfgr = ckcu.gcfgr().read();
    info!("🕐 GCFGR: USB clock prescaler = {}", gcfgr.usbpre().bits());

    // USB clock configuration already read above
    info!("🕐 GCFGR_USBPRE: USB prescaler = {}", gcfgr.usbpre().bits());

    // Try to write to USB CSR to test if writable
    info!("🔧 Testing USB CSR write access...");
    usb.csr().modify(|_, w| w.pdwn().set_bit()); // Power down
    let csr1 = usb.csr().read();
    usb.csr().modify(|_, w| w.pdwn().clear_bit()); // Power up
    let csr2 = usb.csr().read();

    info!("🔌 USB_CSR after power down = {:#010x}", csr1.bits());
    info!("🔌 USB_CSR after power up = {:#010x}", csr2.bits());

    if csr1.bits() != csr2.bits() {
        info!("✅ USB peripheral is writable - good sign!");
    } else {
        error!("❌ USB peripheral not writable - clock or access issue!");
    }

    // Try to enable pull-up resistor and check
    info!("🔧 Testing USB pull-up resistor...");
    usb.csr().modify(|_, w| w.dppuen().set_bit());

    let csr_pullup = usb.csr().read();
    info!("🔌 USB_CSR with pull-up = {:#010x}", csr_pullup.bits());

    if csr_pullup.dppuen().bit_is_set() {
        info!("✅ USB pull-up resistor enabled - should be visible to host!");
    } else {
        error!("❌ USB pull-up resistor not set!");
    }

    // Check interrupt status
    let isr = usb.isr().read();
    info!("🔌 USB_ISR: Initial interrupt status = {:#010x}", isr.bits());

    // Clear any pending interrupts
    unsafe {
        usb.isr().write(|w| w.bits(0xFFFFFFFF));
    }

    let isr_cleared = usb.isr().read();
    info!("🔌 USB_ISR after clear = {:#010x}", isr_cleared.bits());

    info!("🎯 USB Basic Check Complete - Should see device on host if pull-up worked");

    // Simple heartbeat using busy wait
    let mut count = 0;
    loop {
        // Simple delay loop - not accurate but sufficient for basic check
        for _ in 0..10_000_000 {
            cortex_m::asm::nop();
        }

        count += 1;

        let current_isr = usb.isr().read();
        let current_csr = usb.csr().read();

        info!("💓 USB Check #{}: ISR={:#010x} CSR={:#010x} DPPUEN={}",
              count, current_isr.bits(), current_csr.bits(), current_csr.dppuen().bit_is_set());

        if count == 6 {
            info!("🔍 After 30 seconds: USB device should be visible if initialization worked");
            info!("🔌 Check with 'lsusb' or System Information on macOS");
        }
    }
}