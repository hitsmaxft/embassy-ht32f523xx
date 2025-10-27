//! UART (Universal Asynchronous Receiver/Transmitter) driver

use core::marker::PhantomData;
use embassy_sync::waitqueue::AtomicWaker;
use embedded_hal_nb::serial::{ErrorKind};
use embedded_hal_nb::serial::{ErrorType, Read, Write};
use nb;

use crate::pac::{Usart0 as Usart0Pac, Usart1 as Usart1Pac};
use crate::time::Hertz;

/// UART error
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Error {
    /// Framing error
    Framing,
    /// Noise error
    Noise,
    /// RX buffer overrun
    Overrun,
    /// Parity error
    Parity,
    /// Buffer full
    BufferFull,
}

impl embedded_hal_nb::serial::Error for Error {
    fn kind(&self) -> ErrorKind {
        match self {
            Error::Framing => ErrorKind::FrameFormat,
            Error::Noise => ErrorKind::Noise,
            Error::Overrun => ErrorKind::Overrun,
            Error::Parity => ErrorKind::Parity,
            Error::BufferFull => ErrorKind::Other,
        }
    }
}

/// UART TX pin trait
pub trait UartTx<T> {}

/// UART RX pin trait
pub trait UartRx<T> {}

/// UART configuration
#[derive(Debug, Clone)]
pub struct Config {
    /// Baud rate
    pub baudrate: Hertz,
    /// Data bits
    pub data_bits: DataBits,
    /// Stop bits
    pub stop_bits: StopBits,
    /// Parity
    pub parity: Parity,
    /// Enable hardware flow control
    pub hardware_flow_control: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            baudrate: Hertz::hz(115200),
            data_bits: DataBits::Eight,
            stop_bits: StopBits::One,
            parity: Parity::None,
            hardware_flow_control: false,
        }
    }
}

/// Data bits
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DataBits {
    Five,
    Six,
    Seven,
    Eight,
    Nine,
}

/// Stop bits
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum StopBits {
    One,
    Two,
}

/// Parity
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Parity {
    None,
    Even,
    Odd,
}

/// UART instance trait
pub trait Instance {
    /// Get the UART register block
    fn regs() -> &'static crate::pac::usart0::RegisterBlock;

    /// Get the TX waker
    fn tx_waker() -> &'static AtomicWaker;

    /// Get the RX waker
    fn rx_waker() -> &'static AtomicWaker;

    /// Enable UART clock
    fn enable_clock();
}

/// UART0 instance
pub struct Usart0 {
    _private: (),
}

impl Usart0 {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl Instance for Usart0 {
    fn regs() -> &'static crate::pac::usart0::RegisterBlock {
        unsafe { &*Usart0Pac::ptr() }
    }

    fn tx_waker() -> &'static AtomicWaker {
        static WAKER: AtomicWaker = AtomicWaker::new();
        &WAKER
    }

    fn rx_waker() -> &'static AtomicWaker {
        static WAKER: AtomicWaker = AtomicWaker::new();
        &WAKER
    }

    fn enable_clock() {
        let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
        ckcu.apbccr0().modify(|_, w| w.usr0en().set_bit());
    }
}

/// UART1 instance
pub struct Usart1 {
    _private: (),
}

impl Usart1 {
    pub(crate) fn new() -> Self {
        Self { _private: () }
    }
}

impl Instance for Usart1 {
    fn regs() -> &'static crate::pac::usart0::RegisterBlock {
        unsafe { &*Usart1Pac::ptr() }
    }

    fn tx_waker() -> &'static AtomicWaker {
        static WAKER: AtomicWaker = AtomicWaker::new();
        &WAKER
    }

    fn rx_waker() -> &'static AtomicWaker {
        static WAKER: AtomicWaker = AtomicWaker::new();
        &WAKER
    }

    fn enable_clock() {
        let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
        ckcu.apbccr0().modify(|_, w| w.usr1en().set_bit());
    }
}

/// UART driver
pub struct Uart<T: Instance> {
    _instance: PhantomData<T>,
}

