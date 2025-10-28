//! USB Device driver for HT32F523xx
//!
//! This module provides USB device functionality using the embassy-usb framework.
//!
//! ## HT32F52352 USB Controller Specifications:
//! - USB 2.0 full-speed (12 Mbps) compliant
//! - 1 control endpoint (EP0) for control transfer
//! - 3 single-buffered endpoints for bulk and interrupt transfer
//! - 4 double-buffered endpoints for bulk, interrupt and isochronous transfer
//! - 1,024 bytes EP_SRAM for endpoint data buffers
//! - Total: 8 endpoints (1 control + 7 configurable)

use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::waitqueue::AtomicWaker;
use embassy_usb_driver::{
    Direction, EndpointAddress, EndpointAllocError, EndpointError, EndpointInfo, EndpointType,
    Event, Unsupported,
};

use crate::pac;

// HT32F52352 USB Controller Hardware Specifications
const MAX_EP_COUNT: usize = 8;          // 1 control EP + 7 configurable EPs
const MAX_PACKET_SIZE: usize = 64;      // Full-speed USB max packet size
const EP_SRAM_SIZE: usize = 1024;       // Total endpoint buffer memory
const SINGLE_BUFFERED_EPS: usize = 3;   // Single-buffered endpoints (bulk/interrupt)
const DOUBLE_BUFFERED_EPS: usize = 4;   // Double-buffered endpoints (bulk/interrupt/iso)

/// USB peripheral handle
pub struct Usb {
    _private: (),
}

impl Usb {
    pub(crate) fn new() -> Self {
        // Enable USB clock
        // let ckcu = unsafe { &*crate::pac::Ckcu::ptr() };
        // ckcu.apbccr1().modify(|_, w| w.usben().set_bit());
        // TODO: Implement proper USB clock enabling based on HT32F523xx PAC

        Self { _private: () }
    }
}

/// USB driver implementation
pub struct Driver<'d> {
    phantom: PhantomData<&'d ()>,
    alloc_in: AtomicBool,
    alloc_out: AtomicBool,
}

impl<'d> Driver<'d> {
    /// Create a new USB driver instance
    pub fn new(_usb: Usb, config: Config) -> Self {
        let usb = unsafe { &*pac::Usb::ptr() };

        // Initialize USB hardware
        initialize_usb_hardware(usb, config);

        Self {
            phantom: PhantomData,
            alloc_in: AtomicBool::new(false),
            alloc_out: AtomicBool::new(false),
        }
    }
}

/// USB bus implementation for HT32F52352 USB controller
/// Hardware: 1 control EP + 7 configurable EPs, 1024-byte EP_SRAM
pub struct Bus<'d> {
    phantom: PhantomData<&'d ()>,
    // Arrays for all 8 endpoints (EP0 + 7 configurable)
    ep_types: [Option<EndpointType>; MAX_EP_COUNT], // 8 endpoints total
    ep_in_wakers: [AtomicWaker; MAX_EP_COUNT], // IN endpoint wakers
    ep_out_wakers: [AtomicWaker; MAX_EP_COUNT], // OUT endpoint wakers
    bus_waker: AtomicWaker,
}

impl<'d> Bus<'d> {
    fn new() -> Self {
        const NEW_AW: AtomicWaker = AtomicWaker::new();
        Self {
            phantom: PhantomData,
            ep_types: [None; MAX_EP_COUNT], // 8 endpoints
            ep_in_wakers: [NEW_AW; MAX_EP_COUNT], // Full waker arrays
            ep_out_wakers: [NEW_AW; MAX_EP_COUNT], // Full waker arrays
            bus_waker: AtomicWaker::new(),
        }
    }
}

/// USB control pipe implementation
pub struct ControlPipe<'d> {
    _phantom: PhantomData<&'d ()>,
}

/// USB endpoint implementation
pub struct Endpoint<'d, D> {
    _phantom: PhantomData<&'d ()>,
    info: EndpointInfo,
    _direction: PhantomData<D>,
}

