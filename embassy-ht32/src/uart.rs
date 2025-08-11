use core::marker::PhantomData;
use ht32_hal::uart::{Config, Error};
use ht32f523x2::{Usart0, Usart1};

pub struct UartTx<T> {
    phantom: PhantomData<T>,
}

pub struct UartRx<T> {
    phantom: PhantomData<T>,
}

pub struct Uart<T> {
    phantom: PhantomData<T>,
}

impl Uart<Usart0> {
    pub fn new(_usart: Usart0, _config: Config, _clocks: &ht32_hal::rcc::Clocks) -> Self {
        Uart {
            phantom: PhantomData,
        }
    }

    pub fn split(self) -> (UartTx<Usart0>, UartRx<Usart0>) {
        (
            UartTx { phantom: PhantomData },
            UartRx { phantom: PhantomData },
        )
    }

    pub async fn write(&mut self, _buf: &[u8]) -> Result<(), Error> {
        Ok(())
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        Ok(buf.len())
    }
}

impl Uart<Usart1> {
    pub fn new(_usart: Usart1, _config: Config, _clocks: &ht32_hal::rcc::Clocks) -> Self {
        Uart {
            phantom: PhantomData,
        }
    }

    pub fn split(self) -> (UartTx<Usart1>, UartRx<Usart1>) {
        (
            UartTx { phantom: PhantomData },
            UartRx { phantom: PhantomData },
        )
    }
}