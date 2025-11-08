#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::InterruptExecutor;
use embassy_ht32f523xx::{self, pac::Interrupt, Config};
use embassy_time::{Duration, Timer};
use ht32_bsp::Board;
use panic_probe as _;
use cortex_m_rt::entry;

// Static interrupt executor
static EXECUTOR: InterruptExecutor = InterruptExecutor::new();

#[entry]
fn main() -> ! {
    info!("🚀 HT32 Serial Echo Example - Using Interrupt-Mode Executor!");

    // Initialize HAL first - this includes time driver initialization
    let config = Config::default();
    let _p = embassy_ht32f523xx::init(config);
    info!("✅ HAL initialization completed - time driver should be ready");

    // Start the interrupt executor using LVD_BOD interrupt (avoids timer conflicts)
    let spawner = EXECUTOR.start(Interrupt::LVD_BOD);

    // Spawn the main serial echo task
    spawner.spawn(serial_echo_task()).unwrap();

    info!("✅ Serial echo task spawned successfully");

    // Main thread loop - low priority work or sleep
    loop {
        cortex_m::asm::wfi();
    }
}

#[embassy_executor::task]
async fn serial_echo_task() {
    info!("📡 SERIAL_TASK_START: Starting serial echo task with interrupt executor");

    // Test embassy timer first
    info!("⏰ SERIAL_TASK_TIMER_TEST: Testing embassy timer integration...");
    Timer::after(Duration::from_millis(100)).await;
    info!("✅ SERIAL_TASK_TIMER_TEST_OK: Embassy timer working correctly");

    // Initialize board-specific pins
    let board = Board::new();
    info!("✅ SERIAL_TASK_BOARD_OK: Board initialized successfully");

    info!("🔌 SERIAL_TASK_UART_PINS: UART TX/RX pins configured");
    // Reserve UART pins for future use
    let _uart_tx = board.uart_tx;
    let _uart_rx = board.uart_rx;

    info!("📋 SERIAL_TASK_STATUS: Embassy HT32 Serial Echo initialized");
    info!("⚠️  SERIAL_TASK_NOTE: UART async functionality is not yet fully implemented");
    info!("🎯 SERIAL_TASK_PURPOSE: This example demonstrates interrupt executor with embassy timers");

    // Future UART implementation structure (commented out):
    /*
    use embassy_ht32f523xx::uart::{Uart, Config as UartConfig};
    use embassy_ht32f523xx::time::Hertz;

    let uart_config = UartConfig {
        baudrate: Hertz::from_raw(115_200),
        ..Default::default()
    };

    // Create UART instance with TX/RX pins from board
    let mut uart = Uart::new(
        p.usart0,           // UART peripheral
        board.uart_tx,      // TX pin
        board.uart_rx,      // RX pin
        uart_config,
    );

    info!("✅ SERIAL_TASK_UART_OK: UART initialized at 115200 baud, starting echo loop");

    let welcome_msg = b"HT32 Embassy Serial Echo Ready!\r\n";
    uart.write(welcome_msg).await.unwrap();

    let mut buffer = [0u8; 64];
    loop {
        match uart.read(&mut buffer).await {
            Ok(len) => {
                info!("📨 SERIAL_TASK_RX: Received {} bytes", len);
                uart.write(&buffer[..len]).await.unwrap();

                // Add newline for carriage return
                if len > 0 && buffer[0] == b'\r' {
                    uart.write(b"\n").await.unwrap();
                }
                info!("📤 SERIAL_TASK_TX: Echoed {} bytes back", len);
            }
            Err(_e) => {
                error!("❌ SERIAL_TASK_ERROR: UART read error occurred");
            }
        }
    }
    */

    // For now, demonstrate interrupt executor with embassy timers
    info!("⏰ SERIAL_TASK_TIMER_START: Starting periodic status messages with embassy timers");
    let mut status_count = 0u32;

    loop {
        status_count += 1;

        // First timer - demonstrates Timer::await integration
        Timer::after(Duration::from_millis(1000)).await;
        info!("⏰ SERIAL_TASK_TIMER1_OK: First 1s timer completed in cycle {}", status_count);

        // Second timer - validates multiple timer operations
        Timer::after(Duration::from_millis(1000)).await;
        info!("⏰ SERIAL_TASK_TIMER2_OK: Second 1s timer completed in cycle {}", status_count);

        // Status message every cycle
        info!("🔄 SERIAL_TASK_CYCLE_{}_COMPLETE: Status cycle {} finished - UART pins ready for future implementation",
              status_count, status_count);

        // Every 5 cycles, provide additional system information
        if status_count % 5 == 0 {
            info!("🎯 SERIAL_TASK_MILESTONE: Completed {} status cycles ({} seconds total) - Embassy timers working perfectly!",
                  status_count, status_count * 2);
        }
    }
}

// Interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn LVD_BOD() {
    // Safety: This is only called from the LVD_BOD interrupt
    unsafe { EXECUTOR.on_interrupt() }
}