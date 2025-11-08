//! Interrupt-mode embassy-time driver test for HT32F523xx
//! Tests basic embassy-time functionality with interrupt executor

#![no_std]
#![no_main]

use embassy_executor::InterruptExecutor;
use embassy_time::{Duration, Instant, Timer};
use embassy_ht32f523xx as hal;
use hal::pac::Interrupt;
use hal::Config;

use defmt::{info, error, warn};
use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

// Static interrupt executor
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    let _p = hal::init(Config::default());

    info!("🚀 Starting embassy-time Interrupt-Mode Test");

    // Start the interrupt executor using LVD_BOD interrupt (unlikely to conflict with timer)
    let spawner = EXECUTOR.start(Interrupt::LVD_BOD);

    // Spawn the main test task
    spawner.spawn(test_task()).unwrap();

    info!("✅ All tasks spawned successfully");

    // Main thread loop - can do other work here or sleep
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn test_task() {
    info!("TIMER_TEST_START: Starting comprehensive timer tests with interrupt executor");

    // Test 1: Instant::now() should work without hanging
    info!("TEST_INSTANT_NOW_START: Testing embassy_time::Instant::now() basic functionality");
    let now1 = Instant::now();
    info!("✅ TEST_INSTANT_NOW_OK: Instant::now() succeeded: {:?}", now1);

    // Small delay to see if time advances
    info!("TEST_TIMER_1MS_START: Testing Timer::after(1ms) - should complete almost immediately, if not [TEST_TIMER_1MS_OK] logged before test ends, this test FAILED");
    let start = Instant::now();
    Timer::after(Duration::from_millis(1)).await;
    let end = Instant::now();
    let elapsed = end.duration_since(start);
    info!("✅ TEST_TIMER_1MS_OK: Timer::after(1ms) completed successfully in {:?}", elapsed);

    let now2 = Instant::now();
    if now2 > now1 {
        info!("✅ TEST_TIME_ADVANCING_OK: Time is advancing correctly: {:?} > {:?}", now2, now1);
        let elapsed = now2.duration_since(now1);
        info!("⏱️  Time advancement elapsed: {:?}", elapsed);
    } else {
        error!("❌ TEST_TIME_ADVANCING_FAILED: Time is not advancing!");
        return;
    }

    // Test 2: Basic Timer::after() functionality with longer delay
    info!("TEST_TIMER_100MS_START: Testing Timer::after(100ms) accuracy test, if not [TEST_TIMER_100MS_OK] logged before test ends, this test FAILED");
    let start = Instant::now();
    Timer::after(Duration::from_millis(100)).await;
    let end = Instant::now();
    let elapsed = end.duration_since(start);
    info!("✅ TEST_TIMER_100MS_OK: Timer::after(100ms) completed in {:?}", elapsed);

    // Check if timing is reasonable (should be close to 100ms)
    if elapsed >= Duration::from_millis(90) && elapsed <= Duration::from_millis(110) {
        info!("✅ TEST_TIMER_100MS_ACCURACY_OK: Timer accuracy is good: ~100ms");
    } else {
        warn!("⚠️  TEST_TIMER_100MS_ACCURACY_WARNING: Timer accuracy may be off: {:?}", elapsed);
    }

    // Test 3: Multiple timer calls to test concurrent timer handling
    info!("TEST_MULTIPLE_TIMERS_START: Testing 3 sequential Timer::after(50ms) calls, if not [TEST_MULTIPLE_TIMERS_OK] logged before test ends, this test FAILED");
    for i in 1..=3 {
        let start = Instant::now();
        Timer::after(Duration::from_millis(50)).await;
        let end = Instant::now();
        let elapsed = end.duration_since(start);
        info!("  Sequential Timer {}: completed in {:?}", i, elapsed);
    }
    info!("✅ TEST_MULTIPLE_TIMERS_OK: All sequential timer tests completed successfully");

    // Test 4: Very short timer test (1ms again to verify consistency)
    info!("TEST_TIMER_SHORT_START: Testing short Timer::after(2ms) for consistency, if not [TEST_TIMER_SHORT_OK] logged before test ends, this test FAILED");
    let start = Instant::now();
    Timer::after(Duration::from_millis(2)).await;
    let end = Instant::now();
    let elapsed = end.duration_since(start);
    info!("✅ TEST_TIMER_SHORT_OK: Short timer (2ms) completed in {:?}", elapsed);

    // Test 5: Instant arithmetic
    info!("TEST_INSTANT_ARITHMETIC_START: Testing Instant arithmetic operations");
    let base = Instant::now();
    let future = base + Duration::from_secs(1);
    let now = Instant::now();

    if future > now {
        info!("✅ TEST_INSTANT_ARITHMETIC_OK: Instant arithmetic works: future > now");
    } else {
        error!("❌ TEST_INSTANT_ARITHMETIC_FAILED: Instant arithmetic failed!");
        return;
    }

    // Test 6: Timer with longer duration (200ms) to test stability
    info!("TEST_TIMER_200MS_START: Testing Timer::after(200ms) stability test, if not [TEST_TIMER_200MS_OK] logged before test ends, this test FAILED");
    let start = Instant::now();
    Timer::after(Duration::from_millis(200)).await;
    let end = Instant::now();
    let elapsed = end.duration_since(start);
    info!("✅ TEST_TIMER_200MS_OK: Long timer (200ms) completed in {:?}", elapsed);

    // Check if longer timing is reasonable
    if elapsed >= Duration::from_millis(180) && elapsed <= Duration::from_millis(220) {
        info!("✅ TEST_TIMER_200MS_ACCURACY_OK: Long timer accuracy is good: ~200ms");
    } else {
        warn!("⚠️  TEST_TIMER_200MS_ACCURACY_WARNING: Long timer accuracy may be off: {:?}", elapsed);
    }

    info!("🎉 ALL_TIMER_TESTS_COMPLETED: All embassy-time timer tests completed successfully!");
    info!("🚀 TIMER_TEST_COMPLETE: embassy-time Timer::await functionality is working on HT32F52352 with interrupt executor!");
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}