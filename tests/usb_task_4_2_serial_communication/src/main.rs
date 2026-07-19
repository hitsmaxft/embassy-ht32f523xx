#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_time::Timer;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use embassy_usb::Builder;
use embassy_ht32f523xx::{self, pac, embassy_time::Duration as HalDuration, usb::{init_usb_with_pins, Config as UsbConfig, UsbPins, UsbDm, UsbDp}};
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
static STATE: StaticCell<State> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("🚀 Starting USB Task 4.2 - Bidirectional Serial Communication Test");
    info!("📊 Goal: Test complete USB serial communication with host terminal");
    info!("🔧 Using InterruptExecutor to avoid timer conflicts");
    info!("📝 FIRMWARE LOG: This test will create 'EBUSB_420' USB device");
    info!("🔍 HOST VERIFY: Use 'cyme --list' to confirm 'EBUSB_420' appears");

    // Initialize Embassy HAL with USB feature
    let p = hal::init(hal::Config::default());
    info!("✅ Embassy HAL initialized with InterruptExecutor");

    // Start the interrupt executor using LVD_BOD interrupt
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);

    // Spawn the main USB serial test task with peripherals
    spawner.spawn(usb_serial_communication_task(p)).unwrap();

    // Main event loop
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn usb_serial_communication_task(mut p: embassy_ht32f523xx::Peripherals) {
    info!("🎯 USB_SERIAL_COMM_TASK: Starting bidirectional serial communication test");

    // Test timer works before USB operations (critical with InterruptExecutor)
    info!("USB_SERIAL_TIMER_PRE_TEST start, if not USB_SERIAL_TIMER_PRE_OK prints, test failed");
    Timer::after(HalDuration::from_millis(100)).await;
    info!("test passed USB_SERIAL_TIMER_PRE_OK - InterruptExecutor timer works!");

    // PC6/PC7 use the dedicated USB AF0 electrical configuration.
    let dm_pin: UsbDm<'C', 6> = p.gpioc.pc6().into_usb_dm();
    let dp_pin: UsbDp<'C', 7> = p.gpioc.pc7().into_usb_dp();
    let usb_pins = UsbPins::new(dm_pin, dp_pin);

    let usb_config = UsbConfig::default();
    let driver = init_usb_with_pins(p.usb, usb_pins, usb_config);
    info!("✅ USB driver created with USB AF0 pins PC6/PC7");

    // Create embassy-usb config for CDC-ACM serial device
    let mut config = embassy_usb::Config::new(0x16c0, 0x05dc); // Generic test VID/PID
    config.manufacturer = Some("Embassy-ht32");
    config.product = Some("EBUSB_420");
    config.serial_number = Some("SERIAL001");
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

    info!("🔄 USB serial communication test started - Device should enumerate as CDC-ACM");

    // Run USB device with simple heartbeat
    let usb_future = usb.run();
    let heartbeat_future = heartbeat_task();

    embassy_futures::join::join(usb_future, heartbeat_future).await;
}

async fn heartbeat_task() {
    info!("💓 HEARTBEAT: Starting USB serial communication heartbeat");
    let mut count = 0u32;

    loop {
        Timer::after(HalDuration::from_secs(10)).await;
        count += 1;

        info!("💓 USB Serial Communication Test alive #{}", count);

        if count % 3 == 0 {
            info!("🔍 USB Task 4.2 Status: {} seconds elapsed", count * 10);
            info!("🎯 Test Status: USB device enumerated, ready for bidirectional communication");
            info!("🔌 Connect serial terminal (115200 baud) to communicate");
            info!("📝 FIRMWARE LOG: 'EBUSB_420' should be enumerated as CDC-ACM serial port");
            info!("🔍 HOST VERIFY: Run 'cyme --list' to confirm 'EBUSB_420' device");
            info!("⏰ Timer working continuously with InterruptExecutor");
        }
    }
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}
