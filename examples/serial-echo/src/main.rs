#![no_std]
#![no_main]

use defmt::*;
use embassy_executor::Spawner;
use ht32_hal::prelude::*;
use ht32_hal::rcc::RccExt;
use ht32_hal::uart::Config;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("Starting serial echo example");

    let dp = unsafe { ht32_bsp::pac::Peripherals::steal() };
    
    let rcc = dp.ckcu.constrain();
    let clocks = rcc.configure()
        .hclk(48.mhz())
        .freeze();

    embassy_ht32::init();

    let config = Config {
        baudrate: 115_200.hz(),
        ..Default::default()
    };

    let mut uart = embassy_ht32::uart::Uart::<embassy_ht32::pac::Usart0>::new(dp.usart0, config, &clocks);

    info!("UART initialized, starting echo loop");

    let welcome_msg = b"HT32 Embassy Serial Echo Ready!\r\n";
    uart.write(welcome_msg).await.unwrap();

    let mut buffer = [0u8; 64];
    loop {
        match uart.read(&mut buffer).await {
            Ok(len) => {
                info!("Received {} bytes", len);
                uart.write(&buffer[..len]).await.unwrap();
                
                if buffer[0] == b'\r' {
                    uart.write(b"\n").await.unwrap();
                }
            }
            Err(_e) => {
                error!("UART read error occurred");
            }
        }
    }
}