#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx as hal;
use embassy_ht32f523xx::usb::{Config as UsbConfig, Driver};
use embassy_ht32f523xx::{self, embassy_time::Duration as HalDuration, pac};
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;

use cortex_m_rt::entry;
use defmt_rtt as _;
use panic_probe as _;
use static_cell::StaticCell;

// Static interrupt executor - prevents timer conflicts with USB
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

// Static allocation for USB device
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static STATE: StaticCell<State> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("🚀 Starting USB Serial Test - HT32F52352");
    info!("📊 Goal: Test bidirectional USB serial communication");
    info!("🔧 Using InterruptExecutor to avoid timer conflicts");

    // Initialize Embassy HAL with USB feature
    let p = hal::init(hal::Config::default());
    info!("✅ Embassy HAL initialized with InterruptExecutor");

    // Start the interrupt executor using LVD_BOD interrupt
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);

    // Spawn the main USB serial test task with peripherals
    spawner.spawn(usb_serial_test_task(p)).unwrap();

    // Main event loop
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn usb_serial_test_task(p: embassy_ht32f523xx::Peripherals) {
    info!("🎯 USB_SERIAL_TEST_TASK: Starting bidirectional communication test");

    // Test timer works before USB operations (critical with InterruptExecutor)
    info!("USB_SERIAL_TIMER_PRE_TEST start, if not USB_SERIAL_TIMER_PRE_OK prints, test failed");
    Timer::after(HalDuration::from_millis(100)).await;
    info!("test passed USB_SERIAL_TIMER_PRE_OK - InterruptExecutor timer works!");

    // Create USB driver with test configuration
    let usb_config = UsbConfig::default();
    let driver = Driver::new(p.usb, usb_config);
    info!("✅ USB driver created");

    // Create embassy-usb config for CDC-ACM serial device
    let mut config = embassy_usb::Config::new(0x16c0, 0x05dc); // Generic test VID/PID
    config.manufacturer = Some("BHE Embassy HT32");
    config.product = Some("BHE HT32F52352 CDC ECHO");
    config.serial_number = Some("BHE-HT32F52352-CDC-0001");
    config.max_power = 100;
    config.supports_remote_wakeup = false;

    info!("🔧 USB CDC-ACM configuration created for serial communication test");

    // Allocate required buffers
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);
    let state = STATE.init(State::new());

    // Create USB builder
    let mut builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        &mut [], // no msos descriptors
        control_buf,
    );

    // Create CDC-ACM class for serial communication
    let mut serial = CdcAcmClass::new(&mut builder, state, 64);

    // Build the USB device
    let mut usb = builder.build();
    info!("🏗️  USB CDC-ACM device built successfully");
    info!("🔌 Connect USB cable to host to start serial communication");
    info!("⏳ Starting bidirectional serial communication test...");

    // Start USB device
    let usb_future = usb.run();
    let serial_future = async {
        let mut buffer = [0u8; 64];

        loop {
            serial.wait_connection().await;
            info!("🔗 CDC-ACM host connected");

            loop {
                match serial.read_packet(&mut buffer).await {
                    Ok(length) => {
                        info!("📥 Received {} bytes, echoing to host", length);
                        if serial.write_packet(&buffer[..length]).await.is_err() {
                            break;
                        }
                    }
                    Err(EndpointError::Disabled) => break,
                    Err(EndpointError::BufferOverflow) => {
                        warn!("CDC receive buffer overflow");
                        break;
                    }
                }
            }

            info!("🔌 CDC-ACM host disconnected");
        }
    };
    let heartbeat_future = heartbeat_task();

    info!("🔄 USB serial communication started - watch for data transfer events");

    // Run USB and heartbeat tasks
    embassy_futures::join::join3(usb_future, serial_future, heartbeat_future).await;
}

async fn heartbeat_task() {
    info!("💓 HEARTBEAT: Starting USB serial test heartbeat");
    let mut count = 0u32;

    loop {
        Timer::after(HalDuration::from_secs(10)).await;
        count += 1;

        info!(
            "💓 USB Serial Test alive #{} - Bidirectional communication active",
            count
        );

        if count % 3 == 0 {
            info!(
                "🔍 USB Serial Test: {} seconds elapsed - Testing full data transfer",
                count * 10
            );
            info!("🎯 Expected events: USB enumeration, CDC-ACM device detection");
            info!("🔌 Connect to USB serial device to test bidirectional communication");
            info!("⏰ Timer should work continuously with InterruptExecutor");
        }
    }
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}
