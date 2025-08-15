#![no_main]
#![no_std]

#[macro_use]
mod macros;
mod keymap;
mod vial;

use defmt::info;
use embassy_executor::Spawner;
use embassy_ht32::hal::gpio::GpioExt;
use embassy_ht32::usb::Driver;
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
    let p = embassy_ht32::pac::Peripherals::take().unwrap();
    
    // Initialize GPIO
    let gpioa = p.GPIOA.split();
    let gpiob = p.GPIOB.split();
    
    // USB configuration
    static EP_OUT_BUFFER: StaticCell<[u8; 1024]> = StaticCell::new();
    let driver = Driver::new(
        p.USB,
        &mut EP_OUT_BUFFER.init([0; 1024])[..],
    );

    // Pin configuration for 60-key matrix (5 rows, 14 columns)
    // Rows: PA0, PA1, PA2, PA3, PA4 (input pins)
    let input_pins = [gpioa.pa0.into_floating_input(), gpioa.pa1.into_floating_input(), 
                      gpioa.pa2.into_floating_input(), gpioa.pa3.into_floating_input(), 
                      gpioa.pa4.into_floating_input()];
    
    // Cols: PB0-PB13 (output pins)
    let mut output_pins = [gpiob.pb0.into_push_pull_output(), gpiob.pb1.into_push_pull_output(),
                           gpiob.pb2.into_push_pull_output(), gpiob.pb3.into_push_pull_output(),
                           gpiob.pb4.into_push_pull_output(), gpiob.pb5.into_push_pull_output(),
                           gpiob.pb6.into_push_pull_output(), gpiob.pb7.into_push_pull_output(),
                           gpiob.pb8.into_push_pull_output(), gpiob.pb9.into_push_pull_output(),
                           gpiob.pb10.into_push_pull_output(), gpiob.pb11.into_push_pull_output(),
                           gpiob.pb12.into_push_pull_output(), gpiob.pb13.into_push_pull_output()];
    
    // Initialize output pins to low
    use embedded_hal::digital::OutputPin;
    output_pins.iter_mut().for_each(|p| {
        let _ = p.set_low();
    });

    // Create a dummy flash storage for now (will need proper implementation)
    // TODO: Implement proper flash storage for HT32
    let flash = async_flash_wrapper(DummyFlash::new());

    // RMK configuration with Vial support
    let rmk_config = RmkConfig {
        vial_config: VialConfig::new(VIAL_KEYBOARD_ID, VIAL_KEYBOARD_DEF),
        ..Default::default()
    };

    // Initialize the storage and keymap
    let mut default_keymap = keymap::get_default_keymap();
    let behavior_config = BehaviorConfig::default();
    let storage_config = StorageConfig::default();

    let (keymap, mut storage) =
        initialize_keymap_and_storage(&mut default_keymap, flash, &storage_config, behavior_config).await;

    // Initialize the matrix scanner and keyboard
    let debouncer = DefaultDebouncer::<ROW, COL>::new();
    let mut matrix = Matrix::<_, _, _, ROW, COL>::new(input_pins, output_pins, debouncer);
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

// Dummy flash implementation for compilation (needs proper implementation)
struct DummyFlash;

impl DummyFlash {
    fn new() -> Self {
        Self
    }
}

use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash};

impl ErrorType for DummyFlash {
    type Error = ();
}

impl ReadNorFlash for DummyFlash {
    const READ_SIZE: usize = 1;

    fn read(&mut self, _offset: u32, _bytes: &mut [u8]) -> Result<(), Self::Error> {
        Ok(())
    }

    fn capacity(&self) -> usize {
        4096
    }
}

impl NorFlash for DummyFlash {
    const WRITE_SIZE: usize = 1;
    const ERASE_SIZE: usize = 4096;

    fn erase(&mut self, _from: u32, _to: u32) -> Result<(), Self::Error> {
        Ok(())
    }

    fn write(&mut self, _offset: u32, _bytes: &[u8]) -> Result<(), Self::Error> {
        Ok(())
    }
}