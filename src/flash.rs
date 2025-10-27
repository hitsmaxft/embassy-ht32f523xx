//! Flash memory driver for HT32F523xx
//!
//! This module provides flash memory operations using the HT32F523xx Flash Memory Controller (FMC).

use core::ptr;
use embassy_time::{Duration, Timer};
use embedded_storage::nor_flash::{ErrorType, NorFlash, ReadNorFlash, NorFlashError, NorFlashErrorKind};

use crate::pac;

/// Flash memory controller
pub struct Flash {
    _private: (),
}

impl Flash {
    /// Create a new flash controller instance
    pub fn new() -> Self {
        Self { _private: () }
    }

    /// Get the flash capacity in bytes
    pub fn capacity(&self) -> usize {
        crate::chip::MEMORY.flash_kb as usize * 1024
    }

    /// Wait for flash operation to complete
    async fn wait_ready(&self) -> Result<(), FlashError> {
        let fmc = unsafe { &*pac::Fmc::ptr() };

        // Wait for operation to complete (bit 0 of OISR is busy flag)
        let mut timeout = 1000; // 1000ms timeout
        while fmc.oisr().read().bits() & 0x01 != 0 && timeout > 0 {
            Timer::after(Duration::from_millis(1)).await;
            timeout -= 1;
        }

        if timeout == 0 {
            return Err(FlashError::Timeout);
        }

        // Check for errors
        let status = fmc.oisr().read().bits();
        if status & 0x02 != 0 {
            return Err(FlashError::WriteError);
        }
        if status & 0x04 != 0 {
            return Err(FlashError::EraseError);
        }

        Ok(())
    }

    /// Unlock flash for writing/erasing
    fn unlock(&self) {
        let fmc = unsafe { &*pac::Fmc::ptr() };

        // Write unlock sequence to OCMR register
        fmc.ocmr().write(|w| unsafe { w.bits(0xA9B8C7D6) });
        fmc.ocmr().write(|w| unsafe { w.bits(0xD6C7B8A9) });
    }

    /// Lock flash to prevent accidental writes
    fn lock(&self) {
        let fmc = unsafe { &*pac::Fmc::ptr() };
        fmc.ocmr().write(|w| unsafe { w.bits(0x00000000) });
    }

    /// Erase a page of flash memory
    async fn erase_page(&self, address: u32) -> Result<(), FlashError> {
        let fmc = unsafe { &*pac::Fmc::ptr() };

        // Unlock flash
        self.unlock();

        // Set target address
        fmc.tadr().write(|w| unsafe { w.bits(address) });

        // Set erase operation mode (OPM = 0x2 for page erase)
        fmc.opcr().write(|w| unsafe { w.opm().bits(0x2) });

        // Wait for operation to complete
        self.wait_ready().await?;

        // Lock flash
        self.lock();

        Ok(())
    }

    /// Write data to flash memory
    async fn write_word(&self, address: u32, data: u32) -> Result<(), FlashError> {
        let fmc = unsafe { &*pac::Fmc::ptr() };

        // Unlock flash
        self.unlock();

        // Set target address
        fmc.tadr().write(|w| unsafe { w.bits(address) });

        // Set write data
        fmc.wrdr().write(|w| unsafe { w.bits(data) });

        // Set write operation mode (OPM = 0x4 for word write)
        fmc.opcr().write(|w| unsafe { w.opm().bits(0x4) });

        // Wait for operation to complete
        self.wait_ready().await?;

        // Lock flash
        self.lock();

        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlashError {
    Timeout,
    WriteError,
    EraseError,
    AddressOutOfRange,
    UnalignedAddress,
}

impl NorFlashError for FlashError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            FlashError::Timeout => NorFlashErrorKind::Other,
            FlashError::WriteError => NorFlashErrorKind::Other,
            FlashError::EraseError => NorFlashErrorKind::Other,
            FlashError::AddressOutOfRange => NorFlashErrorKind::OutOfBounds,
            FlashError::UnalignedAddress => NorFlashErrorKind::NotAligned,
        }
    }
}

impl ErrorType for Flash {
    type Error = FlashError;
}

impl ReadNorFlash for Flash {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        let flash_base = 0x0000_0000u32;
        let address = flash_base + offset;

        if address >= self.capacity() as u32 {
            return Err(FlashError::AddressOutOfRange);
        }

        // Read directly from flash memory
        unsafe {
            ptr::copy_nonoverlapping(
                address as *const u8,
                bytes.as_mut_ptr(),
                bytes.len(),
            );
        }

        Ok(())
    }

    fn capacity(&self) -> usize {
        Flash::capacity(self)
    }
}

impl NorFlash for Flash {
    const WRITE_SIZE: usize = 4; // HT32 flash writes in 32-bit words
    const ERASE_SIZE: usize = 1024; // HT32 typical page size is 1KB

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if from % Self::ERASE_SIZE as u32 != 0 || to % Self::ERASE_SIZE as u32 != 0 {
            return Err(FlashError::UnalignedAddress);
        }

        if to > self.capacity() as u32 {
            return Err(FlashError::AddressOutOfRange);
        }

        // Note: This would need to be called from an async context
        // For now, we'll return an error as sync erase is not supported
        // Use erase_async() instead
        Err(FlashError::WriteError)
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        if offset % Self::WRITE_SIZE as u32 != 0 {
            return Err(FlashError::UnalignedAddress);
        }

        if offset + bytes.len() as u32 > self.capacity() as u32 {
            return Err(FlashError::AddressOutOfRange);
        }

        if bytes.len() % Self::WRITE_SIZE != 0 {
            return Err(FlashError::UnalignedAddress);
        }

        // Note: This would need to be called from an async context
        // For now, we'll return an error as sync write is not supported
        Err(FlashError::WriteError)
    }
}

/// Async flash operations for Embassy integration
impl Flash {
    /// Erase a range of flash memory (async)
    pub async fn erase_async(&mut self, from: u32, to: u32) -> Result<(), FlashError> {
        if from % Self::ERASE_SIZE as u32 != 0 || to % Self::ERASE_SIZE as u32 != 0 {
            return Err(FlashError::UnalignedAddress);
        }

        if to > self.capacity() as u32 {
            return Err(FlashError::AddressOutOfRange);
        }

        // Erase all pages in the range
        let mut address = from;
        while address < to {
            self.erase_page(address).await?;
            address += Self::ERASE_SIZE as u32;
        }

        Ok(())
    }

    /// Write data to flash memory (async)
    pub async fn write_async(&mut self, offset: u32, bytes: &[u8]) -> Result<(), FlashError> {
        if offset % Self::WRITE_SIZE as u32 != 0 {
            return Err(FlashError::UnalignedAddress);
        }

        if offset + bytes.len() as u32 > self.capacity() as u32 {
            return Err(FlashError::AddressOutOfRange);
        }

        if bytes.len() % Self::WRITE_SIZE != 0 {
            return Err(FlashError::UnalignedAddress);
        }

        // Write data in 32-bit chunks
        let mut address = offset;
        let mut data_ptr = bytes.as_ptr();

        for _ in 0..(bytes.len() / Self::WRITE_SIZE) {
            let word = unsafe {
                ((*data_ptr) as u32) |
                ((*data_ptr.add(1)) as u32) << 8 |
                ((*data_ptr.add(2)) as u32) << 16 |
                ((*data_ptr.add(3)) as u32) << 24
            };

            self.write_word(address, word).await?;

            address += Self::WRITE_SIZE as u32;
            data_ptr = unsafe { data_ptr.add(Self::WRITE_SIZE) };
        }

        Ok(())
    }
}