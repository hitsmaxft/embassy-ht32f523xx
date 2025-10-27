#![no_main]
#![no_std]

#[macro_use]
mod macros;
mod keymap;
mod vial;

use defmt::info;
use embassy_executor::Spawner;
use embassy_ht32f523xx::gpio;
use embassy_ht32f523xx::usb::Driver;
use keymap::{COL, ROW};
use rmk::channel::EVENT_CHANNEL;
use rmk::config::{BehaviorConfig, RmkConfig, StorageConfig, VialConfig};
use rmk::debounce::default_debouncer::DefaultDebouncer;
use rmk::futures::future::join3;
use rmk::input_device::Runnable;
use rmk::keyboard::Keyboard;
use rmk::matrix::Matrix;
use rmk::storage::async_flash_wrapper;
use rmk::{initialize_keymap_and_storage, run_devices, run_rmk};
use static_cell::StaticCell;
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};
use {defmt_rtt as _, panic_probe as _};

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("RMK HT32 60-Key Keyboard start!");
    
    // Initialize HT32 peripherals
    let p = embassy_ht32f523xx::init(embassy_ht32f523xx::Config::default());
    
    // Initialize GPIO - Our embassy-ht32f523xx doesn't use split() pattern
    
    // USB configuration
    let usb_config = embassy_ht32f523xx::usb::Config::default();
    let driver = Driver::new(p.USB, usb_config);

    // Pin configuration for 60-key matrix (5 rows, 14 columns)
    // For the demo, we'll use simplified pin configuration
    // TODO: Update with actual HT32 pin assignments for your board
    use embassy_ht32f523xx::gpio::{AnyPin, Pin};

    // Create dummy input/output pins as placeholders
    // In a real implementation, these would be actual configured pins
    let input_pins: [AnyPin; ROW] = [
        p.GPIOA.pa0.degrade(),
        p.GPIOA.pa1.degrade(),
        p.GPIOA.pa2.degrade(),
        p.GPIOA.pa3.degrade(),
        p.GPIOA.pa4.degrade(),
    ];

    let mut output_pins: [AnyPin; COL] = [
        p.GPIOB.pb0.degrade(), p.GPIOB.pb1.degrade(),
        p.GPIOB.pb2.degrade(), p.GPIOB.pb3.degrade(),
        p.GPIOB.pb4.degrade(), p.GPIOB.pb5.degrade(),
        p.GPIOB.pb6.degrade(), p.GPIOB.pb7.degrade(),
        p.GPIOB.pb8.degrade(), p.GPIOB.pb9.degrade(),
        p.GPIOB.pb10.degrade(), p.GPIOB.pb11.degrade(),
        p.GPIOB.pb12.degrade(), p.GPIOB.pb13.degrade(),
    ];
    
    // Initialize output pins to low
    use embedded_hal::digital::OutputPin;
    output_pins.iter_mut().for_each(|p| {
        let _ = p.set_low();
    });

    // Initialize HT32 flash storage
    let flash = async_flash_wrapper(p.FLASH);

    // RMK configuration with Vial support
    let unlock_keys = &[(0, 0), (0, 1)]; // ESC + 1 keys to unlock Vial
    let rmk_config = RmkConfig {
        vial_config: VialConfig::new(VIAL_KEYBOARD_ID, VIAL_KEYBOARD_DEF, unlock_keys),
        ..Default::default()
    };

    // Initialize the storage and keymap
    let mut default_keymap = keymap::get_default_keymap();
    let mut behavior_config = BehaviorConfig::default();
    let storage_config = StorageConfig::default();
    let mut positional_config = rmk::config::PositionalConfig::default();

    let (keymap, mut storage) =
        initialize_keymap_and_storage(&mut default_keymap, flash, &storage_config, &mut behavior_config, &mut positional_config).await;

    // Initialize the matrix scanner and keyboard
    let debouncer = DefaultDebouncer::<ROW, COL>::new();
    let mut matrix = Matrix::<_, _, _, ROW, COL, true>::new(input_pins, output_pins, debouncer);
    let mut keyboard = Keyboard::new(&keymap);

    info!("RMK initialized, starting main loop...");

    // Start the main application tasks
    join3(
        run_devices! (
            (matrix) => EVENT_CHANNEL,
        ),
        keyboard.run(),
        run_rmk(&keymap, driver, &mut storage, rmk_config),
    )
    .await;
}

// Flash support is now handled by our HT32 flash driver in embassy_ht32f523xx::flash