/// Endpoint direction markers
pub struct In;
pub struct Out;

impl<'d> embassy_usb_driver::Driver<'d> for Driver<'d> {
    type EndpointOut = Endpoint<'d, Out>;
    type EndpointIn = Endpoint<'d, In>;
    type ControlPipe = ControlPipe<'d>;
    type Bus = Bus<'d>;

    fn alloc_endpoint_in(
        &mut self,
        ep_type: EndpointType,
        ep_addr: Option<EndpointAddress>,
        max_packet_size: u16,
        interval: u8,
    ) -> Result<Self::EndpointIn, EndpointAllocError> {
        if self.alloc_in.load(Ordering::Relaxed) {
            return Err(EndpointAllocError);
        }
        self.alloc_in.store(true, Ordering::Relaxed);

        let addr = ep_addr.unwrap_or(EndpointAddress::from_parts(1, Direction::In));

        // Configure hardware endpoint
        configure_endpoint_hardware(addr, ep_type, max_packet_size);

        Ok(Endpoint {
            _phantom: PhantomData,
            info: EndpointInfo {
                addr,
                ep_type,
                max_packet_size: max_packet_size.min(MAX_PACKET_SIZE as u16),
                interval_ms: interval,
            },
            _direction: PhantomData,
        })
    }

    fn alloc_endpoint_out(
        &mut self,
        ep_type: EndpointType,
        ep_addr: Option<EndpointAddress>,
        max_packet_size: u16,
        interval: u8,
    ) -> Result<Self::EndpointOut, EndpointAllocError> {
        if self.alloc_out.load(Ordering::Relaxed) {
            return Err(EndpointAllocError);
        }
        self.alloc_out.store(true, Ordering::Relaxed);

        let addr = ep_addr.unwrap_or(EndpointAddress::from_parts(1, Direction::Out));

        // Configure hardware endpoint
        configure_endpoint_hardware(addr, ep_type, max_packet_size);

        Ok(Endpoint {
            _phantom: PhantomData,
            info: EndpointInfo {
                addr,
                ep_type,
                max_packet_size: max_packet_size.min(MAX_PACKET_SIZE as u16),
                interval_ms: interval,
            },
            _direction: PhantomData,
        })
    }

    fn start(self, control_max_packet_size: u16) -> (Self::Bus, Self::ControlPipe) {
        let bus = Bus::new();
        let control_pipe = ControlPipe {
            _phantom: PhantomData,
        };

        // Configure EP0 for control transfers
        configure_control_endpoint(control_max_packet_size);

        (bus, control_pipe)
    }
}

// Implement endpoint traits
impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, In> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
        // Wait for endpoint to be enabled by host
        // This is a simplified implementation
    }
}

impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, Out> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
        // Wait for endpoint to be enabled by host
    }
}

impl<'d> embassy_usb_driver::EndpointOut for Endpoint<'d, Out> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, EndpointError> {
        if buf.len() > MAX_PACKET_SIZE {
            return Err(EndpointError::BufferOverflow);
        }

        // Read from USB hardware
        read_endpoint_data(self.info.addr, buf).await
    }
}

impl<'d> embassy_usb_driver::EndpointIn for Endpoint<'d, In> {
    async fn write(&mut self, buf: &[u8]) -> Result<(), EndpointError> {
        if buf.len() > MAX_PACKET_SIZE {
            return Err(EndpointError::BufferOverflow);
        }

        // Write to USB hardware
        write_endpoint_data(self.info.addr, buf).await
    }
}

impl<'d> embassy_usb_driver::ControlPipe for ControlPipe<'d> {
    fn max_packet_size(&self) -> usize {
        64
    }

    async fn setup(&mut self) -> [u8; 8] {
        // Read setup packet from hardware
        read_setup_packet().await
    }

    async fn data_out(&mut self, buf: &mut [u8], _first: bool, _last: bool) -> Result<usize, EndpointError> {
        // Read control data from hardware
        Ok(buf.len().min(64))
    }

