use crate::pac::{Gptm0, Gptm1};
use crate::rcc::Clocks;
use crate::time::{MicroSeconds};
use embedded_hal::delay::DelayNs;

pub struct Timer<TIM> {
    tim: TIM,
    clocks: Clocks,
}

impl Timer<Gptm0> {
    pub fn new(tim: Gptm0, clocks: Clocks) -> Self {
        Timer { tim, clocks }
    }

    pub fn start_count_down<T>(&mut self, _timeout: T) 
    where
        T: Into<MicroSeconds>,
    {
    }

    pub fn wait(&mut self) -> nb::Result<(), core::convert::Infallible> {
        Ok(())
    }
}

impl Timer<Gptm1> {
    pub fn new(tim: Gptm1, clocks: Clocks) -> Self {
        Timer { tim, clocks }
    }

    pub fn start_count_down<T>(&mut self, _timeout: T) 
    where
        T: Into<MicroSeconds>,
    {
    }

    pub fn wait(&mut self) -> nb::Result<(), core::convert::Infallible> {
        Ok(())
    }
}

pub struct Delay<TIM> {
    timer: Timer<TIM>,
}

impl<TIM> Delay<TIM> {
    pub fn new(timer: Timer<TIM>) -> Self {
        Delay { timer }
    }
}

macro_rules! impl_delay {
    ($TIM:ty) => {
        impl DelayNs for Delay<$TIM> {
            fn delay_ns(&mut self, ns: u32) {
                let us = (ns + 999) / 1000;
                self.timer.start_count_down(crate::time::MicroSeconds(us));
                nb::block!(self.timer.wait()).ok();
            }
        }
    };
}

impl_delay!(Gptm0);
impl_delay!(Gptm1);