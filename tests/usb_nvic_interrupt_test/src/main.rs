#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_time::Timer;
use embassy_ht32f523xx::{self, pac, embassy_time::Duration as HalDuration};
use embassy_ht32f523xx as hal;

use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

// Static interrupt executor
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    info!("🔧 Starting USB NVIC Interrupt Test");
    info!("🎯 Goal: Verify USB interrupts are working and device enumerates");
    info!("📝 FIRMWARE LOG: This test will create 'EBUSB_NVIC' USB device");
    info!("🔍 HOST VERIFY: Use 'cyme --list' to confirm 'EBUSB_NVIC' appears");

    // Initialize Embassy HAL
    let p = hal::init(hal::Config::default());
    info!("✅ Embassy HAL initialized");

    // Start interrupt executor
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);

    // Spawn USB test task
    spawner.spawn(usb_nvic_test_task(p)).unwrap();

    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn usb_nvic_test_task(p: embassy_ht32f523xx::Peripherals) {
    info!("🎯 USB_NVIC_TEST: Starting USB interrupt verification");

    // Test timer before USB operations
    info!("USB_NVIC_TIMER_PRE_TEST start, if not USB_NVIC_TIMER_PRE_OK prints, test failed");
    Timer::after(HalDuration::from_millis(100)).await;
    info!("test passed USB_NVIC_TIMER_PRE_OK - InterruptExecutor timer works!");

    // Check USB peripheral access and NVIC status
    let usb = unsafe { &*pac::Usb::ptr() };
    let ckcu = unsafe { &*pac::Ckcu::ptr() };

    info!("🔍 PRE_CHECK: USB peripheral status");
    let csr = usb.csr().read();
    let gcfgr = ckcu.gcfgr().read();
    info!("🔌 USB_CSR: {:#010x}", csr.bits());
    info!("🕐 USB prescaler: {}", gcfgr.usbpre().bits());

    // Test if USB interrupt is enabled in NVIC (CRITICAL TEST)
    unsafe {
        // Raw NVIC register access to check if USB interrupt is enabled
        let nvic = 0xE000E100 as *const u32;
        let iser0 = nvic.add(0x100); // ISER[0] offset
        let iser_value = iser0.read_volatile();
        let usb_enabled = (iser_value & (1 << 29)) != 0;
        info!("🔧 NVIC_USB_ISENTRY: USB interrupt enabled = {}", usb_enabled);

        // Check USB interrupt priority
        let ipr7 = nvic.add(0x400 + 7 * 4); // IPR[7] offset (USB is in register 7)
        let ipr_value = ipr7.read_volatile();
        let priority = (ipr_value >> 8) & 0xFF; // USB uses bits 8-15 of IPR[7]
        info!("🔧 NVIC_USB_PRIORITY: USB interrupt priority = {}", priority);
    }

    // Initialize USB with fixed driver
    let usb_config = hal::usb::Config::default();
    let driver = hal::usb::Driver::new(p.usb, usb_config);
    info!("✅ USB driver created with NVIC interrupt support");

    // Create simple USB device (CDC-ACM for easy testing)
    let mut config = embassy_usb::Config::new(0x16c0, 0x05dc);
    config.manufacturer = Some("Embassy-ht32");
    config.product = Some("EBUSB_NVIC");
    config.serial_number = Some("NVIC001");
    config.max_power = 100;

    // Allocate buffers
    static mut CONFIG_DESCRIPTOR: [u8; 256] = [0; 256];
    static mut BOS_DESCRIPTOR: [u8; 256] = [0; 256];
    static mut CONTROL_BUF: [u8; 64] = [0; 64];
    static mut STATE: embassy_usb::class::cdc_acm::State = embassy_usb::class::cdc_acm::State::new();

    // Create USB builder
    let mut builder = embassy_usb::Builder::new(
        driver,
        config,
        unsafe { &mut CONFIG_DESCRIPTOR },
        unsafe { &mut BOS_DESCRIPTOR },
        &mut [], // no msos descriptors
        unsafe { &mut CONTROL_BUF },
    );

    // Create CDC-ACM class (easy to test with serial terminal)
    let mut serial = embassy_usb::class::cdc_acm::CdcAcmClass::new(&mut builder, unsafe { &mut STATE }, 64);

    // Build USB device
    let mut usb = builder.build();
    info!("🏗️  USB device built - NVIC interrupts should be working");

    // Start USB device with heartbeat
    let usb_future = usb.run();
    let heartbeat_future = heartbeat_task();

    info!("🚀 Starting USB device - Should enumerate on host now!");

    embassy_futures::join::join(usb_future, heartbeat_future).await;
}

async fn heartbeat_task() {
    info!("💓 HEARTBEAT: Starting USB NVIC test heartbeat");
    let mut count = 0;
    let _interrupts_seen = 0;

    loop {
        Timer::after(HalDuration::from_secs(5)).await;
        count += 1;

        info!("💓 USB NVIC Test #{} - Device should be enumerated on host", count);

        if count % 3 == 0 {
            info!("🔍 USB NVIC Status Check:");
            info!("📝 FIRMWARE LOG: USB device 'EBUSB_NVIC' should be enumerated");
            info!("📝 FIRMWARE LOG: USB interrupts should be working (reset, setup, SOF)");
            info!("🔍 HOST VERIFY: Run 'cyme --list' to confirm 'EBUSB_NVIC' device");
            info!("🔍 HOST VERIFY: Check 'lsusb' on Linux or System Information on macOS");
            info!("🔍 HOST VERIFY: Connect serial terminal (115200 baud) to test communication");
        }

        if count == 12 {
            info!("🎯 FINAL STATUS: After 60 seconds, USB device should be fully enumerated");
            info!("✅ SUCCESS: USB device visible on host = NVIC interrupts working!");
            info!("❌ FAILURE: No device on host = NVIC interrupts not working");
        }
    }
}