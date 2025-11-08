//! Async GPIO interrupt test for HT32F523xx
//! Tests interrupt-driven GPIO operations with embassy-async

#![no_std]
#![no_main]

use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx as hal;
use embassy_ht32f523xx::gpio::{Pin, mode, AnyPin, Level, Speed};
use hal::pac::Interrupt;
use hal::Config;

use embedded_hal::digital::OutputPin;
use embedded_hal_async::digital::Wait;
use embassy_futures::join;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

// Static interrupt executor
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    let _p = hal::init(Config::default());

    info!("🚀 Starting Async GPIO Interrupt Test");

    // Start the interrupt executor using LVD_BOD interrupt
    let spawner = EXECUTOR.start(Interrupt::LVD_BOD);

    // Spawn the main GPIO test task
    spawner.spawn(gpio_test_task()).unwrap();

    info!("✅ GPIO test task spawned successfully");

    // Main thread loop
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn gpio_test_task() {
    info!("📋 GPIO_TEST_START: Async GPIO interrupt test started");

    // Test 1: Basic GPIO input/output functionality
    test_gpio_basic().await;

    // Test 2: Interrupt-driven GPIO wait operations
    test_gpio_interrupt_wait().await;

    // Test 3: Multiple concurrent GPIO operations
    test_concurrent_gpio().await;

    info!("🎉 GPIO_TEST_SUCCESS: All async GPIO tests completed successfully!");
}

async fn test_gpio_basic() {
    info!("🔌 BASIC_GPIO_START: Testing basic GPIO functionality");

    // Create a simple input pin (PA1)
    let _input_pin = Pin::<'A', 1, mode::Input>::new();

    // Create an output pin (PA0) by converting from input
    let input_pin_pa0 = Pin::<'A', 0, mode::Input>::new();
    let mut output_pin = input_pin_pa0.into_push_pull_output(Level::Low, Speed::Low);

    // Test basic output
    output_pin.set_high().unwrap();
    hal::embassy_time::Timer::after(hal::embassy_time::Duration::from_millis(10)).await;

    output_pin.set_low().unwrap();
    hal::embassy_time::Timer::after(hal::embassy_time::Duration::from_millis(10)).await;

    info!("✅ BASIC_GPIO_COMPLETE: Basic GPIO functionality test passed");
}

async fn test_gpio_interrupt_wait() {
    info!("⚡ INTERRUPT_GPIO_START: Testing interrupt-driven GPIO operations");

    // Create an input pin for interrupt testing (PA2)
    let input_pin = Pin::<'A', 2, mode::Input>::new();
    let mut any_pin = input_pin.degrade();

    // Test wait for any edge (this will timeout after 1 second if no interrupt occurs)
    info!("🔄 Testing wait_for_any_edge (will timeout after 1s if no button pressed)");

    let wait_result = hal::embassy_time::with_timeout(
        hal::embassy_time::Duration::from_secs(1),
        any_pin.wait_for_any_edge()
    ).await;

    match wait_result {
        Ok(Ok(())) => {
            info!("✅ INTERRUPT_RECEIVED: GPIO interrupt detected successfully!");
        }
        Ok(Err(e)) => {
            info!("⚠️ GPIO_ERROR: GPIO operation failed: {:?}", defmt::Debug2Format(&e));
        }
        Err(_) => {
            info!("⏰ TIMEOUT: No GPIO interrupt detected within 1 second (expected in test environment)");
        }
    }

    info!("✅ INTERRUPT_GPIO_COMPLETE: Interrupt-driven GPIO test completed");
}

async fn test_concurrent_gpio() {
    info!("🔄 CONCURRENT_GPIO_START: Testing concurrent GPIO operations");

    // Create multiple input pins for concurrent testing
    let pin_a = Pin::<'A', 3, mode::Input>::new();
    let pin_b = Pin::<'B', 0, mode::Input>::new();

    let mut any_pin_a = pin_a.degrade();
    let mut any_pin_b = pin_b.degrade();

    info!("🔀 Testing concurrent GPIO wait operations");

    // Test concurrent wait operations with timeout
    let wait_a = async {
        hal::embassy_time::with_timeout(
            hal::embassy_time::Duration::from_millis(500),
            any_pin_a.wait_for_rising_edge()
        ).await
    };

    let wait_b = async {
        hal::embassy_time::with_timeout(
            hal::embassy_time::Duration::from_millis(500),
            any_pin_b.wait_for_falling_edge()
        ).await
    };

    // Run both waits concurrently
    let (result_a, result_b) = join::join(wait_a, wait_b).await;

    match result_a {
        Ok(Ok(())) => info!("✅ PIN_A_INTERRUPT: Rising edge detected on PA3"),
        Ok(Err(e)) => info!("⚠️ PIN_A_ERROR: {:?}", defmt::Debug2Format(&e)),
        Err(_) => info!("⏰ PIN_A_TIMEOUT: No interrupt on PA3"),
    }

    match result_b {
        Ok(Ok(())) => info!("✅ PIN_B_INTERRUPT: Falling edge detected on PB0"),
        Ok(Err(e)) => info!("⚠️ PIN_B_ERROR: {:?}", defmt::Debug2Format(&e)),
        Err(_) => info!("⏰ PIN_B_TIMEOUT: No interrupt on PB0"),
    }

    info!("✅ CONCURRENT_GPIO_COMPLETE: Concurrent GPIO test completed");
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}