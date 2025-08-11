use crate::pac::{Gpioa, Gpiob, Gpioc};
use embedded_hal::digital::{InputPin, OutputPin, StatefulOutputPin, ErrorType};

pub trait GpioExt {
    type Parts;
    fn split(self) -> Self::Parts;
}

pub enum Mode<MODE> {
    Input(MODE),
    Output(MODE),
}

pub struct Input;
pub struct Output;

pub struct Pin<const P: char, const N: u8, MODE = Input> {
    _mode: core::marker::PhantomData<MODE>,
}

impl<const P: char, const N: u8, MODE> ErrorType for Pin<P, N, MODE> {
    type Error = core::convert::Infallible;
}

impl<const P: char, const N: u8, MODE> Pin<P, N, MODE> {
    pub fn into_push_pull_output(self) -> Pin<P, N, Output> {
        Pin { _mode: core::marker::PhantomData }
    }

    pub fn into_floating_input(self) -> Pin<P, N, Input> {
        Pin { _mode: core::marker::PhantomData }
    }
}

impl<const P: char, const N: u8> OutputPin for Pin<P, N, Output> {
    fn set_high(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn set_low(&mut self) -> Result<(), Self::Error> {
        Ok(())
    }
}

impl<const P: char, const N: u8> StatefulOutputPin for Pin<P, N, Output> {
    fn is_set_high(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn is_set_low(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

impl<const P: char, const N: u8> InputPin for Pin<P, N, Input> {
    fn is_high(&mut self) -> Result<bool, Self::Error> {
        Ok(false)
    }

    fn is_low(&mut self) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

macro_rules! gpio {
    ($GPIOX:ident, $gpiox:ident, $PXx:ident, $extigpion:ident, $port:expr, [
        $($PXi:ident: ($pxi:ident, $i:expr),)+
    ]) => {
        pub mod $gpiox {
            use super::*;

            pub struct Parts {
                $(
                    pub $pxi: Pin<$port, $i, Input>,
                )+
            }

            impl GpioExt for $GPIOX {
                type Parts = Parts;

                fn split(self) -> Parts {
                    Parts {
                        $(
                            $pxi: Pin { _mode: core::marker::PhantomData },
                        )+
                    }
                }
            }
        }
    };
}

gpio!(Gpioa, gpioa, PAx, exti_gpioa, 'A', [
    PA0: (pa0, 0),
    PA1: (pa1, 1),
    PA2: (pa2, 2),
    PA3: (pa3, 3),
    PA4: (pa4, 4),
    PA5: (pa5, 5),
    PA6: (pa6, 6),
    PA7: (pa7, 7),
    PA8: (pa8, 8),
    PA9: (pa9, 9),
    PA10: (pa10, 10),
    PA11: (pa11, 11),
    PA12: (pa12, 12),
    PA13: (pa13, 13),
    PA14: (pa14, 14),
    PA15: (pa15, 15),
]);

gpio!(Gpiob, gpiob, PBx, exti_gpiob, 'B', [
    PB0: (pb0, 0),
    PB1: (pb1, 1),
    PB2: (pb2, 2),
    PB3: (pb3, 3),
    PB4: (pb4, 4),
    PB5: (pb5, 5),
    PB6: (pb6, 6),
    PB7: (pb7, 7),
    PB8: (pb8, 8),
    PB9: (pb9, 9),
    PB10: (pb10, 10),
    PB11: (pb11, 11),
    PB12: (pb12, 12),
    PB13: (pb13, 13),
    PB14: (pb14, 14),
    PB15: (pb15, 15),
]);

gpio!(Gpioc, gpioc, PCx, exti_gpioc, 'C', [
    PC0: (pc0, 0),
    PC1: (pc1, 1),
    PC2: (pc2, 2),
    PC3: (pc3, 3),
    PC4: (pc4, 4),
    PC5: (pc5, 5),
    PC6: (pc6, 6),
    PC7: (pc7, 7),
    PC8: (pc8, 8),
    PC9: (pc9, 9),
    PC10: (pc10, 10),
    PC11: (pc11, 11),
    PC12: (pc12, 12),
    PC13: (pc13, 13),
    PC14: (pc14, 14),
    PC15: (pc15, 15),
]);