impl<T: Instance> Uart<T> {
    /// Create a new UART instance
    pub fn new(
        _uart: T,
        _tx_pin: impl UartTx<T>,
        _rx_pin: impl UartRx<T>,
        config: Config,
    ) -> Self {
        // Enable clock
        T::enable_clock();

        let regs = T::regs();

        // Disable UART while configuring
        regs.usart_usrcr().modify(|_, w| {
            w.urtxen().clear_bit()
             .urrxen().clear_bit()
        });

        // Configure baud rate
        let clock_freq = crate::rcc::get_clocks().apb_clk().to_hz();
        let baudrate = config.baudrate.to_hz();
        let brr = clock_freq / baudrate;
        regs.usart_usrdlr().write(|w| unsafe { w.bits(brr) });

        // Configure data format in control register
        regs.usart_usrcr().modify(|_, w| {
            // Data bits
            let wls = match config.data_bits {
                DataBits::Five => 0b00,
                DataBits::Six => 0b01,
                DataBits::Seven => 0b10,
                DataBits::Eight => 0b11,
                DataBits::Nine => 0b11, // Use 8 bits + parity for 9-bit mode
            };

            // Stop bits
            let nsb = match config.stop_bits {
                StopBits::One => false,
                StopBits::Two => true,
            };

            // Parity
            let (pbe, epe) = match config.parity {
                Parity::None => (false, false),
                Parity::Even => (true, true),
                Parity::Odd => (true, false),
            };

            unsafe {
                w.wls().bits(wls)
                 .nsb().bit(nsb)
                 .pbe().bit(pbe)
                 .epe().bit(epe)
            }
        });

        // Configure FIFOs
        regs.usart_usrfcr().modify(|_, w| unsafe {
            w.rxtl().bits(0b01)      // RX trigger level
             .txtl().bits(0b00)      // TX trigger level
        });

        // Configure interrupts
        regs.usart_usrier().modify(|_, w| {
            w.rxdrie().set_bit()     // RX data ready interrupt
             .txdeie().set_bit()     // TX data empty interrupt
             .oeie().set_bit()       // Overrun error interrupt
        });

        // Enable UART
        regs.usart_usrcr().modify(|_, w| {
            w.urtxen().set_bit()     // TX enable
             .urrxen().set_bit()     // RX enable
        });

        Self {
            _instance: PhantomData,
        }
    }

    /// Write a single byte (blocking)
    pub fn write_byte(&mut self, byte: u8) -> nb::Result<(), Error> {
        let regs = T::regs();

        if regs.usart_usrsifr().read().txde().bit_is_set() {
            regs.usart_usrdr().write(|w| unsafe { w.bits(byte as u32) });
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    /// Read a single byte (blocking)
    pub fn read_byte(&mut self) -> nb::Result<u8, Error> {
        let regs = T::regs();
        let lsr = regs.usart_usrsifr().read();

        // Check for errors
        if lsr.oei().bit_is_set() {
            return Err(nb::Error::Other(Error::Overrun));
        }
        if lsr.pei().bit_is_set() {
            return Err(nb::Error::Other(Error::Parity));
        }
        if lsr.fei().bit_is_set() {
            return Err(nb::Error::Other(Error::Framing));
        }

        if lsr.rxdr().bit_is_set() {
            Ok(regs.usart_usrdr().read().bits() as u8)
        } else {
            Err(nb::Error::WouldBlock)
        }
    }

    /// Write a buffer asynchronously
    pub async fn write(&mut self, buffer: &[u8]) -> Result<(), Error> {
        for &byte in buffer {
            self.write_byte_async(byte).await?;
        }
        Ok(())
    }

    /// Read into a buffer asynchronously
    pub async fn read(&mut self, buffer: &mut [u8]) -> Result<usize, Error> {
        let mut count = 0;
        for slot in buffer.iter_mut() {
            match self.read_byte_async().await {
                Ok(byte) => {
                    *slot = byte;
                    count += 1;
                }
                Err(e) => return Err(e),
            }
        }
        Ok(count)
    }

    async fn write_byte_async(&mut self, byte: u8) -> Result<(), Error> {
        let waker = T::tx_waker();

        core::future::poll_fn(|cx| {
            waker.register(cx.waker());

            match self.write_byte(byte) {
                Ok(()) => core::task::Poll::Ready(Ok(())),
                Err(nb::Error::WouldBlock) => core::task::Poll::Pending,
                Err(nb::Error::Other(e)) => core::task::Poll::Ready(Err(e)),
            }
        }).await
    }

    async fn read_byte_async(&mut self) -> Result<u8, Error> {
        let waker = T::rx_waker();

        core::future::poll_fn(|cx| {
            waker.register(cx.waker());

            match self.read_byte() {
                Ok(byte) => core::task::Poll::Ready(Ok(byte)),
                Err(nb::Error::WouldBlock) => core::task::Poll::Pending,
                Err(nb::Error::Other(e)) => core::task::Poll::Ready(Err(e)),
            }
        }).await
    }

    /// Flush the TX buffer
    pub async fn flush(&mut self) -> Result<(), Error> {
        let regs = T::regs();
        let waker = T::tx_waker();

        core::future::poll_fn(|cx| {
            waker.register(cx.waker());

            if regs.usart_usrsifr().read().txde().bit_is_set() {
                core::task::Poll::Ready(Ok(()))
            } else {
                core::task::Poll::Pending
            }
        }).await
    }
}

// Implement embedded-hal traits
impl<T: Instance> ErrorType for Uart<T> {
    type Error = Error;
}

impl<T: Instance> Write<u8> for Uart<T> {
    fn write(&mut self, word: u8) -> nb::Result<(), Self::Error> {
        self.write_byte(word)
    }

    fn flush(&mut self) -> nb::Result<(), Self::Error> {
        let regs = T::regs();
        if regs.usart_usrsifr().read().txde().bit_is_set() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl<T: Instance> Read<u8> for Uart<T> {
    fn read(&mut self) -> nb::Result<u8, Self::Error> {
        self.read_byte()
    }
}

// TODO: Implement Embassy async traits when embassy-futures is available
// Embassy async implementations would go here