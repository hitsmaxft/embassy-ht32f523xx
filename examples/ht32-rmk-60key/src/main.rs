//! 60-key keyboard firmware for HT32F52352 (C18 revision) using RMK framework
//!
//! Hardware specifications:
//! - MCU: HT32F52352 (16KB RAM, 128KB Flash)
//! - Layout: 5x14 matrix (60 keys)
//! - Layers: 3 layers (Base, Function, System)
//! - USB: Full-speed USB 2.0 with 8 endpoints
//!
//! This target intentionally uses a fixed keymap without Vial or persistent
//! storage so it can fit the HT32F52352's 16 KiB RAM budget.

#![no_main]
#![no_std]

mod keymap;
#[cfg(feature = "swd-key-inject")]
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_ht32f523xx::usb::Driver;
use keymap::{COL, ROW};
#[cfg(not(feature = "swd-key-inject"))]
use panic_halt as _;
#[cfg(feature = "swd-key-inject")]
use panic_probe as _;
use rmk::config::{BehaviorConfig, PositionalConfig, RmkConfig};
use rmk::debounce::default_debouncer::DefaultDebouncer;
use rmk::keyboard::Keyboard;
use rmk::matrix::Matrix;
use rmk::usb::UsbTransport;
use rmk::{KeymapData, initialize_keymap, run_all};

#[cfg(feature = "swd-key-inject")]
use core::sync::atomic::{AtomicU32, Ordering};
#[cfg(feature = "swd-key-inject")]
use embassy_time::{Duration, Timer};
#[cfg(feature = "swd-key-inject")]
use rmk::core_traits::Runnable;
#[cfg(feature = "swd-key-inject")]
use rmk::event::{KeyboardEvent, publish_event_async};
#[cfg(feature = "swd-key-inject")]
use rmk::hid::{KeyboardReport, Report};
#[cfg(feature = "swd-key-inject")]
use rmk::state::set_usb_state;
#[cfg(feature = "swd-key-inject")]
use rmk_types::connection::UsbState;

/// SWD test command encoding:
/// bit 31 = valid, bit 30 = pressed, bits 15:8 = row, bits 7:0 = column.
#[cfg(feature = "swd-key-inject")]
#[unsafe(no_mangle)]
pub static RMK_SWD_KEY_COMMAND: AtomicU32 = AtomicU32::new(0);

/// The last consumed command, with bit 29 set as an acknowledgement marker.
#[cfg(feature = "swd-key-inject")]
#[unsafe(no_mangle)]
pub static RMK_SWD_KEY_ACK: AtomicU32 = AtomicU32::new(0);

#[cfg(feature = "swd-key-inject")]
struct SwdKeyInjector;

#[cfg(feature = "swd-key-inject")]
impl Runnable for SwdKeyInjector {
    async fn run(&mut self) -> ! {
        loop {
            let command = RMK_SWD_KEY_COMMAND.load(Ordering::Acquire);
            if command & 0x8000_0000 != 0 {
                RMK_SWD_KEY_COMMAND.store(0, Ordering::Release);
                let pressed = command & 0x4000_0000 != 0;
                if command & 0x1000_0000 != 0 {
                    set_usb_state(UsbState::Configured);
                    defmt::info!("SWD forced RMK USB state to Configured");
                    RMK_SWD_KEY_ACK.store(command | 0x2000_0000, Ordering::Release);
                    continue;
                }
                if command & 0x0800_0000 != 0 {
                    let keycodes = if pressed { [4, 0, 0, 0, 0, 0] } else { [0; 6] };
                    rmk::channel::USB_REPORT_CHANNEL
                        .send(Report::KeyboardReport(KeyboardReport {
                            modifier: 0,
                            reserved: 0,
                            leds: 0,
                            keycodes,
                        }))
                        .await;
                    defmt::info!("SWD direct keyboard report pressed={}", pressed);
                    RMK_SWD_KEY_ACK.store(command | 0x2000_0000, Ordering::Release);
                    continue;
                }
                let row = ((command >> 8) & 0xff) as u8;
                let col = (command & 0xff) as u8;
                if row < ROW as u8 && col < COL as u8 {
                    publish_event_async(KeyboardEvent::key(row, col, pressed)).await;
                    defmt::info!("SWD RMK event row={} col={} pressed={}", row, col, pressed);
                    RMK_SWD_KEY_ACK.store(command | 0x2000_0000, Ordering::Release);
                } else {
                    RMK_SWD_KEY_ACK.store(0xe000_0000, Ordering::Release);
                }
            }
            Timer::after(Duration::from_millis(1)).await;
        }
    }
}

