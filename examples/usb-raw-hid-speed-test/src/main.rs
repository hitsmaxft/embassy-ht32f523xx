#![no_std]
#![no_main]

use cortex_m_rt::entry;
use defmt::*;
use defmt_rtt as _;
use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx as hal;
use embassy_ht32f523xx::pac;
use embassy_ht32f523xx::usb::{Config as UsbConfig, Driver};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::channel::Channel;
use embassy_time::{Duration, Timer};
use embassy_usb::Builder;
use embassy_usb::class::hid::{
    Config as HidConfig, HidBootProtocol, HidReader, HidReaderWriter, HidSubclass, HidWriter, State,
};
use embassy_usb::driver::EndpointError;
use panic_probe as _;
use portable_atomic::{AtomicU32, Ordering};
use static_cell::StaticCell;

const VID: u16 = 0x16c0;
const PID: u16 = 0x05dc;
const REPORT_SIZE: usize = 64;

// Vendor-defined input/output reports only. There are deliberately no
// keyboard, mouse, consumer-control, or other system-input usages.
const RAW_HID_REPORT_DESCRIPTOR: &[u8] = &[
    0x06, 0x00, 0xff, // Usage Page (Vendor Defined 0xFF00)
    0x09, 0x01, // Usage (0x01)
    0xa1, 0x01, // Collection (Application)
    0x15, 0x00, //   Logical Minimum (0)
    0x26, 0xff, 0x00, //   Logical Maximum (255)
    0x75, 0x08, //   Report Size (8 bits)
    0x95, 0x40, //   Report Count (64 bytes)
    0x09, 0x01, //   Usage (Vendor Output)
    0x91, 0x02, //   Output (Data, Variable, Absolute)
    0x09, 0x02, //   Usage (Vendor Input)
    0x81, 0x02, //   Input (Data, Variable, Absolute)
    0xc0, // End Collection
];

static EXECUTOR: InterruptExecutor = InterruptExecutor::new();
static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
static BOS_DESCRIPTOR: StaticCell<[u8; 64]> = StaticCell::new();
static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
static HID_STATE: StaticCell<State> = StaticCell::new();
static ECHO_QUEUE: Channel<CriticalSectionRawMutex, [u8; REPORT_SIZE], 4> = Channel::new();
static RX_PACKETS: AtomicU32 = AtomicU32::new(0);
static TX_PACKETS: AtomicU32 = AtomicU32::new(0);

#[entry]
fn main() -> ! {
    info!("HT32_RAW_HID_SPEED_BOOT");
    let p = hal::init(hal::Config::default());
    let spawner = EXECUTOR.start(pac::Interrupt::LVD_BOD);
    spawner.spawn(raw_hid_test_task(p)).unwrap();
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn raw_hid_test_task(p: hal::Peripherals) {
    let driver = Driver::new(p.usb, UsbConfig::default());
    let mut usb_config = embassy_usb::Config::new(VID, PID);
    usb_config.manufacturer = Some("BHE Embassy HT32");
    usb_config.product = Some("BHE HT32F52352 RAW HID SPEED");
    usb_config.serial_number = Some("BHE-HT32F52352-RAWHID-0001");
    usb_config.max_power = 100;
    usb_config.supports_remote_wakeup = false;

    let mut builder = Builder::new(
        driver,
        usb_config,
        CONFIG_DESCRIPTOR.init([0; 256]),
        BOS_DESCRIPTOR.init([0; 64]),
        &mut [],
        CONTROL_BUF.init([0; 64]),
    );
    let hid_config = HidConfig {
        report_descriptor: RAW_HID_REPORT_DESCRIPTOR,
        request_handler: None,
        poll_ms: 1,
        max_packet_size: REPORT_SIZE as u16,
        hid_subclass: HidSubclass::No,
        hid_boot_protocol: HidBootProtocol::None,
    };
    let hid = HidReaderWriter::<_, REPORT_SIZE, REPORT_SIZE>::new(
        &mut builder,
        HID_STATE.init(State::new()),
        hid_config,
    );
    let (reader, writer) = hid.split();
    let mut usb = builder.build();

    info!("HT32_RAW_HID_ID vid={=u16:04x} pid={=u16:04x}", VID, PID);
    info!("HT32_RAW_HID_PRODUCT BHE HT32F52352 RAW HID SPEED");
    info!("HT32_RAW_HID_READY report=64 poll_ms=1");

    embassy_futures::join::join4(
        usb.run(),
        receive_task(reader),
        transmit_task(writer),
        statistics_task(),
    )
    .await;
}

async fn receive_task<'a>(mut reader: HidReader<'a, Driver<'a>, REPORT_SIZE>) {
    let sender = ECHO_QUEUE.sender();
    let mut announced = false;
    loop {
        reader.ready().await;
        if !announced {
            info!("HT32_RAW_HID_OUT_ENABLED");
            announced = true;
        }
        let mut report = [0u8; REPORT_SIZE];
        match reader.read(&mut report).await {
            Ok(REPORT_SIZE) => {
                RX_PACKETS.fetch_add(1, Ordering::Relaxed);
                sender.send(report).await;
            }
            Ok(length) => warn!("HT32_RAW_HID_SHORT_OUT length={}", length),
            Err(_) => {
                warn!("HT32_RAW_HID_OUT_DISABLED");
                announced = false;
            }
        }
    }
}

async fn transmit_task<'a>(mut writer: HidWriter<'a, Driver<'a>, REPORT_SIZE>) {
    let receiver = ECHO_QUEUE.receiver();
    let mut announced = false;
    loop {
        writer.ready().await;
        if !announced {
            info!("HT32_RAW_HID_IN_ENABLED");
            announced = true;
        }
        let report = receiver.receive().await;
        match writer.write(&report).await {
            Ok(()) => {
                TX_PACKETS.fetch_add(1, Ordering::Relaxed);
            }
            Err(EndpointError::Disabled) => {
                warn!("HT32_RAW_HID_IN_DISABLED");
                announced = false;
            }
            Err(EndpointError::BufferOverflow) => core::unreachable!(),
        }
    }
}

async fn statistics_task() {
    let mut last_rx = 0;
    let mut last_tx = 0;
    loop {
        Timer::after(Duration::from_secs(5)).await;
        let rx = RX_PACKETS.load(Ordering::Relaxed);
        let tx = TX_PACKETS.load(Ordering::Relaxed);
        info!(
            "HT32_RAW_HID_STATS rx_total={} tx_total={} rx_pps={} tx_pps={}",
            rx,
            tx,
            (rx - last_rx) / 5,
            (tx - last_tx) / 5
        );
        last_rx = rx;
        last_tx = tx;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    unsafe { EXECUTOR.on_interrupt() }
}
