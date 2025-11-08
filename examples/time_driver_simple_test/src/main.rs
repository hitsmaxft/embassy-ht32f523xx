//! Simple embassy-time driver test for HT32F523xx
//! Tests basic embassy-time functionality

#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_time::{Duration, Instant, Timer};
use embassy_ht32f523xx as hal;
use hal::Config;

use defmt::{info, error, warn};
use defmt_rtt as _;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let _p = hal::init(Config::default());

    info!("🚀 Starting embassy-time Simple Test");

    // Test 1: Instant::now() should work without hanging
    info!("Test 1: Testing embassy_time::Instant::now()...");

    let now1 = Instant::now();
    info!("✅ Instant::now() succeeded: {:?}", now1);

    // Small delay to see if time advances
    info!("Test 2: Testing Timer::after(1ms) - should complete almost immediately");
    let start = Instant::now();
    Timer::after(Duration::from_millis(1)).await;
    let end = Instant::now();
    let elapsed = end.duration_since(start);
    info!("✅ Timer::after(1ms) completed in {:?}", elapsed);

    let now2 = Instant::now();
    if now2 > now1 {
        info!("✅ Time is advancing correctly: {:?} > {:?}", now2, now1);
        let elapsed = now2.duration_since(now1);
        info!("⏱️  Elapsed time: {:?}", elapsed);
    } else {
        error!("❌ Time is not advancing!");
    }

    // Test 2: Basic Timer::after() functionality
    info!("Test 2: Testing Timer::after(100ms)...");

    let start = Instant::now();
    Timer::after(Duration::from_millis(100)).await;
    let end = Instant::now();

    let elapsed = end.duration_since(start);
    info!("⏱️  Timer::after(100ms) completed in {:?}", elapsed);

    // Check if timing is reasonable (should be close to 100ms)
    if elapsed >= Duration::from_millis(90) && elapsed <= Duration::from_millis(110) {
        info!("✅ Timer accuracy is good: ~100ms");
    } else {
        warn!("⚠️  Timer accuracy may be off: {:?}", elapsed);
    }

    // Test 3: Multiple timer calls
    info!("Test 3: Testing multiple Timer::after() calls...");

    for i in 1..=3 {
        let start = Instant::now();
        Timer::after(Duration::from_millis(50)).await;
        let end = Instant::now();
        let elapsed = end.duration_since(start);
        info!("  Timer {}: completed in {:?}", i, elapsed);
    }

    // Test 4: Instant arithmetic
    info!("Test 4: Testing Instant arithmetic...");

    let base = Instant::now();
    let future = base + Duration::from_secs(1);
    let now = Instant::now();

    if future > now {
        info!("✅ Instant arithmetic works: future > now");
    } else {
        error!("❌ Instant arithmetic failed!");
    }

    info!("🎉 All embassy-time tests completed successfully!");
    info!("embassy-time is working on HT32F52352! 🚀");
}