    async fn data_in(&mut self, _data: &[u8], _first: bool, _last: bool) -> Result<(), EndpointError> {
        // Write control data to hardware
        Ok(())
    }

    async fn accept(&mut self) {
        // Send ACK to host
    }

    async fn reject(&mut self) {
        // Send STALL to host
    }

    async fn accept_set_address(&mut self, addr: u8) {
        // Set device address
        set_device_address(addr);
    }
}

impl<'d> embassy_usb_driver::Bus for Bus<'d> {
    async fn poll(&mut self) -> Event {
        // Poll USB hardware for events
        poll_usb_events().await
    }

    fn endpoint_set_stalled(&mut self, ep_addr: EndpointAddress, stalled: bool) {
        // Set/clear endpoint stall
        set_endpoint_stall(ep_addr, stalled);
    }

    fn endpoint_is_stalled(&mut self, ep_addr: EndpointAddress) -> bool {
        // Check if endpoint is stalled
        get_endpoint_stall(ep_addr)
    }

    fn endpoint_set_enabled(&mut self, ep_addr: EndpointAddress, enabled: bool) {
        // Enable/disable endpoint
        set_endpoint_enabled(ep_addr, enabled);
    }

    async fn enable(&mut self) {
        // Enable USB device
        enable_usb_device();
    }

    async fn disable(&mut self) {
        // Disable USB device
        disable_usb_device();
    }

    async fn remote_wakeup(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported)
    }
}

/// USB configuration
pub struct Config {
    /// Enable VBUS detection
    pub vbus_detection: bool,
    /// Enable VBUS detect interrupt
    pub enable_vbus_detect: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            vbus_detection: false,
            enable_vbus_detect: false,
        }
    }
}

// Hardware-specific implementation functions
fn initialize_usb_hardware(usb: &crate::pac::usb::RegisterBlock, _config: Config) {
    // Initialize USB hardware registers
    // This is a simplified implementation - actual HT32 USB initialization
    // would involve proper register configuration based on the datasheet

    // Reset USB
    usb.csr().modify(|_, w| w.fres().set_bit());
    usb.csr().modify(|_, w| w.fres().clear_bit());

    // Enable USB
    usb.csr().modify(|_, w| w.pdwn().clear_bit());
}

fn configure_endpoint_hardware(addr: EndpointAddress, _ep_type: EndpointType, max_packet_size: u16) {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    // Calculate buffer address in 1024-byte EP_SRAM
    // EP0 (control): First 64 bytes, then distribute remaining space among configurable EPs
    let buffer_addr = if ep_num == 0 {
        0 // EP0 control endpoint starts at beginning
    } else {
        // Configurable endpoints: distribute remaining 960 bytes (1024 - 64)
        64 + ((ep_num - 1) * (960 / 7)) // Approximately equal distribution
    };

    // Configure endpoint based on endpoint number and type
    // Hardware supports: EP1-3 (single-buffered), EP4-7 (double-buffered)
    match ep_num {
        0 => {
            usb.ep0cfgr().modify(|_, w| unsafe {
                w.epbufa().bits(buffer_addr as u16)
                 .eplen().bits(max_packet_size.min(64) as u8)
                 .epadr().bits(0) // EP0 address is always 0
                 .epen().set_bit()
            });
        }
        1 => {
            usb.ep1cfgr().modify(|_, w| unsafe {
                w.epbufa().bits(buffer_addr as u16)
                 .eplen().bits(max_packet_size.min(64) as u8)
                 .epadr().bits(ep_num as u8)
                 .epen().set_bit()
            });
        }
        2 => {
            usb.ep2cfgr().modify(|_, w| unsafe {
                w.epbufa().bits(buffer_addr as u16)
                 .eplen().bits(max_packet_size.min(64) as u8)
                 .epadr().bits(ep_num as u8)
                 .epen().set_bit()
            });
        }
        3 => {
            usb.ep3cfgr().modify(|_, w| unsafe {
                w.epbufa().bits(buffer_addr as u16)
                 .eplen().bits(max_packet_size.min(64) as u8)
                 .epadr().bits(ep_num as u8)
                 .epen().set_bit()
            });
        }
        _ => {
            // Additional endpoints can be configured here if needed
        }
    }
}