// A workspace-wide build unifies Embassy's optional defmt feature through
// other examples. This target does not log, but defmt still requires this Rust
// symbol when its checked arithmetic reaches a panic path.
#[cfg(not(feature = "swd-key-inject"))]
#[unsafe(export_name = "_defmt_panic")]
fn defmt_panic() -> ! {
    loop {
        cortex_m::asm::bkpt();
    }
}

#[cfg(feature = "swd-key-inject")]
#[defmt::panic_handler]
fn defmt_panic() -> ! {
    panic_probe::hard_fault()
}

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialize HT32 peripherals
    let mut p = embassy_ht32f523xx::init(embassy_ht32f523xx::Config::default());

    // USB configuration
    let usb_config = embassy_ht32f523xx::usb::Config::default();
    let driver = Driver::new(p.usb, usb_config);

    // Minimal pin configuration
    use embassy_ht32f523xx::gpio::AnyPin;

    let input_pins: [AnyPin; ROW] = [
        p.gpioa.pa0().degrade(),
        p.gpioa.pa1().degrade(),
        p.gpioa.pa2().degrade(),
        p.gpioa.pa3().degrade(),
        p.gpioa.pa4().degrade(),
    ];

    let output_pins: [AnyPin; COL] = [
        p.gpiob.pb0().degrade(),
        p.gpiob.pb1().degrade(),
        p.gpiob.pb2().degrade(),
        p.gpiob.pb3().degrade(),
        p.gpiob.pb4().degrade(),
        p.gpiob.pb5().degrade(),
        p.gpiob.pb6().degrade(),
        p.gpiob.pb7().degrade(),
        p.gpiob.pb8().degrade(),
        p.gpiob.pb9().degrade(),
        p.gpiob.pb10().degrade(),
        p.gpiob.pb11().degrade(),
        p.gpiob.pb12().degrade(),
        p.gpiob.pb13().degrade(),
    ];

    // Initialize the fixed in-memory keymap. Vial and persistent storage are
    // intentionally excluded from this constrained target.
    let mut keymap_data = KeymapData::new(keymap::get_default_keymap());
    let mut behavior_config = BehaviorConfig::default();
    let positional_config = PositionalConfig::default();

    let keymap =
        initialize_keymap(&mut keymap_data, &mut behavior_config, &positional_config).await;

    // Initialize the matrix scanner and keyboard
    let debouncer = DefaultDebouncer::<ROW, COL>::new();
    let mut matrix = Matrix::<_, _, _, ROW, COL, true>::new(input_pins, output_pins, debouncer);
    let mut keyboard = Keyboard::new(&keymap);

    let mut rmk_config = RmkConfig::default();
    rmk_config.device_config.manufacturer = "Embassy HT32";
    rmk_config.device_config.product_name = "HT32 RMK Integration";
    rmk_config.device_config.serial_number = "HT32-RMK-60K-0001";
    let mut usb_transport = UsbTransport::new(driver, rmk_config.device_config);

    // RMK owns matrix and HID semantics. This target is an integration and
    // resource-budget example for the HT32 USB driver.
    #[cfg(not(feature = "swd-key-inject"))]
    run_all!(matrix, keyboard, usb_transport).await;

    #[cfg(feature = "swd-key-inject")]
    {
        let mut swd_key_injector = SwdKeyInjector;
        run_all!(matrix, keyboard, usb_transport, swd_key_injector).await;
    }
}
