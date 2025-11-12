#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_usb::Builder;
use embassy_time::Timer;
use embassy_ht32f523xx::{self, pac, embassy_time::Duration};
use embassy_ht32f523xx::usb::{Driver, Config as UsbConfig};
use embassy_ht32f523xx as hal;

use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;
use static_cell::StaticCell;

// Static interrupt executor - prevents timer conflicts with USB
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

// Static allocation for USB device
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("🚀 Starting USB RTT Test - HT32F52352");
    info!("📊 Goal: Verify USB enumeration sequence via probe-run RTT logs");
    info!("🔧 Using InterruptExecutor to avoid timer conflicts");

    // Initialize Embassy HAL with USB feature - ONLY ONCE!
    let p = hal::init(hal::Config::default());
    info!("✅ Embassy HAL initialized with InterruptExecutor");

    // Start the interrupt executor using LVD_BOD interrupt
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);

    // Spawn the main USB test task with peripherals
    spawner.spawn(usb_rtt_test_task(p)).unwrap();

    // Main event loop
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn usb_rtt_test_task(p: embassy_ht32f523xx::Peripherals) {
    info!("🎯 USB_RTT_TEST_TASK: USB enumeration monitoring started");

    // Test timer works before USB operations (critical with InterruptExecutor)
    info!("USB_RTT_TIMER_PRE_TEST start, if not USB_RTT_TIMER_PRE_OK prints, test failed");
    Timer::after(Duration::from_millis(100)).await;
    info!("test passed USB_RTT_TIMER_PRE_OK - InterruptExecutor timer works!");

    // Create USB driver with test configuration
    let usb_config = UsbConfig::default();
    let driver = Driver::new(p.usb, usb_config);
    info!("✅ USB driver created");

    // Create embassy-usb config for test device
    let mut config = embassy_usb::Config::new(0x16c0, 0x05dc); // Generic test VID/PID
    config.manufacturer = Some("Embassy-ht32");
    config.product = Some("USB RTT Test");
    config.serial_number = Some("TEST001");
    config.max_power = 100;

    info!("🔧 USB configuration created for RTT enumeration test");

    // Allocate required buffers
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);

    // Create USB builder
    let builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        &mut [], // no msos descriptors
        control_buf,
    );

    // Build the USB device
    let mut usb = builder.build();
    info!("🏗️  USB device built successfully");
    info!("⏳ Starting USB enumeration monitoring...");
    info!("🔌 Connect USB cable to host to trigger enumeration sequence");
    info!("🔍 Expected RTT events: USB reset, EP0 transfers, SETUP packets");

    // Start heartbeat monitoring task
    let heartbeat_future = heartbeat_task();

    info!("🔄 USB RTT monitoring started - watch for enumeration events");

    // Run USB device with concurrent heartbeat
    embassy_futures::join::join(usb.run(), heartbeat_future).await;
}

async fn heartbeat_task() {
    info!("💓 HEARTBEAT: trying to start USB RTT Test heartbeat");
    let mut count = 0u32;

    loop {
        Timer::after(Duration::from_secs(10)).await;
        count += 1;

        info!("💓 USB RTT Test alive #{} - Connect USB cable to see enumeration events", count);

        if count % 6 == 0 {
            info!("🔍 USB RTT Test: {} minutes elapsed - Looking for enumeration logs", count / 6);
            info!("🎯 Expected RTT events: 🔄 USB_IRQ_RESET, 📋 SETUP_PACKET, 📨 USB_IRQ_EP0");
            info!("🔌 Connect USB cable to host system to trigger enumeration sequence");
            info!("⏰ After USB operations, timer should still work with InterruptExecutor");
        }
    }
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}