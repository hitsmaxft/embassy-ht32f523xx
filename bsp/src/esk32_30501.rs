use crate::hal::gpio::{Pin, mode, Level, Speed};

pub struct Board {
    pub led1: Pin<'C', 14, mode::Output>,
    pub led2: Pin<'C', 15, mode::Output>,
    pub user_button: Pin<'B', 12, mode::Input>,
    pub uart_tx: Pin<'A', 2, mode::Input>,
    pub uart_rx: Pin<'A', 3, mode::Input>,
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
            led1: pc14_input.into_push_pull_output(Level::Low, Speed::Low),
            led2: pc15_input.into_push_pull_output(Level::Low, Speed::Low),
            user_button: pb12_input.into_floating_input(),
            uart_tx: pa2_input,
            uart_rx: pa3_input,
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
            led1: pc14_input.into_push_pull_output(Level::Low, Speed::Low),
            led2: pc15_input.into_push_pull_output(Level::Low, Speed::Low),
        }
    }
}