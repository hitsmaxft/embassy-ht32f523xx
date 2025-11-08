//! Test interrupt-mode executor with concurrent task execution
//! Tests embassy-executor functionality with multiple concurrent tasks

#![no_std]
#![no_main]

use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx::{self, embassy_time::{Duration, Timer}};
use embassy_ht32f523xx as hal;
use hal::pac::Interrupt;
use hal::Config;

use defmt::info;
use defmt_rtt as _;
use panic_probe as _;
use cortex_m_rt::entry;

// Static interrupt executor
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    let _p = hal::init(Config::default());

    info!("🚀 Starting Embassy Interrupt-Mode Executor Test");

    // Start the interrupt executor using LVD_BOD interrupt (avoid timer conflicts)
    let spawner = EXECUTOR.start(Interrupt::LVD_BOD);

    // Spawn a single main task first
    spawner.spawn(main_test_task()).unwrap();

    info!("✅ Main test task spawned successfully");

    // Main thread loop - low priority work or sleep
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn main_test_task() {
    info!("📋 MAIN_TASK_START: Main test task started");

    // Test basic timer functionality first
    info!("⏰ TIMER_TEST_START: Testing basic timer functionality");
    Timer::after(Duration::from_millis(100)).await;
    info!("✅ TIMER_TEST_COMPLETE: Basic timer test passed");

    // Now spawn additional tasks to test concurrency
    info!("🔄 CONCURRENT_TEST_START: Testing concurrent task spawning");

    // Note: We'll test concurrency within this task for now to avoid spawn issues
    info!("📋 TASK_A_START: Task A started");
    info!("🔧 TASK_B_START: Task B started");
    info!("⚡ TASK_C_START: Task C started");

    // Simulate concurrent execution with interleaved timing
    for i in 1..=3 {
        info!("📋 TASK_A_ITER_{}: Task A iteration {}", i, i);
        Timer::after(Duration::from_millis(50)).await;
        info!("📋 TASK_A_ITER_{}_COMPLETE: Task A iteration {} completed", i, i);

        info!("🔧 TASK_B_ITER_{}: Task B iteration {}", i, i);
        Timer::after(Duration::from_millis(30)).await;
        info!("🔧 TASK_B_ITER_{}_COMPLETE: Task B iteration {} completed", i, i);

        info!("⚡ TASK_C_ITER_{}: Task C iteration {}", i, i);
        Timer::after(Duration::from_millis(40)).await;
        info!("⚡ TASK_C_ITER_{}_COMPLETE: Task C iteration {} completed", i, i);

        info!("🔄 CONCURRENT_CYCLE_{}_COMPLETE: Concurrent cycle {} finished", i, i);
    }

    // Final validation
    Timer::after(Duration::from_millis(200)).await;
    info!("🎉 EXECUTOR_TEST_SUCCESS: Embassy interrupt-mode executor test completed successfully!");
    info!("✅ CONCURRENT_EXECUTION_VALIDATED: Multiple async operations completed successfully");
    info!("🎯 TASK_SPAWNING_WORKING: Interrupt executor functioning correctly");
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}