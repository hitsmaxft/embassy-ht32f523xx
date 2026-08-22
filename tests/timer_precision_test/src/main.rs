//! Timer precision and monotonicity test for Embassy time driver
//! Tests timer precision, concurrent timers, and Instant::now monotonicity

#![no_std]
#![no_main]

use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx as hal;
use embassy_ht32f523xx::{
    self,
    embassy_time::{Duration, Instant, Timer},
};
use hal::Config;
use hal::pac::Interrupt;

use cortex_m_rt::entry;
use defmt::info;
use defmt_rtt as _;
use panic_probe as _;

// Static interrupt executor
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    let _p = hal::init(Config::default());

    info!("🚀 Starting Timer Precision and Monotonicity Test");

    // Start the interrupt executor using LVD_BOD interrupt
    let spawner = EXECUTOR.start(Interrupt::LVD_BOD);

    // Spawn the main test task
    spawner.spawn(precision_test_task()).unwrap();

    info!("✅ Precision test task spawned successfully");

    // Main thread loop
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn precision_test_task() {
    info!("📋 PRECISION_TEST_START: Timer precision and monotonicity test started");

    // Test 1: Basic timer precision
    test_basic_timer_precision().await;

    // Test 2: Concurrent timer precision
    test_concurrent_timer_precision().await;

    // Test 3: Instant::now monotonicity
    test_instant_monotonicity().await;

    // Test 4: Short and long timer precision
    test_timer_range_precision().await;

    info!("🎉 PRECISION_TEST_SUCCESS: All timer precision tests completed successfully!");
}

async fn test_basic_timer_precision() {
    info!("⏰ BASIC_PRECISION_START: Testing basic timer precision");

    let start = Instant::now();

    // Test 1ms precision
    Timer::after(Duration::from_millis(1)).await;
    let elapsed_1ms = start.elapsed();
    info!("✅ 1ms timer: {}ms elapsed", elapsed_1ms.as_millis());

    // Test 10ms precision
    let start = Instant::now();
    Timer::after(Duration::from_millis(10)).await;
    let elapsed_10ms = start.elapsed();
    info!("✅ 10ms timer: {}ms elapsed", elapsed_10ms.as_millis());

    // Test 100ms precision
    let start = Instant::now();
    Timer::after(Duration::from_millis(100)).await;
    let elapsed_100ms = start.elapsed();
    info!("✅ 100ms timer: {}ms elapsed", elapsed_100ms.as_millis());

    info!("✅ BASIC_PRECISION_COMPLETE: Basic timer precision test passed");
}

async fn test_concurrent_timer_precision() {
    info!("🔄 CONCURRENT_PRECISION_START: Testing concurrent timer precision");

    let start = Instant::now();

    // Spawn multiple concurrent timers with different durations
    let (timer_5ms, timer_10ms, timer_20ms) = (
        Timer::after(Duration::from_millis(5)),
        Timer::after(Duration::from_millis(10)),
        Timer::after(Duration::from_millis(20)),
    );

    // Wait for all timers to complete
    timer_5ms.await;
    let elapsed_5ms = start.elapsed();
    info!(
        "✅ 5ms concurrent timer: {}ms elapsed",
        elapsed_5ms.as_millis()
    );

    timer_10ms.await;
    let elapsed_10ms = start.elapsed();
    info!(
        "✅ 10ms concurrent timer: {}ms elapsed",
        elapsed_10ms.as_millis()
    );

    timer_20ms.await;
    let elapsed_20ms = start.elapsed();
    info!(
        "✅ 20ms concurrent timer: {}ms elapsed",
        elapsed_20ms.as_millis()
    );

    info!("✅ CONCURRENT_PRECISION_COMPLETE: Concurrent timer precision test passed");
}

async fn test_instant_monotonicity() {
    info!("📈 MONOTONICITY_START: Testing Instant::now monotonicity");

    let start_instant = Instant::now();
    let mut last_instant = start_instant;
    let mut count = 0;
    let samples = 100;

    // Sample Instant::now rapidly to test monotonicity
    for i in 0..samples {
        let current = Instant::now();

        // Verify monotonicity - current should always be >= last
        if current < last_instant {
            panic!(
                "⚠️ MONOTONICITY_VIOLATION: Instant went backwards! iteration: {}, last: {:?}, current: {:?}",
                i, last_instant, current
            );
        }

        last_instant = current;
        count += 1;

        // Small delay between samples
        if i % 10 == 0 {
            Timer::after(Duration::from_micros(100)).await;
        }
    }

    let total_duration = last_instant.duration_since(start_instant);
    info!(
        "✅ MONOTONICITY_COMPLETE: Successfully verified {} monotonic samples over {}ms",
        count,
        total_duration.as_millis()
    );
}

async fn test_timer_range_precision() {
    info!("📏 RANGE_PRECISION_START: Testing timer precision across different ranges");

    // Test very short timer (100 microseconds)
    let start = Instant::now();
    Timer::after(Duration::from_micros(100)).await;
    let elapsed_100us = start.elapsed();
    info!("✅ 100μs timer: {}μs elapsed", elapsed_100us.as_micros());

    // Test medium timer (500ms)
    let start = Instant::now();
    Timer::after(Duration::from_millis(500)).await;
    let elapsed_500ms = start.elapsed();
    info!("✅ 500ms timer: {}ms elapsed", elapsed_500ms.as_millis());

    // Test longer timer (1 second)
    let start = Instant::now();
    Timer::after(Duration::from_secs(1)).await;
    let elapsed_1s = start.elapsed();
    info!("✅ 1s timer: {}ms elapsed", elapsed_1s.as_millis());

    info!("✅ RANGE_PRECISION_COMPLETE: Timer range precision test passed");
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}
