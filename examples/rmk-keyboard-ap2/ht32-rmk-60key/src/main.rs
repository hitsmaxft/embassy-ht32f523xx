//! 60-key keyboard firmware for HT32F52352 (C18 revision) using RMK framework
//!
//! Hardware specifications:
//! - MCU: HT32F52352 (16KB RAM, 128KB Flash)
//! - Layout: 5x14 matrix (60 keys)
//! - Layers: 3 layers (Base, Function, System)
//! - USB: Full-speed USB 2.0 with 8 endpoints
//!
//! ## Memory Usage with Vial Support
//! ✅ **Vial support**: Successfully enabled and fits in 128KB flash in release mode.
//! ⚠️  **Build requirements**:
//! - Debug builds will overflow flash memory
//! - **Use `cargo build --release`** for Vial support
//! - Release mode optimizations reduce binary size significantly
//! - Estimated flash usage: ~120KB (fits comfortably in 128KB)

#![no_main]
#![no_std]

mod keymap;
mod vial;

use embassy_executor::Spawner;
use embassy_ht32f523xx::usb::Driver;
use keymap::{COL, ROW};
use rmk::channel::EVENT_CHANNEL;
use rmk::config::{BehaviorConfig, PositionalConfig, RmkConfig, StorageConfig, VialConfig};
use rmk::debounce::default_debouncer::DefaultDebouncer;
use rmk::futures::future::join3;
use rmk::input_device::Runnable;
use rmk::keyboard::Keyboard;
use rmk::matrix::Matrix;
use rmk::storage::async_flash_wrapper;
use rmk::{initialize_keymap_and_storage, run_devices, run_rmk};
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};
use {panic_halt as _};

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
        p.gpiob.pb0().degrade(), p.gpiob.pb1().degrade(),
        p.gpiob.pb2().degrade(), p.gpiob.pb3().degrade(),
        p.gpiob.pb4().degrade(), p.gpiob.pb5().degrade(),
        p.gpiob.pb6().degrade(), p.gpiob.pb7().degrade(),
        p.gpiob.pb8().degrade(), p.gpiob.pb9().degrade(),
        p.gpiob.pb10().degrade(), p.gpiob.pb11().degrade(),
        p.gpiob.pb12().degrade(), p.gpiob.pb13().degrade(),
    ];

    // Initialize HT32F52352 flash storage (16KB RAM, 128KB Flash)
    let flash = async_flash_wrapper(p.flash);

    // Initialize the storage and keymap with full RMK functionality
    let mut default_keymap = keymap::get_default_keymap();
    let mut behavior_config = BehaviorConfig::default();
    let storage_config = StorageConfig::default();
    let mut positional_config = PositionalConfig::default();

    let (keymap, mut storage) =
        initialize_keymap_and_storage(&mut default_keymap, flash, &storage_config, &mut behavior_config, &mut positional_config).await;

    // Initialize the matrix scanner and keyboard
    let debouncer = DefaultDebouncer::<ROW, COL>::new();
    let mut matrix = Matrix::<_, _, _, ROW, COL, true>::new(input_pins, output_pins, debouncer);
    let mut keyboard = Keyboard::new(&keymap);

    // RMK configuration with Vial support
    let unlock_keys = &[(0, 0), (0, 1)]; // ESC + 1 keys to unlock Vial
    let rmk_config = RmkConfig {
        vial_config: VialConfig::new(VIAL_KEYBOARD_ID, VIAL_KEYBOARD_DEF, unlock_keys),
        ..Default::default()
    };

    // Run the keyboard firmware with full storage and features
    join3(
        run_devices! (
            (matrix) => EVENT_CHANNEL,
        ),
        keyboard.run(),
        run_rmk(&keymap, driver, &mut storage, rmk_config),
    )
    .await;
}
