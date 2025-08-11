#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use embassy_usb::class::hid::{HidReaderWriter, ReportId, RequestHandler, State};
use embassy_usb::control::OutResponse;
use embassy_usb::{Builder, Config};
use embedded_hal::digital::InputPin;
use ht32_bsp::Board;
use ht32_hal::prelude::*;
use ht32_hal::rcc::RccExt;
use panic_probe as _;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting USB HID Keyboard example");

    let dp = unsafe { ht32_bsp::pac::Peripherals::steal() };
    
    let rcc = dp.ckcu.constrain();
    let _clocks = rcc.configure()
        .hclk(48.mhz())
        .freeze();

    embassy_ht32::init();

    let board = Board::new();
    let button = board.user_button;

    // Create the USB driver
    let _usb_config = embassy_ht32::usb::Config::default();
    let driver = embassy_ht32::usb::Driver::new();

    // Create embassy-usb Config
    let mut config = Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("HT32 HID Keyboard");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Device descriptor
    config.device_class = 0x00;
    config.device_sub_class = 0x00;
    config.device_protocol = 0x00;
    config.composite_with_iads = true;

    // Create embassy-usb DeviceBuilder
    let mut device_descriptor = [0; 256];
    let mut config_descriptor = [0; 256];
    let mut bos_descriptor = [0; 256];
    let mut control_buf = [0; 64];

    let mut state = State::new();

    let mut builder = Builder::new(
        driver,
        config,
        &mut device_descriptor,
        &mut config_descriptor,
        &mut bos_descriptor,
        &mut control_buf,
    );

    // Create HID Class
    let config = embassy_usb::class::hid::Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 64,
    };

    let hid = HidReaderWriter::<_, 1, 8>::new(&mut builder, &mut state, config);

    // Build the USB device
    let mut usb = builder.build();

    // Run the USB device and keyboard task concurrently
    let usb_fut = usb.run();
    let keyboard_fut = keyboard_task(hid, button);

    join(usb_fut, keyboard_fut).await;
}

struct MyRequestHandler {}

impl RequestHandler for MyRequestHandler {
    fn get_report(&mut self, id: ReportId, _buf: &mut [u8]) -> Option<usize> {
        info!("Get report for {:?}", id);
        None
    }

    fn set_report(&mut self, id: ReportId, data: &[u8]) -> OutResponse {
        info!("Set report for {:?}: {=[u8]}", id, data);
        OutResponse::Accepted
    }

    fn set_idle_ms(&mut self, id: Option<ReportId>, dur: u32) {
        info!("Set idle rate for {:?} to {}ms", id, dur);
    }

    fn get_idle_ms(&mut self, id: Option<ReportId>) -> Option<u32> {
        info!("Get idle rate for {:?}", id);
        None
    }
}

async fn keyboard_task<'a, T>(mut hid: HidReaderWriter<'a, embassy_ht32::usb::Driver<'a>, 1, 8>, mut button: T)
where
    T: InputPin,
{
    info!("Starting keyboard task");

    let mut last_button_state = false;
    let mut report = KeyboardReport {
        modifier: 0,
        reserved: 0,
        leds: 0,
        keycodes: [0; 6],
    };

    loop {
        // Read button state
        let button_pressed = match button.is_low() {
            Ok(pressed) => pressed,
            Err(_) => false,
        };

        // Detect button press/release
        if button_pressed && !last_button_state {
            // Button pressed - send 'A' key
            info!("Button pressed - sending 'A'");
            report.keycodes[0] = 0x04; // HID keycode for 'A'
            
            match hid.write_serialize(&report).await {
                Ok(()) => {},
                Err(e) => error!("Failed to send HID report: {:?}", e),
            }
            
            Timer::after(Duration::from_millis(10)).await;
            
            // Send key release
            report.keycodes[0] = 0;
            match hid.write_serialize(&report).await {
                Ok(()) => {},
                Err(e) => error!("Failed to send key release: {:?}", e),
            }
        }

        last_button_state = button_pressed;
        Timer::after(Duration::from_millis(10)).await;
    }
}