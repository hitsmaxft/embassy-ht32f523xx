use crate::hal::gpio::{Pin, Input, Output, GpioExt};
use crate::pac::{Gpioa, Gpiob};

pub struct Board {
    pub led1: Pin<'A', 4, Output>,
    pub led2: Pin<'A', 5, Output>,
    pub led3: Pin<'A', 6, Output>,
    pub user_button: Pin<'B', 12, Input>,
    pub uart_tx: Pin<'A', 2, Input>,
    pub uart_rx: Pin<'A', 3, Input>,
}

impl Board {
    pub fn new() -> Self {
        let gpioa = unsafe { Gpioa::steal() }.split();
        let gpiob = unsafe { Gpiob::steal() }.split();
        
        Self {
            led1: gpioa.pa4.into_push_pull_output(),
            led2: gpioa.pa5.into_push_pull_output(),
            led3: gpioa.pa6.into_push_pull_output(),
            user_button: gpiob.pb12.into_floating_input(),
            uart_tx: gpioa.pa2,
            uart_rx: gpioa.pa3,
        }
    }
}

pub struct Leds {
    pub led1: Pin<'A', 4, Output>,
    pub led2: Pin<'A', 5, Output>,
    pub led3: Pin<'A', 6, Output>,
}

impl Leds {
    pub fn new() -> Self {
        let gpioa = unsafe { Gpioa::steal() }.split();
        
        Self {
            led1: gpioa.pa4.into_push_pull_output(),
            led2: gpioa.pa5.into_push_pull_output(),
            led3: gpioa.pa6.into_push_pull_output(),
        }
    }
}