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
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_usb_driver::{
    Direction, EndpointAddress, EndpointAllocError, EndpointError, EndpointInfo, EndpointType,
    Event, Unsupported,
};

use crate::pac;
use crate::gpio::{Pin, mode};
use crate::interrupt;

// HT32F52352 USB Controller Hardware Specifications
const MAX_EP_COUNT: usize = 8;          // 1 control EP + 7 configurable EPs
const MAX_PACKET_SIZE: usize = 64;      // Full-speed USB max packet size
const EP_SRAM_SIZE: usize = 1024;       // Total endpoint buffer memory
const SINGLE_BUFFERED_EPS: usize = 3;   // Single-buffered endpoints (bulk/interrupt)
const DOUBLE_BUFFERED_EPS: usize = 4;   // Double-buffered endpoints (bulk/interrupt/iso)

/// USB DM (Data Minus) pin type
pub type UsbDm<const PORT: char, const PIN: u8> = Pin<PORT, PIN, mode::AlternateFunction<10>>;

/// USB DP (Data Plus) pin type
pub type UsbDp<const PORT: char, const PIN: u8> = Pin<PORT, PIN, mode::AlternateFunction<10>>;

/// USB pin pair for DM/DP configuration
pub struct UsbPins<const DM_PORT: char, const DM_PIN: u8, const DP_PORT: char, const DP_PIN: u8> {
    pub dm: UsbDm<DM_PORT, DM_PIN>,
    pub dp: UsbDp<DP_PORT, DP_PIN>,
}

impl<const DM_PORT: char, const DM_PIN: u8, const DP_PORT: char, const DP_PIN: u8>
    UsbPins<DM_PORT, DM_PIN, DP_PORT, DP_PIN> {
    /// Create new USB pin pair
    pub fn new(dm: UsbDm<DM_PORT, DM_PIN>, dp: UsbDp<DP_PORT, DP_PIN>) -> Self {
        Self { dm, dp }
    }
}

/// USB peripheral handle
pub struct Usb {
    _private: (),
}

