use crate::pac::Ckcu;
use crate::time::Hertz;

pub trait RccExt {
    fn constrain(self) -> Rcc;
}

impl RccExt for Ckcu {
    fn constrain(self) -> Rcc {
        Rcc { ckcu: self }
    }
}

pub struct Rcc {
    pub ckcu: Ckcu,
}

impl Rcc {
    pub fn configure(self) -> Config {
        Config {
            hclk: None,
            ckcu: self.ckcu,
        }
    }
}

pub struct Config {
    hclk: Option<u32>,
    ckcu: Ckcu,
}

impl Config {
    pub fn hclk<F>(mut self, freq: F) -> Self
    where
        F: Into<Hertz>,
    {
        self.hclk = Some(freq.into().0);
        self
    }

    pub fn freeze(self) -> Clocks {
        let hclk = self.hclk.unwrap_or(8_000_000);

        Clocks {
            hclk: Hertz(hclk),
            pclk: Hertz(hclk),
        }
    }
}

#[derive(Clone, Copy)]
pub struct Clocks {
    hclk: Hertz,
    pclk: Hertz,
}

impl Clocks {
    pub fn hclk(&self) -> Hertz {
        self.hclk
    }

    pub fn pclk(&self) -> Hertz {
        self.pclk
    }
}
