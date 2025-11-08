#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_time::{Duration, Timer};
use embedded_hal::digital::OutputPin;
use ht32_bsp::Leds;
use embassy_ht32f523xx::{self, pac::Interrupt, Config};
use {defmt_rtt as _, panic_probe as _};
use cortex_m_rt::entry;

// Static interrupt executor
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    // Initialize HAL
    let config = Config::default();
    let _p = embassy_ht32f523xx::init(config);

    info!("🚀 HT32 Blink Example Started - Using Interrupt-Mode Executor!");

    // Start the interrupt executor using LVD_BOD interrupt (avoids timer conflicts)
    let spawner = EXECUTOR.start(Interrupt::LVD_BOD);

    // Spawn the main blink task
    spawner.spawn(blink_task()).unwrap();

    info!("✅ Blink task spawned successfully");

    // Main thread loop - low priority work or sleep
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn blink_task() {
    info!("🔦 BLINK_TASK_START: Starting LED blink task with interrupt executor");

    let mut leds = Leds::new();
    info!("✅ BLINK_TASK_LEDS_OK: LEDs initialized successfully");

    info!("🔄 BLINK_TASK_LOOP_START: Starting async blink loop with 500ms intervals");
    let mut cycle_count = 0u32;

    // Main blink loop with async timers
    loop {
        cycle_count += 1;

        // Turn LED1 ON, LED2 OFF
        leds.led1.set_high().unwrap();
        leds.led2.set_low().unwrap();
        info!("💡 BLINK_CYCLE_{}_LED1_ON: LED1 ON, LED2 OFF", cycle_count);

        // Use async timer - this validates Timer::await works with interrupt executor
        Timer::after(Duration::from_millis(500)).await;
        info!("⏰ BLINK_CYCLE_{}_TIMER1_OK: First 500ms timer completed", cycle_count);

        // Turn LED1 OFF, LED2 ON
        leds.led1.set_low().unwrap();
        leds.led2.set_high().unwrap();
        info!("🔌 BLINK_CYCLE_{}_LED2_ON: LED1 OFF, LED2 ON", cycle_count);

        // Second async timer
        Timer::after(Duration::from_millis(500)).await;
        info!("⏰ BLINK_CYCLE_{}_TIMER2_OK: Second 500ms timer completed", cycle_count);

        info!("✅ BLINK_CYCLE_{}_COMPLETE: Blink cycle {} completed successfully", cycle_count, cycle_count);

        // Every 10 cycles, log a status message
        if cycle_count % 10 == 0 {
            info!("🎯 BLINK_MILESTONE: Successfully completed {} blink cycles ({} seconds total)",
                  cycle_count, cycle_count);
        }
    }
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}