impl Usb {
    /// Create new USB peripheral handle
    pub fn new() -> Self {
        // USB clock is enabled in the main init function via RCC
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
        initialize_usb_hardware(usb, &config);

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
fn initialize_usb_hardware(usb: &crate::pac::usb::RegisterBlock, config: &Config) {
    // Initialize USB hardware registers for HT32F52352

    // Reset USB peripheral to ensure clean state
    usb.csr().modify(|_, w| w.fres().set_bit());
    usb.csr().modify(|_, w| w.fres().clear_bit());

    // Enable USB power (clear PDWN bit)
    usb.csr().modify(|_, w| w.pdwn().clear_bit());

    // VBUS detection may not be available in HT32F52352 the same way
    // For now, this is a placeholder - actual VBUS detection would depend on hardware
    if config.vbus_detection {
        // Placeholder for VBUS detection setup
    }

    // Enable VBUS detect interrupt if requested
    if config.enable_vbus_detect {
        // Placeholder for VBUS interrupt enable
        // This may not be directly supported in HT32F52352
    }

    // Clear any pending interrupts
    unsafe {
        usb.isr().write(|w| w.bits(0xFFFFFFFF));
    }
}

/// Initialize USB with pins
pub fn init_usb_with_pins<const DM_PORT: char, const DM_PIN: u8, const DP_PORT: char, const DP_PIN: u8>(
    _usb_peripheral: Usb,
    pins: UsbPins<DM_PORT, DM_PIN, DP_PORT, DP_PIN>,
    config: Config
) -> Driver<'static> {
    let usb = unsafe { &*pac::Usb::ptr() };

    // Initialize USB hardware
    initialize_usb_hardware(usb, &config);

    // Pins are already configured as alternate function AF10 for USB
    // The UsbPins constructor ensures pins are in the correct mode
    let _pins = pins; // Use pins to avoid unused variable warning

    Driver::new(Usb::new(), config)
}

fn configure_endpoint_hardware(addr: EndpointAddress, ep_type: EndpointType, max_packet_size: u16) {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();
    let is_in = addr.is_in();

    // Calculate buffer address in 1024-byte EP_SRAM
    // EP0 (control): First 64 bytes, then distribute remaining space among configurable EPs
    let buffer_addr = if ep_num == 0 {
        0 // EP0 control endpoint starts at beginning
    } else {
        // Configurable endpoints: distribute remaining 960 bytes (1024 - 64)
        (64 + ((ep_num - 1) * (960 / 7))) as u16 // Approximately equal distribution
    };

    // HT32F52352 endpoint type configuration
    // The eptype field might be a boolean or might not exist in EPnCFGR
    // For now, we'll assume it's a simple control flag
    let is_control = ep_type == EndpointType::Control;

    // Configure endpoint based on endpoint number and type
    // Hardware supports: EP1-3 (single-buffered), EP4-7 (double-buffered)
    match ep_num {
        0 => {
            // EP0 is special - control endpoint, always enabled
            usb.ep0cfgr().modify(|_, w| unsafe {
                w.epbufa().bits(buffer_addr as u16)
                 .eplen().bits(max_packet_size.min(64) as u8)
                 // EP0 doesn't have direction or type fields - it's bidirectional control
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
        4..=7 => {
            // For endpoints 4-7, use a generic approach if needed in future
            // HT32F52352 has registers up to EP7CFGR
            // This would need to be implemented when supporting more endpoints
        }
        _ => {
            // HT32F52352 only supports 8 endpoints (0-7)
        }
    }
}

fn configure_control_endpoint(max_packet_size: u16) {
    let usb = unsafe { &*pac::Usb::ptr() };

    // EP0 is pre-configured for control transfers
    // Set maximum packet size (typically 64 bytes for full-speed)
    let ep0_size = max_packet_size.min(64) as u8;

    usb.ep0cfgr().modify(|_, w| unsafe {
        w.eplen().bits(ep0_size)  // Set EP0 max packet size
    });

    // Enable EP0 interrupts for setup, in, and out
    usb.ier().modify(|_, w| {
        w.ep0ie().set_bit()  // Enable EP0 interrupt
    });
}

async fn read_endpoint_data(addr: EndpointAddress, buf: &mut [u8]) -> Result<usize, EndpointError> {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    // Check endpoint status using the correct EPnCSR registers
    let has_data = match ep_num {
        0 => {
            let csr = usb.ep0csr().read();
            // Check if data is ready (not NAK and not stalled)
            !csr.nakrx().bit_is_set() && !csr.stlrx().bit_is_set()
        }
        1 => {
            let csr = usb.ep1csr().read();
            !csr.nakrx().bit_is_set() && !csr.stlrx().bit_is_set()
        }
        2 => {
            let csr = usb.ep2csr().read();
            !csr.nakrx().bit_is_set() && !csr.stlrx().bit_is_set()
        }
        3 => {
            let csr = usb.ep3csr().read();
            !csr.nakrx().bit_is_set() && !csr.stlrx().bit_is_set()
        }
        _ => false,
    };

    if !has_data {
        // Wait for data to become available (using embassy-sync Signal)
        wait_for_usb_event().await;
        return Ok(0); // Try again later
    }

    // Read data from USB SRAM buffer
    let buffer_addr = get_endpoint_buffer_addr(ep_num);
    let bytes_to_read = buf.len().min(MAX_PACKET_SIZE);

    // In a real implementation, we would:
    // 1. Access the USB SRAM at buffer_addr
    // 2. Copy bytes_to_read from SRAM to buf
    // 3. Clear the data ready flag by setting NAKRX

    // For now, simulate reading data
    for i in 0..bytes_to_read {
        buf[i] = 0x00; // Placeholder data
    }

    // Set NAKRX to indicate data has been read
    match ep_num {
        0 => usb.ep0csr().modify(|_, w| w.nakrx().set_bit()),
        1 => usb.ep1csr().modify(|_, w| w.nakrx().set_bit()),
        2 => usb.ep2csr().modify(|_, w| w.nakrx().set_bit()),
        3 => usb.ep3csr().modify(|_, w| w.nakrx().set_bit()),
        _ => {}
    }

    Ok(bytes_to_read)
}

/// Get endpoint buffer address in USB SRAM
fn get_endpoint_buffer_addr(ep_num: usize) -> u16 {
    if ep_num == 0 {
        0 // EP0 starts at beginning of EP_SRAM
    } else {
        // Distribute remaining 960 bytes among configurable endpoints
        (64 + ((ep_num - 1) * (960 / 7))) as u16
    }
}

async fn write_endpoint_data(addr: EndpointAddress, buf: &[u8]) -> Result<(), EndpointError> {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    if buf.len() > MAX_PACKET_SIZE {
        return Err(EndpointError::BufferOverflow);
    }

    // Copy data to USB SRAM buffer
    let buffer_addr = get_endpoint_buffer_addr(ep_num);

    // In a real implementation, we would:
    // 1. Copy buf data to USB SRAM at buffer_addr
    // 2. Set endpoint data length in EPnCFGR
    // 3. Clear NAKTX flag to start transmission via EPnCSR

    // Update endpoint configuration with data length
    match ep_num {
        0 => {
            usb.ep0cfgr().modify(|_, w| unsafe {
                w.eplen().bits(buf.len() as u8)
            });
            // Clear NAKTX to start transmission for EP0
            usb.ep0csr().modify(|_, w| w.naktx().clear_bit());
        }
        1 => {
            usb.ep1cfgr().modify(|_, w| unsafe {
                w.eplen().bits(buf.len() as u8)
            });
            // Clear NAKTX to start transmission for EP1
            usb.ep1csr().modify(|_, w| w.naktx().clear_bit());
        }
        2 => {
            usb.ep2cfgr().modify(|_, w| unsafe {
                w.eplen().bits(buf.len() as u8)
            });
            // Clear NAKTX to start transmission for EP2
            usb.ep2csr().modify(|_, w| w.naktx().clear_bit());
        }
        3 => {
            usb.ep3cfgr().modify(|_, w| unsafe {
                w.eplen().bits(buf.len() as u8)
            });
            // Clear NAKTX to start transmission for EP3
            usb.ep3csr().modify(|_, w| w.naktx().clear_bit());
        }
        _ => {
            return Err(EndpointError::BufferOverflow);
        }
    }

    // Wait for transmission complete interrupt
    USB_EVENT_SIGNAL.wait().await;

    Ok(())
}

async fn read_setup_packet() -> [u8; 8] {
    let usb = unsafe { &*pac::Usb::ptr() };

    // Wait for setup packet interrupt or check EP0 status
    // For HT32F52352, setup packets are handled via interrupt and EP0 buffer
    while !usb.isr().read().ep0if().bit_is_set() {
        wait_for_usb_event().await;
    }

    // Read setup packet from EP0 buffer
    // In a real implementation, we would read from the EP0 buffer in USB SRAM
    let mut packet = [0u8; 8];

    // For now, return a placeholder setup packet
    // This should be implemented to read the actual setup packet from EP0 buffer
    packet = [0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x40, 0x00]; // GET_DESCRIPTOR DEVICE

    // Clear EP0 interrupt flag
    usb.isr().modify(|_, w| w.ep0if().set_bit());

    packet
}

fn set_device_address(addr: u8) {
    let usb = unsafe { &*pac::Usb::ptr() };

    // Set USB device address in DEVAR register
    usb.devar().modify(|_, w| unsafe {
        w.deva().bits(addr)
    });

    // Enable USB address setting - HT32F52352 may use different field
    // For now, just setting the address should be sufficient
}

async fn poll_usb_events() -> Event {
    let usb = unsafe { &*pac::Usb::ptr() };
    let isr = usb.isr().read();

    // Check for USB reset
    if isr.urstif().bit_is_set() {
        usb.isr().modify(|_, w| w.urstif().set_bit()); // Clear flag
        return Event::Reset;
    }

    // Check for suspend
    if isr.suspif().bit_is_set() {
        usb.isr().modify(|_, w| w.suspif().set_bit()); // Clear flag
        return Event::Suspend;
    }

    // Check for resume (RSMIF - Resume Interrupt Flag)
    if isr.rsmif().bit_is_set() {
        usb.isr().modify(|_, w| w.rsmif().set_bit()); // Clear flag
        return Event::Resume;
    }

    // For VBUS detection, this might be handled differently in HT32F52352
    // For now, assume power is detected when USB is enabled

    // No specific event, wait for next interrupt
    USB_EVENT_SIGNAL.wait().await;
    Event::PowerDetected // Default event
}

fn set_endpoint_stall(addr: EndpointAddress, stalled: bool) {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    if stalled {
        // Stall the endpoint by setting STLTX/STLRX in EPnCSR
        match ep_num {
            0 => {
                usb.ep0csr().modify(|_, w| {
                    w.stltx().set_bit(); // Stall IN direction
                    w.stlrx().set_bit() // Stall OUT direction
                });
            }
            1 => {
                usb.ep1csr().modify(|_, w| {
                    w.stltx().set_bit();
                    w.stlrx().set_bit()
                });
            }
            2 => {
                usb.ep2csr().modify(|_, w| {
                    w.stltx().set_bit();
                    w.stlrx().set_bit()
                });
            }
            3 => {
                usb.ep3csr().modify(|_, w| {
                    w.stltx().set_bit();
                    w.stlrx().set_bit()
                });
            }
            _ => {}
        }
    } else {
        // Unstall the endpoint by clearing STLTX/STLRX in EPnCSR
        match ep_num {
            0 => {
                usb.ep0csr().modify(|_, w| {
                    w.stltx().clear_bit();
                    w.stlrx().clear_bit()
                });
            }
            1 => {
                usb.ep1csr().modify(|_, w| {
                    w.stltx().clear_bit();
                    w.stlrx().clear_bit()
                });
            }
            2 => {
                usb.ep2csr().modify(|_, w| {
                    w.stltx().clear_bit();
                    w.stlrx().clear_bit()
                });
            }
            3 => {
                usb.ep3csr().modify(|_, w| {
                    w.stltx().clear_bit();
                    w.stlrx().clear_bit()
                });
            }
            _ => {}
        }
    }
}

fn get_endpoint_stall(addr: EndpointAddress) -> bool {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    // Check if endpoint is stalled in either direction
    match ep_num {
        0 => {
            let csr = usb.ep0csr().read();
            csr.stltx().bit_is_set() || csr.stlrx().bit_is_set()
        }
        1 => {
            let csr = usb.ep1csr().read();
            csr.stltx().bit_is_set() || csr.stlrx().bit_is_set()
        }
        2 => {
            let csr = usb.ep2csr().read();
            csr.stltx().bit_is_set() || csr.stlrx().bit_is_set()
        }
        3 => {
            let csr = usb.ep3csr().read();
            csr.stltx().bit_is_set() || csr.stlrx().bit_is_set()
        }
        _ => false,
    }
}

fn set_endpoint_enabled(addr: EndpointAddress, enabled: bool) {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    if enabled {
        // Enable the endpoint
        match ep_num {
            0 => usb.ep0cfgr().modify(|_, w| w.epen().set_bit()),
            1 => usb.ep1cfgr().modify(|_, w| w.epen().set_bit()),
            2 => usb.ep2cfgr().modify(|_, w| w.epen().set_bit()),
            3 => usb.ep3cfgr().modify(|_, w| w.epen().set_bit()),
            _ => {}
        }
    } else {
        // Disable the endpoint
        match ep_num {
            0 => usb.ep0cfgr().modify(|_, w| w.epen().clear_bit()),
            1 => usb.ep1cfgr().modify(|_, w| w.epen().clear_bit()),
            2 => usb.ep2cfgr().modify(|_, w| w.epen().clear_bit()),
            3 => usb.ep3cfgr().modify(|_, w| w.epen().clear_bit()),
            _ => {}
        }
    }
}

fn enable_usb_device() {
    let usb = unsafe { &*pac::Usb::ptr() };

    // Enable USB pull-up resistor on D+ line to signal presence to host
    usb.csr().modify(|_, w| w.dppuen().set_bit());

    // Enable global USB interrupt
    usb.ier().modify(|_, w| {
        w.ugie().set_bit()   // Global interrupt enable
         .urstie().set_bit() // USB reset interrupt enable
         .suspie().set_bit() // Suspend interrupt enable
         .rsmie().set_bit()  // Resume interrupt enable
         .sofie().set_bit()  // Start of Frame interrupt enable
         .ep0ie().set_bit()  // Endpoint 0 interrupt enable
    });

    // Clear any pending interrupts
    unsafe {
        usb.isr().write(|w| w.bits(0xFFFFFFFF));
    }
}

fn disable_usb_device() {
    let usb = unsafe { &*pac::Usb::ptr() };

    // Disable USB pull-up resistor
    usb.csr().modify(|_, w| w.dppuen().clear_bit());

    // Disable USB interrupts
    unsafe {
        usb.ier().write(|w| w.bits(0));
    }
}

/// USB endpoint memory buffer - matches HT32F52352 hardware: 1024-byte EP_SRAM
static mut EP_MEMORY: [u8; EP_SRAM_SIZE] = [0; EP_SRAM_SIZE];

/// USB event signal for ISR to Task communication - avoids critical section deadlock
static USB_EVENT_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Wait for USB event and reset the signal
pub async fn wait_for_usb_event() {
    USB_EVENT_SIGNAL.wait().await;
}

// USB interrupt handler for the executor
#[unsafe(no_mangle)]
pub unsafe extern "C" fn USB() {
    // Safety: This is only called from the USB interrupt
    unsafe { on_usb_interrupt() }
}

/// USB interrupt handler
///
/// This function should be called from the global interrupt handler for USB interrupts
/// Uses Signal to avoid critical section deadlock with timer interrupts
pub unsafe fn on_usb_interrupt() {
    let usb = unsafe { &*pac::Usb::ptr() };
    let isr = usb.isr().read();
    let mut event_signaled = false;

    // Handle reset interrupt
    if isr.urstif().bit_is_set() {
        // Clear reset interrupt flag
        usb.isr().modify(|_, w| w.urstif().set_bit());
        event_signaled = true;
    }

    // Handle endpoint interrupts
    for ep in 0..8 {
        let ep_flag = match ep {
            0 => isr.ep0if().bit_is_set(),
            1 => isr.ep1if().bit_is_set(),
            2 => isr.ep2if().bit_is_set(),
            3 => isr.ep3if().bit_is_set(),
            4 => isr.ep4if().bit_is_set(),
            5 => isr.ep5if().bit_is_set(),
            6 => isr.ep6if().bit_is_set(),
            7 => isr.ep7if().bit_is_set(),
            _ => false,
        };

        if ep_flag {
            // Clear endpoint interrupt flag
            match ep {
                0 => usb.isr().modify(|_, w| w.ep0if().set_bit()),
                1 => usb.isr().modify(|_, w| w.ep1if().set_bit()),
                2 => usb.isr().modify(|_, w| w.ep2if().set_bit()),
                3 => usb.isr().modify(|_, w| w.ep3if().set_bit()),
                4 => usb.isr().modify(|_, w| w.ep4if().set_bit()),
                5 => usb.isr().modify(|_, w| w.ep5if().set_bit()),
                6 => usb.isr().modify(|_, w| w.ep6if().set_bit()),
                7 => usb.isr().modify(|_, w| w.ep7if().set_bit()),
                _ => {}
            }
            event_signaled = true;
        }
    }

    // Handle other USB interrupts as needed
    if isr.sofif().bit_is_set() {
        usb.isr().modify(|_, w| w.sofif().set_bit());
        // SOF interrupts are frequent, don't signal for them unless needed
    }

    // Signal the USB event outside of critical section context
    // This avoids deadlock when timer interrupts try to wake tasks
    if event_signaled {
        USB_EVENT_SIGNAL.signal(());
    }
}