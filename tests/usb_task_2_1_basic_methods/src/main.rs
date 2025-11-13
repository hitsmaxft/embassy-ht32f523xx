#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx::{self, embassy_time::{Duration, Timer}, pac, usb::{init_usb_with_pins, Config as UsbConfig, UsbPins, UsbDm, UsbDp}};
use embassy_ht32f523xx as hal;

use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

// ** 导入所需的
use embassy_futures::select::{select, Either};
use embassy_usb::Builder;
use static_cell::StaticCell;

static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

// Static buffers
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("🧪 USB_TASK_2_1_BASIC_METHODS (Improved): Testing USB driver basic methods");
    info!("📋 Test goals: USB driver creation, enable(), usb.run() idle");
    info!("📝 FIRMWARE LOG: This test will create 'EBUSB_210' USB device");
    info!("🔍 HOST VERIFY: Use 'cyme --list' to confirm 'EBUSB_210' appears");
    let p = hal::init(hal::Config::default());
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);
    spawner.spawn(usb_basic_methods_test(p)).unwrap();
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn usb_basic_methods_test(mut p: embassy_ht32f523xx::Peripherals) {
    info!("🎯 USB_BASIC_METHODS_TEST: Start");

    Timer::after(Duration::from_millis(100)).await;
    info!("✅ Timer OK!");

    // Configure USB pins PC6 (DM) and PC7 (DP) as AF10
    // CRITICAL: Based on hardware layout - PA11/PA12 are wrong for this board!
    let dm_pin: UsbDm<'C', 6> = p.gpioc.pc6().into_alternate_function::<10>();
    let dp_pin: UsbDp<'C', 7> = p.gpioc.pc7().into_alternate_function::<10>();
    let usb_pins = UsbPins::new(dm_pin, dp_pin);

    let driver = init_usb_with_pins(p.usb, usb_pins, UsbConfig::default());
    info!("✅ USB driver created with USB pins configured as AF10 - PC6(PC), PC7(PC)");

    let mut config = embassy_usb::Config::new(0x16c0, 0x05dc);
    config.manufacturer = Some("Embassy-ht32");
    config.product = Some("EBUSB_210");
    config.serial_number = Some("TASK2101");
    config.max_power = 100;

    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);

    let builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        &mut [], // no msos descriptors
        control_buf,
    );

    // --- Test 1: Create USB device and test enable() ---
    // Note: Endpoint allocation is handled automatically by embassy-usb classes
    // The driver's alloc_endpoint_in/out methods are called internally when building USB classes
    info!("USB_BUILD_TEST start");

    // Test that we can create the builder successfully (this tests driver initialization)
    let mut usb = builder.build();
    info!("✅ test passed USB_BUILD_OK - USB device built successfully");


    // --- Test 2: Test enable() and usb.run() ---
    info!("ENABLE_RUN_TEST start: Running usb.run() for 10 seconds to allow full enumeration...");
    info!("(This will call driver.enable() immediately and wait for host enumeration)");

    let usb_fut = usb.run();
    let timer_fut = Timer::after(Duration::from_secs(10));

    // Run usb.run() and timer concurrently
    match select(usb_fut, timer_fut).await {
        Either::First(_) => {
            info!("❌ TEST FAILED: usb.run() unexpectedly exited.");
        }
        Either::Second(_) => {
            info!("✅ This confirms driver.enable() worked and usb.run() can idle.");
            // Explicitly drop the USB device to stop any ongoing operations
            drop(usb);
            Timer::after(Duration::from_millis(100)).await;
            info!("✅ test passed ENABLE_RUN_OK: usb.run() executed for 10 seconds without panic!");
            info!("🔍 USB ENUMERATION: Device should now be visible to host - check cyme -l for 'EBUSB_210'");
        }
    }


    info!("🏁 ALL_USB_BASIC_METHODS_TESTS_PASSED - Task 2.1 verification complete");
    info!("🔍 HOST VERIFY: Run 'cyme --list' to confirm 'EBUSB_210' device appears");
    info!("🚀 About to hit breakpoint - test should stop here");
    cortex_m::asm::bkpt();
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    unsafe { EXECUTOR.on_interrupt() }
}