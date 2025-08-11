use crate::pac::{Usart0, Usart1};
use crate::rcc::Clocks;
use crate::time::Hertz;

pub struct Serial<USART> {
    usart: USART,
    pins: (Tx<USART>, Rx<USART>),
}

pub struct Tx<USART> {
    _usart: core::marker::PhantomData<USART>,
}

pub struct Rx<USART> {
    _usart: core::marker::PhantomData<USART>,
}

#[derive(Debug)]
pub enum Error {
    Overrun,
    Noise,
    Framing,
    Parity,
}

pub struct Config {
    pub baudrate: Hertz,
    pub wordlength: WordLength,
    pub parity: Parity,
    pub stopbits: StopBits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WordLength {
    DataBits8,
    DataBits9,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    ParityNone,
    ParityEven,
    ParityOdd,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopBits {
    STOP1,
    STOP2,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baudrate: Hertz(115_200),
            wordlength: WordLength::DataBits8,
            parity: Parity::ParityNone,
            stopbits: StopBits::STOP1,
        }
    }
}

impl Serial<Usart0> {
    pub fn new(usart: Usart0, _config: Config, _clocks: &Clocks) -> Self {
        Serial {
            usart,
            pins: (
                Tx { _usart: core::marker::PhantomData },
                Rx { _usart: core::marker::PhantomData },
            ),
        }
    }

    pub fn split(self) -> (Tx<Usart0>, Rx<Usart0>) {
        self.pins
    }
}

impl Serial<Usart1> {
    pub fn new(usart: Usart1, _config: Config, _clocks: &Clocks) -> Self {
        Serial {
            usart,
            pins: (
                Tx { _usart: core::marker::PhantomData },
                Rx { _usart: core::marker::PhantomData },
            ),
        }
    }

    pub fn split(self) -> (Tx<Usart1>, Rx<Usart1>) {
        self.pins
    }
}