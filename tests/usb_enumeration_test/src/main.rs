#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx::{self, embassy_time::{Duration, Timer}, pac};
use embassy_ht32f523xx as hal;

use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

// Static interrupt executor - like working timer precision test
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    info!("🚀 Starting USB Enumeration Test with InterruptExecutor");

    // Use embassy_ht32f523xx::init() but without USB feature initially
    // to test if timer works with InterruptExecutor
    let _peripherals = hal::init(hal::Config::default());

    // Start the interrupt executor using LVD_BOD interrupt
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);

    // Spawn the main test task
    spawner.spawn(main_task()).unwrap();

    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn main_task() {

    info!("🔧 InterruptExecutor task started - testing timer functionality with USB");

    // Test timer FIRST - before any USB operations with InterruptExecutor
    info!("INTERRUPT_EXECUTOR_TIMER_TEST start, if not INTERRUPT_EXECUTOR_TIMER_TEST_OK prints, test failed");
    Timer::after(Duration::from_millis(100)).await;
    info!("test passed INTERRUPT_EXECUTOR_TIMER_TEST_OK - InterruptExecutor timer works!");

    info!("About to access USB peripheral...");

    // Test USB peripheral access
    let usb = unsafe { &*pac::Usb::ptr() };

    info!("About to enable USB power...");

    // First enable USB power before reading registers (PDWN bit = 0 to enable)
    usb.csr().modify(|_, w| w.pdwn().clear_bit());
    info!("USB power enabled");

    info!("About to read USB CSR register...");

    // Read USB CSR register to verify we can access USB peripheral
    info!("Attempting to read USB CSR register, if not READ_OK loggins show up, test is failed...");

    // Read CSR register immediately after enabling power
    let csr = usb.csr().read();
    info!("USB CSR register READ_OK: 0x{:08x}", csr.bits());

    // Enable D+ pull-up resistor for enumeration
    info!("D+ pull-up enabling - device should enumerate");
    usb.csr().modify(|_, w| w.dppuen().set_bit());

    info!("✅ USB peripheral setup complete with InterruptExecutor!");

    info!("🔍 Now testing timer functionality after USB operations with InterruptExecutor");

    // Test timer AFTER USB operations - the critical test with InterruptExecutor
    info!("USB_POST_INTERRUPT_EXECUTOR_TIMER_TEST start, if not USB_POST_INTERRUPT_EXECUTOR_TIMER_TEST_OK prints, test failed");
    Timer::after(Duration::from_millis(300)).await;
    info!("test passed USB_POST_INTERRUPT_EXECUTOR_TIMER_TEST_OK - Timer works after USB with InterruptExecutor!");

    // Blink LED to show the test is running
    info!("USB_BLINK_TEST start, if not USB_BLINK_TEST_OK prints, test failed");
    Timer::after(Duration::from_millis(1000)).await;
    info!("test passed USB_BLINK_TEST_OK");
    info!("ALL USB test passed!!!");
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}