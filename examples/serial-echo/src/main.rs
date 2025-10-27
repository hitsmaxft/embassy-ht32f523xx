#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use embassy_ht32f523xx::{self, Config};
use embassy_time::Timer;
use ht32_bsp::Board;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting serial echo example");

    // Initialize HAL - following the pattern from blink-embassy
    let config = Config::default();
    let _p = embassy_ht32f523xx::init(config);

    // Initialize board-specific pins
    let board = Board::new();

    info!("Embassy HT32 Serial Echo initialized");
    info!("NOTE: UART functionality is not yet fully implemented");
    info!("This example demonstrates the basic embassy-ht32 initialization");

    // TODO: Once UART is fully implemented, the code should look like this:
    /*
    use embassy_ht32f523xx::uart::{Uart, Config as UartConfig};
    use embassy_ht32f523xx::time::Hertz;

    let uart_config = UartConfig {
        baudrate: Hertz::from_raw(115_200),
        ..Default::default()
    };

    // Create UART instance with TX/RX pins from board
    let mut uart = Uart::new(
        p.USART0,           // UART peripheral
        board.uart_tx,      // TX pin
        board.uart_rx,      // RX pin
        uart_config,
    );

    info!("UART initialized at 115200 baud, starting echo loop");

    let welcome_msg = b"HT32 Embassy Serial Echo Ready!\r\n";
    uart.write(welcome_msg).await.unwrap();

    let mut buffer = [0u8; 64];
    loop {
        match uart.read(&mut buffer).await {
            Ok(len) => {
                info!("Received {} bytes", len);
                uart.write(&buffer[..len]).await.unwrap();

                // Add newline for carriage return
                if len > 0 && buffer[0] == b'\r' {
                    uart.write(b"\n").await.unwrap();
                }
            }
            Err(_e) => {
                error!("UART read error occurred");
            }
        }
    }
    */

    // For now, just show system is running with UART pins available
    let _ = board.uart_tx;
    let _ = board.uart_rx;

    loop {
        info!("System running - UART TX/RX pins configured but HAL implementation pending");
        Timer::after_millis(2000).await;
    }
}