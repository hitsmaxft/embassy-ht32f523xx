#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Hertz(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MicroSeconds(pub u32);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct MilliSeconds(pub u32);

pub trait U32Ext {
    fn hz(self) -> Hertz;
    fn khz(self) -> Hertz;
    fn mhz(self) -> Hertz;
    fn us(self) -> MicroSeconds;
    fn ms(self) -> MilliSeconds;
}

impl U32Ext for u32 {
    fn hz(self) -> Hertz {
        Hertz(self)
    }

    fn khz(self) -> Hertz {
        Hertz(self * 1_000)
    }

    fn mhz(self) -> Hertz {
        Hertz(self * 1_000_000)
    }

    fn us(self) -> MicroSeconds {
        MicroSeconds(self)
    }

    fn ms(self) -> MilliSeconds {
        MilliSeconds(self)
    }
}

impl From<Hertz> for u32 {
    fn from(hertz: Hertz) -> Self {
        hertz.0
    }
}

impl From<MicroSeconds> for u32 {
    fn from(us: MicroSeconds) -> Self {
        us.0
    }
}

impl From<MilliSeconds> for u32 {
    fn from(ms: MilliSeconds) -> Self {
        ms.0
    }
}