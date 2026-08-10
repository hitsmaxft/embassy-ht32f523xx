use crate::hal::gpio::{Level, Pin, Speed, mode};

pub struct Board {
    /// LED1 is active-low and connected to PC14.
    pub led1: Pin<'C', 14, mode::Output>,
    /// LED2 is active-low and connected to PC15.
    pub led2: Pin<'C', 15, mode::Output>,
    /// PB12/WAKEUP is routed to CN4 pin 27; it is not an on-board button.
    pub wake_up: Pin<'B', 12, mode::Input>,
    /// USART0 TX routed as M_TX on CN4 pin 13.
    pub uart_tx: Pin<'A', 2, mode::AlternateFunction<6>>,
    /// USART0 RX routed as M_RX on CN4 pin 14.
    pub uart_rx: Pin<'A', 3, mode::AlternateFunction<6>>,
}

impl Board {
    pub fn new() -> Self {
        // Create pins using the new() constructor
        let pc14_input = Pin::<'C', 14, mode::Input>::new();
        let pc15_input = Pin::<'C', 15, mode::Input>::new();
        let pb12_input = Pin::<'B', 12, mode::Input>::new();
        let pa2_input = Pin::<'A', 2, mode::Input>::new();
        let pa3_input = Pin::<'A', 3, mode::Input>::new();

        Self {
            // LEDs are wired from VDD33 to the GPIOs, so high is off.
            led1: pc14_input.into_push_pull_output(Level::High, Speed::Low),
            led2: pc15_input.into_push_pull_output(Level::High, Speed::Low),
            wake_up: pb12_input.into_floating_input(),
            uart_tx: pa2_input.into_alternate_function::<6>(),
            uart_rx: pa3_input.into_alternate_function_input::<6>(),
        }
    }
}

pub struct Leds {
    pub led1: Pin<'C', 14, mode::Output>,
    pub led2: Pin<'C', 15, mode::Output>,
}

impl Leds {
    pub fn new() -> Self {
        // Create pins using the new() constructor
        let pc14_input = Pin::<'C', 14, mode::Input>::new();
        let pc15_input = Pin::<'C', 15, mode::Input>::new();

        Self {
            led1: pc14_input.into_push_pull_output(Level::High, Speed::Low),
            led2: pc15_input.into_push_pull_output(Level::High, Speed::Low),
        }
    }
}