fn configure_control_endpoint(_max_packet_size: u16) {
    // Configure EP0 for control transfers
    let _usb = unsafe { &*pac::Usb::ptr() };
}

async fn read_endpoint_data(addr: EndpointAddress, buf: &mut [u8]) -> Result<usize, EndpointError> {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    // Wait for data to be available (simplified - in real implementation would use interrupts)
    let bytes_available = match ep_num {
        0 => {
            // Read from EP0 buffer
            let cfg = usb.ep0cfgr().read();
            cfg.eplen().bits() as usize
        }
        1 => {
            let cfg = usb.ep1cfgr().read();
            cfg.eplen().bits() as usize
        }
        2 => {
            let cfg = usb.ep2cfgr().read();
            cfg.eplen().bits() as usize
        }
        _ => 0,
    };

    let _bytes_to_read = bytes_available.min(buf.len());

    // In real hardware implementation, would copy from USB SRAM to buffer
    // For now, return 0 bytes as placeholder
    Ok(0)
}

async fn write_endpoint_data(addr: EndpointAddress, buf: &[u8]) -> Result<(), EndpointError> {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    if buf.len() > MAX_PACKET_SIZE {
        return Err(EndpointError::BufferOverflow);
    }

    // In real implementation, would:
    // 1. Copy data to USB SRAM buffer
    // 2. Set endpoint length
    // 3. Trigger transmission
    // 4. Wait for completion interrupt

    // For now, update the endpoint length to indicate data is ready
    match ep_num {
        0 => {
            usb.ep0cfgr().modify(|_, w| unsafe {
                w.eplen().bits(buf.len() as u8)
            });
        }
        1 => {
            usb.ep1cfgr().modify(|_, w| unsafe {
                w.eplen().bits(buf.len() as u8)
            });
        }
        2 => {
            usb.ep2cfgr().modify(|_, w| unsafe {
                w.eplen().bits(buf.len() as u8)
            });
        }
        _ => return Err(EndpointError::BufferOverflow), // Use available error variant
    }

    // Wait for transmission complete (simplified)
    crate::interrupt::get_waker(crate::pac::Interrupt::USB).wait().await;

    Ok(())
}

async fn read_setup_packet() -> [u8; 8] {
    // Read setup packet from control endpoint
    [0; 8]
}

fn set_device_address(addr: u8) {
    // Set USB device address
    let usb = unsafe { &*pac::Usb::ptr() };
    usb.devar().modify(|_, w| unsafe { w.deva().bits(addr) });
}

async fn poll_usb_events() -> Event {
    // Poll for USB events (reset, suspend, resume, etc.)
    Event::PowerDetected
}

fn set_endpoint_stall(_addr: EndpointAddress, _stalled: bool) {
    // Set/clear endpoint stall condition
}

fn get_endpoint_stall(_addr: EndpointAddress) -> bool {
    // Check if endpoint is stalled
    false
}

fn set_endpoint_enabled(_addr: EndpointAddress, _enabled: bool) {
    // Enable/disable endpoint
}

fn enable_usb_device() {
    // Enable USB device functionality
    let usb = unsafe { &*pac::Usb::ptr() };
    usb.csr().modify(|_, w| w.genrsm().set_bit());
}

fn disable_usb_device() {
    // Disable USB device functionality
    let usb = unsafe { &*pac::Usb::ptr() };
    usb.csr().modify(|_, w| w.genrsm().clear_bit());
}

/// USB endpoint memory buffer - matches HT32F52352 hardware: 1024-byte EP_SRAM
static mut EP_MEMORY: [u8; EP_SRAM_SIZE] = [0; EP_SRAM_SIZE];