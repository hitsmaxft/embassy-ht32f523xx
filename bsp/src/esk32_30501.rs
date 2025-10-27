use crate::hal::gpio::{Pin, mode, Level, Speed};

pub struct Board {
    pub led1: Pin<'A', 4, mode::Output>,
    pub led2: Pin<'A', 5, mode::Output>,
    pub led3: Pin<'A', 6, mode::Output>,
    pub user_button: Pin<'B', 12, mode::Input>,
    pub uart_tx: Pin<'A', 2, mode::Input>,
    pub uart_rx: Pin<'A', 3, mode::Input>,
}

impl Board {
    pub fn new() -> Self {
        // Create pins using the new() constructor
        let pa4_input = Pin::<'A', 4, mode::Input>::new();
        let pa5_input = Pin::<'A', 5, mode::Input>::new();
        let pa6_input = Pin::<'A', 6, mode::Input>::new();
        let pb12_input = Pin::<'B', 12, mode::Input>::new();
        let pa2_input = Pin::<'A', 2, mode::Input>::new();
        let pa3_input = Pin::<'A', 3, mode::Input>::new();

        Self {
            led1: pa4_input.into_push_pull_output(Level::Low, Speed::Low),
            led2: pa5_input.into_push_pull_output(Level::Low, Speed::Low),
            led3: pa6_input.into_push_pull_output(Level::Low, Speed::Low),
            user_button: pb12_input.into_floating_input(),
            uart_tx: pa2_input,
            uart_rx: pa3_input,
        }
    }
}

pub struct Leds {
    pub led1: Pin<'A', 4, mode::Output>,
    pub led2: Pin<'A', 5, mode::Output>,
    pub led3: Pin<'A', 6, mode::Output>,
}

impl Leds {
    pub fn new() -> Self {
        // Create pins using the new() constructor
        let pa4_input = Pin::<'A', 4, mode::Input>::new();
        let pa5_input = Pin::<'A', 5, mode::Input>::new();
        let pa6_input = Pin::<'A', 6, mode::Input>::new();

        Self {
            led1: pa4_input.into_push_pull_output(Level::Low, Speed::Low),
            led2: pa5_input.into_push_pull_output(Level::Low, Speed::Low),
            led3: pa6_input.into_push_pull_output(Level::Low, Speed::Low),
        }
    }
}