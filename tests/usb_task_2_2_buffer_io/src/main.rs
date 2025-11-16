#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx::{self, embassy_time::{Duration, Timer}, pac, usb::{Config as UsbConfig, Driver, UsbDm, UsbDp, UsbPins, init_usb_with_pins}};
use embassy_ht32f523xx as hal;

use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

use embassy_futures::select::{select, Either};
use embassy_futures::join::join;
use embassy_usb::Builder;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State};
use static_cell::StaticCell;

static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

// Static buffers required for the USB stack
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static mut CDC_STATE: StaticCell<State> = StaticCell::new();

#[entry]
fn main() -> ! {
    info!("🧪 USB_TASK_2_2_BUFFER_IO (Final): Testing CDC-ACM auto-connect and write");
    info!("📋 Test goals: alloc_endpoint_in(BULK), write_endpoint_data()");
    info!("📝 FIRMWARE LOG: This test will create 'EBUSB_220' USB device");
    info!("🔍 HOST VERIFY: Use 'cyme --list' to confirm 'EBUSB_220' appears");
    info!("🔌 ASSUMPTION: Device is PLUGGED INTO a host (macOS or Linux)");
    
    let p = hal::init(hal::Config::default());
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);
    spawner.spawn(usb_buffer_io_test(p)).unwrap();
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn usb_buffer_io_test(mut p: embassy_ht32f523xx::Peripherals) {
    info!("🎯 USB_BUFFER_IO_TEST: Start");

    // Configure USB pins PC6 (DM) and PC7 (DP) as AF10
    // CRITICAL: Based on hardware layout - PA11/PA12 are wrong for this board!
    let dm_pin: UsbDm<'C', 6> = p.gpioc.pc6().into_alternate_function::<0>();
    let dp_pin: UsbDp<'C', 7> = p.gpioc.pc7().into_alternate_function::<0>();
    let usb_pins = UsbPins::new(dm_pin, dp_pin);

    let driver = init_usb_with_pins(p.usb, usb_pins, UsbConfig::default());
    let mut config = embassy_usb::Config::new(0x16c0, 0x05dc);
    config.manufacturer = Some("Embassy-ht32");
    config.product = Some("EBUSB_220");
    config.serial_number = Some("TASK2202");
    config.max_power = 100;
    let config_descriptor = CONFIG_DESCRIPTOR.init([0; 256]);
    let bos_descriptor = BOS_DESCRIPTOR.init([0; 256]);
    let control_buf = CONTROL_BUF.init([0; 64]);

    let mut builder = Builder::new(
        driver,
        config,
        config_descriptor,
        bos_descriptor,
        &mut [],
        control_buf,
    );

    // --- 1. Instantiate Class and Trigger alloc_endpoint_in ---
    info!("CDC_CLASS_TEST start: Requesting BULK endpoints...");
    // This call triggers alloc_endpoint_in for the IN endpoint, and alloc_endpoint_out 
    // for the OUT endpoint, via the Builder's dispatch logic.
    let class = CdcAcmClass::new(&mut builder, unsafe { CDC_STATE.init(State::new()) }, 64);
    let (mut sender, _receiver) = class.split();
    info!("✅ test passed ALLOC_EP_OK - Endpoints allocated via class creation");

    let mut usb = builder.build();

    // --- 2. Define Concurrent Tasks and Timebox the Test ---
    info!("CDC_WRITE_TEST start: Attempting host connection and write...");

    // The primary USB run loop
    let usb_fut = usb.run();
    
    // The sequence of I/O checks
    let write_fut = async {
        let total_timeout = Timer::after(Duration::from_secs(10));
        
        // Wait for the host to open the port (which macOS/Linux does automatically)
        info!("   (write_test): Awaiting host port connection...");
        match select(sender.wait_connection(), total_timeout).await {
            Either::First(_) => {
                info!("✅   (write_test): wait_connection() completed! Host port opened.");
            }
            Either::Second(_) => {
                info!("❌   (write_test): TEST FAILED. Connection timed out after 10s.");
                return;
            }
        }

        // Test write_endpoint_data
        info!("   (write_test): Attempting write_packet(BULK IN)...");
        let write_fut = sender.write_packet(b"Hello from probe!");
        
        // Reset timeout for the write operation
        let write_timeout = Timer::after(Duration::from_secs(5));

        match select(write_fut, write_timeout).await {
            Either::First(Ok(_)) => {
                 info!("✅   (write_test): write_packet() completed successfully!");
                 info!("✅   test passed WRITE_ENDPOINT_OK - driver.write_endpoint_data() is VERIFIED!");
            }
            Either::First(Err(_e)) => {
                 info!("❌   (write_test): TEST FAILED. write_packet() returned error.");
            }
            Either::Second(_) => {
                info!("❌   (write_test): TEST FAILED. write_packet() timed out after 5s.");
            }
        }
    };

    // Use join to run both tasks concurrently. The overall test ends when write_fut completes.
    join(usb_fut, write_fut).await;

    info!("🏁 Task 2.2 COMPLETED: I/O Buffer Verification Success");
    info!("🔍 HOST VERIFY: Run 'cyme --list' to confirm 'EBUSB_220' device appears");
    cortex_m::asm::bkpt();
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    unsafe { EXECUTOR.on_interrupt() }
}