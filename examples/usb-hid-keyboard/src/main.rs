#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};
use embassy_usb::class::hid::{HidWriter, State, Config};
use embassy_usb::driver::EndpointError;
use embassy_usb::Builder;
use embedded_hal::digital::InputPin;
use embassy_ht32f523xx::gpio::{Pin, mode};
use embassy_ht32f523xx::usb::{Driver, Config as UsbConfig};
use static_cell::StaticCell;
use usbd_hid::descriptor::{KeyboardReport, SerializedDescriptor};
use panic_probe as _;

use ht32_bsp::Board;
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting USB HID Keyboard example");

    // Initialize Embassy
    let config = embassy_ht32f523xx::Config::default();
    let p = embassy_ht32f523xx::init(config);

    // Initialize board
    let board = Board::new();
    let button = board.user_button;

    info!("Board initialized, setting up USB HID");

    // Create the USB driver
    let usb_config = UsbConfig::default();
    let driver = Driver::new(p.USB, usb_config);

    // Create embassy-usb Config
    let mut config = embassy_usb::Config::new(0xc0de, 0xcafe);
    config.manufacturer = Some("Embassy");
    config.product = Some("HT32 HID Keyboard");
    config.serial_number = Some("12345678");
    config.max_power = 100;
    config.max_packet_size_0 = 64;

    // Required buffers for USB
    static CONFIG_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static BOS_DESCRIPTOR: StaticCell<[u8; 256]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 64]> = StaticCell::new();
    static STATE: StaticCell<State> = StaticCell::new();

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

    // Create HID class with keyboard report descriptor
    let hid_config = Config {
        report_descriptor: KeyboardReport::desc(),
        request_handler: None,
        poll_ms: 60,
        max_packet_size: 8,
    };

    let hid = HidWriter::<_, 8>::new(&mut builder, state, hid_config);

    // Build the USB device
    let mut usb = builder.build();

    // Start the USB task in the background
    let usb_future = usb.run();

    // Start the HID keyboard task
    let hid_future = hid_keyboard_task(hid, button);

    info!("Starting USB device and HID keyboard tasks");

    // Run both tasks concurrently
    embassy_futures::join::join(usb_future, hid_future).await;
}

async fn hid_keyboard_task<'a>(mut hid: HidWriter<'a, Driver<'a>, 8>, mut button: Pin<'B', 12, mode::Input>) {
    info!("Starting HID keyboard task");

    let mut last_button_state = false;
    let mut button_count = 0u32;

    info!("Press the user button (PB12) to send HID keyboard reports");
    info!("Each button press will send 'Hello' via USB HID");

    loop {
        // Read button state
        let button_pressed = match button.is_low() {
            Ok(pressed) => pressed,
            Err(_) => {
                error!("Failed to read button state");
                false
            }
        };

        // Detect button press
        if button_pressed && !last_button_state {
            button_count += 1;
            info!("Button pressed! Count: {} - Sending HID report", button_count);

            // Send "Hello" via HID keyboard
            if let Err(_e) = send_hello_via_hid(&mut hid).await {
                error!("Failed to send HID report");
            } else {
                info!("HID report sent successfully");
            }
        } else if !button_pressed && last_button_state {
            info!("Button released");
        }

        last_button_state = button_pressed;
        Timer::after(Duration::from_millis(50)).await;
    }
}

/// Send "Hello" string via HID keyboard reports
async fn send_hello_via_hid<'a>(hid: &mut HidWriter<'a, Driver<'a>, 8>) -> Result<(), EndpointError> {
    let hello_chars = [
        0x0B, // H
        0x08, // E
        0x0F, // L
        0x0F, // L
        0x12, // O
    ];

    for &keycode in &hello_chars {
        // Key press
        let report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [keycode, 0, 0, 0, 0, 0],
        };

        hid.write_serialize(&report).await?;
        Timer::after(Duration::from_millis(50)).await;

        // Key release
        let release_report = KeyboardReport {
            modifier: 0,
            reserved: 0,
            leds: 0,
            keycodes: [0, 0, 0, 0, 0, 0],
        };

        hid.write_serialize(&release_report).await?;
        Timer::after(Duration::from_millis(50)).await;
    }

    Ok(())
}
