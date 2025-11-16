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
use core::sync::atomic::{AtomicBool, AtomicU16, Ordering};

use embassy_sync::waitqueue::AtomicWaker;
use embassy_sync::signal::Signal;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_usb_driver::{
    Direction, EndpointAddress, EndpointAllocError, EndpointError, EndpointInfo, EndpointType,
    Event, Unsupported,
};

use crate::pac;
use crate::gpio::{Pin, mode};

// Use defmt logging when available, otherwise provide stub implementations
#[cfg(feature = "defmt")]
use defmt::{debug, error, info, warn};

#[cfg(not(feature = "defmt"))]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! debug {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! error {
    ($($arg:tt)*) => {};
}

// HT32F52352 USB Controller Hardware Specifications
const MAX_EP_COUNT: usize = 8;          // 1 control EP + 7 configurable EPs
const MAX_PACKET_SIZE: usize = 64;      // Full-speed USB max packet size
const EP_SRAM_SIZE: usize = 1024;       // Total endpoint buffer memory
const SINGLE_BUFFERED_EPS: usize = 3;   // Single-buffered endpoints (bulk/interrupt)
const DOUBLE_BUFFERED_EPS: usize = 4;   // Double-buffered endpoints (bulk/interrupt/iso)

/// USB DM (Data Minus) pin type
pub type UsbDm<const PORT: char, const PIN: u8> = Pin<PORT, PIN, mode::AlternateFunction<0>>;

/// USB DP (Data Plus) pin type
pub type UsbDp<const PORT: char, const PIN: u8> = Pin<PORT, PIN, mode::AlternateFunction<0>>;

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
    allocated_eps: AtomicU16, // Bit mask for allocated endpoints (bit 0 = EP0, bit 1 = EP1, etc.)
}

impl<'d> Driver<'d> {
    /// Create a new USB driver instance
    pub fn new(_usb: Usb, config: Config) -> Self {
        info!("🔌 USB_DRIVER_START: Initializing HT32F52352 USB driver");

        let usb = unsafe { &*pac::Usb::ptr() };

        // Read USB CSR to check hardware status
        let csr = usb.csr().read();
        info!("🔌 USB_DRIVER_CSR: Initial CSR state = {:#010x}", csr.bits());

        // Initialize USB hardware
        initialize_usb_hardware(usb, &config);

        info!("✅ USB_DRIVER_INIT: USB hardware initialization completed");

        Self {
            phantom: PhantomData,
            allocated_eps: AtomicU16::new(0),
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
    power_detected_sent: AtomicBool, // Track if PowerDetected event has been sent
    device_configured: Signal<CriticalSectionRawMutex, ()>, // Signal when device is configured
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
            power_detected_sent: AtomicBool::new(false),
            device_configured: Signal::new(),
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
        // Use suggested address or default based on endpoint type
        let addr = ep_addr.unwrap_or_else(|| {
            match ep_type {
                EndpointType::Interrupt => EndpointAddress::from_parts(1, Direction::In), // EP1 IN for CDC notifications
                EndpointType::Bulk => EndpointAddress::from_parts(3, Direction::In), // EP3 IN for CDC data
                EndpointType::Isochronous => EndpointAddress::from_parts(5, Direction::In), // EP5 IN for isochronous
                EndpointType::Control => EndpointAddress::from_parts(0, Direction::In), // EP0 (control)
            }
        });

        // Check if endpoint is already allocated
        let ep_mask = 1u16 << addr.index();
        let current_allocated = self.allocated_eps.load(Ordering::Relaxed);
        if current_allocated & ep_mask != 0 {
            return Err(EndpointAllocError);
        }

        // Mark endpoint as allocated
        self.allocated_eps.store(current_allocated | ep_mask, Ordering::Relaxed);

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
        // Use suggested address or default based on endpoint type
        let addr = ep_addr.unwrap_or_else(|| {
            match ep_type {
                EndpointType::Bulk => EndpointAddress::from_parts(2, Direction::Out), // EP2 OUT for CDC data
                EndpointType::Isochronous => EndpointAddress::from_parts(4, Direction::Out), // EP4 OUT for isochronous
                EndpointType::Interrupt => EndpointAddress::from_parts(6, Direction::Out), // EP6 OUT for interrupt
                EndpointType::Control => EndpointAddress::from_parts(0, Direction::Out), // EP0 (control)
            }
        });

        // Check if endpoint is already allocated
        let ep_mask = 1u16 << addr.index();
        let current_allocated = self.allocated_eps.load(Ordering::Relaxed);
        if current_allocated & ep_mask != 0 {
            return Err(EndpointAllocError);
        }

        // Mark endpoint as allocated
        self.allocated_eps.store(current_allocated | ep_mask, Ordering::Relaxed);

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
        info!("🚀 USB_DRIVER_START: Starting USB driver with control max packet size = {}", control_max_packet_size);

        let bus = Bus::new();
        let control_pipe = ControlPipe {
            _phantom: PhantomData,
        };

        // Configure EP0 for control transfers
        configure_control_endpoint(control_max_packet_size);

        info!("✅ USB_DRIVER_READY: USB driver started successfully with EP0 configured");

        (bus, control_pipe)
    }
}

// Implement endpoint traits
impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, In> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
        // Wait for device to be configured by host before endpoint is usable
        // This prevents race condition where write_packet is called before
        // SET_CONFIGURATION has been processed by the USB driver
        debug!("⏳ EP_WAIT_ENABLED: EP{} IN waiting for device configuration", self.info.addr.index());

        // Check if device is already configured
        if DEVICE_CONFIGURED.load(Ordering::Acquire) {
            debug!("⚡ EP_WAIT_ENABLED: EP{} IN device already configured", self.info.addr.index());
            return;
        }

        // Wait for configuration using the signal
        DEVICE_CONFIG_SIGNAL.wait().await;

        debug!("✅ EP_WAIT_ENABLED: EP{} IN device configured, endpoint ready", self.info.addr.index());
    }
}

impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, Out> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
        // Wait for device to be configured by host before endpoint is usable
        // This prevents race condition where read_packet is called before
        // SET_CONFIGURATION has been processed by the USB driver
        debug!("⏳ EP_WAIT_ENABLED: EP{} OUT waiting for device configuration", self.info.addr.index());

        // Check if device is already configured
        if DEVICE_CONFIGURED.load(Ordering::Acquire) {
            debug!("⚡ EP_WAIT_ENABLED: EP{} OUT device already configured", self.info.addr.index());
            return;
        }

        // Wait for configuration using the signal
        DEVICE_CONFIG_SIGNAL.wait().await;

        debug!("✅ EP_WAIT_ENABLED: EP{} OUT device configured, endpoint ready", self.info.addr.index());
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
        // Read control OUT data from hardware (EP0)
        info!("📥 CONTROL_DATA_OUT: Reading {} bytes from control endpoint", buf.len());

        let usb = unsafe { &*pac::Usb::ptr() };

        // Wait for data to be available (not NAK and not stalled)
        let mut timeout = 1000;
        while (usb.ep0csr().read().nakrx().bit_is_set() || usb.ep0csr().read().stlrx().bit_is_set()) && timeout > 0 {
            embassy_futures::yield_now().await;
            timeout -= 1;
        }

        if timeout == 0 {
            warn!("⚠️  CONTROL_DATA_OUT: Timeout waiting for data");
            return Ok(0);
        }

        // Read data from EP0 SRAM buffer
        let ep0tcr = usb.ep0tcr().read();
        let data_len = ep0tcr.rxcnt().bits() as usize;
        let actual_len = buf.len().min(data_len);

        if actual_len > 0 {
            // EP0 RX buffer is at offset 0x048 in USB SRAM
            let buffer_addr = EP0_RX_OFFSET as usize;

            // Use the new USB SRAM access functions
            read_usb_sram_bytes(buffer_addr, &mut buf[..actual_len]);

            // Set NAKRX to indicate data has been read
            usb.ep0csr().modify(|_, w| w.nakrx().set_bit());

            info!("📥 CONTROL_DATA_OUT: Successfully read {} bytes", actual_len);
        }

        Ok(actual_len)
    }

    async fn data_in(&mut self, data: &[u8], _first: bool, _last: bool) -> Result<(), EndpointError> {
        // Write control IN data to hardware (EP0)
        info!("📤 CONTROL_DATA_IN: Writing {} bytes to control endpoint", data.len());

        if data.is_empty() {
            return Ok(());
        }

        let usb = unsafe { &*pac::Usb::ptr() };

        // Wait for EP0 to be ready for transmission
        let mut timeout = 1000;
        while !usb.ep0csr().read().naktx().bit_is_set() && timeout > 0 {
            embassy_futures::yield_now().await;
            timeout -= 1;
        }

        if timeout == 0 {
            warn!("⚠️  CONTROL_DATA_IN: Timeout waiting for TX ready");
            return Err(EndpointError::BufferOverflow);
        }

        // Write data to EP0 TX buffer in USB SRAM
        let buffer_addr = EP0_TX_OFFSET as usize;
        let data_len = data.len().min(64);

        if buffer_addr + data_len <= EP_SRAM_SIZE {
            // Use the new USB SRAM access functions
            write_usb_sram_bytes(buffer_addr, &data[..data_len]);

            // Set TX count and clear NAKTX to start transmission
            usb.ep0tcr().modify(|_, w| unsafe {
                w.txcnt().bits(data_len as u8)
            });
            usb.ep0csr().modify(|_, w| w.naktx().clear_bit());

            info!("📤 CONTROL_DATA_IN: Successfully wrote {} bytes", data_len);
        } else {
            return Err(EndpointError::BufferOverflow);
        }

        Ok(())
    }

    async fn accept(&mut self) {
        // Send ACK to host - clear STALL bit for EP0
        info!("✅ CONTROL_ACCEPT: Sending ACK to host");
        let usb = unsafe { &*pac::Usb::ptr() };
        usb.ep0csr().modify(|_, w| w.stlrx().clear_bit());
    }

    async fn reject(&mut self) {
        // Send STALL to host - set STALL bit for EP0
        warn!("❌ CONTROL_REJECT: Sending STALL to host");
        let usb = unsafe { &*pac::Usb::ptr() };
        usb.ep0csr().modify(|_, w| w.stlrx().set_bit());
    }

    async fn accept_set_address(&mut self, addr: u8) {
        // Set device address
        set_device_address(addr);
    }
}

impl<'d> embassy_usb_driver::Bus for Bus<'d> {
    async fn poll(&mut self) -> Event {
        let event = poll_usb_events(self).await;
        event
    }

    fn endpoint_set_stalled(&mut self, ep_addr: EndpointAddress, stalled: bool) {
        debug!("🔧 USB_EP_STALL: Setting endpoint {} stalled = {}", ep_addr.index(), stalled);
        set_endpoint_stall(ep_addr, stalled);
    }

    fn endpoint_is_stalled(&mut self, ep_addr: EndpointAddress) -> bool {
        let stalled = get_endpoint_stall(ep_addr);
        debug!("🔧 USB_EP_STALL_CHECK: Endpoint {} stalled = {}", ep_addr.index(), stalled);
        stalled
    }

    fn endpoint_set_enabled(&mut self, ep_addr: EndpointAddress, enabled: bool) {
        debug!("🔧 USB_EP_ENABLE: Setting endpoint {} enabled = {}", ep_addr.index(), enabled);
        set_endpoint_enabled(ep_addr, enabled);
    }

    async fn enable(&mut self) {
        info!("🚀 USB_BUS_ENABLE: Enabling USB device");
        enable_usb_device();
        info!("✅ USB_BUS_ENABLED: USB device enabled successfully");
    }

    async fn disable(&mut self) {
        warn!("⚠️  USB_BUS_DISABLE: Disabling USB device");
        disable_usb_device();
        info!("✅ USB_BUS_DISABLED: USB device disabled");
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
    info!("🔌 USB_HW_INIT: Starting USB hardware initialization");

    // Reset USB peripheral to ensure clean state
    usb.csr().modify(|_, w| w.fres().set_bit());
    usb.csr().modify(|_, w| w.fres().clear_bit());

    // Step 1: USB Power Up configuration 
    // CSR配置: DPWKEN, DPPUEN, LPMODE, PDWN (ALL enabled initially)
    usb.csr().modify(|_, w| unsafe {
        w.dpwken().set_bit()   // DP唤醒使能 (DP wake enable)
         .dppuen().set_bit()   // DP上拉使能 (DP pull-up enable) - CRITICAL for enumeration!
         .lpmode().set_bit()   // 低功耗模式 (low power mode)
         .pdwn().set_bit()     // 掉电模式 (power down mode)
    });

    // Step 2: Clear all pending interrupts
    unsafe {
        usb.isr().write(|w| w.bits(0xFFFFFFFF));
    }

    // Step 3: Disable DP wake (normal operation)
    // This transitions USB from powered-up to active state
    // 修正后的 Step 3：禁用 DP 唤醒
    usb.csr().modify(|_, w| {
        // 确保清除 DPWKEN
        w.dpwken().clear_bit()
    });
    

    // Step 4: Enable USB interrupts at peripheral level
    // Following ChibiOS: USBIER_UGIE | USBIER_SOFIE | USBIER_URSTIE | USBIER_RSMIE | USBIER_SUSPIE | USBIER_EP0IE
    usb.ier().modify(|_, w| {
        w.ugie().set_bit()     // USB全局中断使能 (USB global interrupt enable) - CRITICAL!
         .sofie().set_bit()    // 帧起始中断使能 (Start of Frame interrupt)
         .urstie().set_bit()   // USB复位中断使能 (USB reset interrupt) - CRITICAL for enumeration
         .rsmie().set_bit()    // 恢复中断使能 (Resume interrupt)
         .suspie().set_bit()   // 挂起中断使能 (Suspend interrupt)
         .ep0ie().set_bit()    // 端点0中断使能 (Endpoint 0 interrupt) - CRITICAL for control transfers
    });

    // Note: DPPUEN (DP pull-up) is enabled in Step 1 and maintained in Step 3

    info!("🔌 USB_HW_INIT: USB hardware and interrupts initialized successfully");
}

/// Initialize USB with pins
pub fn init_usb_with_pins<const DM_PORT: char, const DM_PIN: u8, const DP_PORT: char, const DP_PIN: u8>(
    _usb_peripheral: Usb,
    pins: UsbPins<DM_PORT, DM_PIN, DP_PORT, DP_PIN>,
    config: Config
) -> Driver<'static> {
    let usb = unsafe { &*pac::Usb::ptr() };

    // Pins are already configured as alternate function AF0 for USB
    // The UsbPins constructor ensures pins are in the correct mode
    let _pins = pins; // Use pins to avoid unused variable warning

    Driver::new(Usb::new(), config)
}

fn configure_endpoint_hardware(addr: EndpointAddress, ep_type: EndpointType, max_packet_size: u16) {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();
    let is_in = addr.is_in();

    // Calculate buffer address in 1024-byte EP_SRAM
    // Get proper buffer address based on endpoint number and direction
    let buffer_addr = get_endpoint_buffer_addr(ep_num, is_in);

    let ep_type_str = match ep_type {
        EndpointType::Control => "Control",
        EndpointType::Isochronous => "Isochronous",
        EndpointType::Bulk => "Bulk",
        EndpointType::Interrupt => "Interrupt",
    };
    info!("🔧 EP_CONFIG: EP{} {} {} size={} addr={:#x}",
         ep_num, if is_in { "IN" } else { "OUT" }, ep_type_str, max_packet_size, buffer_addr);

    // Configure endpoint based on endpoint number and type
    // Hardware supports: EP1-3 (single-buffered), EP4-7 (double-buffered)
    match ep_num {
        0 => {
            // EP0 is special - control endpoint, uses SETUP buffer address (0x000)
            // EP0 SETUP buffer is always at offset 0x000, regardless of direction
            let ep0_setup_addr = get_ep0_setup_addr();
            usb.ep0cfgr().modify(|_, w| unsafe {
                w.epbufa().bits(ep0_setup_addr)
                 .eplen().bits(max_packet_size.min(64) as u8)
                 // EP0 doesn't have direction or type fields - it's bidirectional control
            });

            // Enable EP0-specific interrupts for setup, in, and out (CRITICAL for enumeration!)
            // Following ChibiOS pattern: USBEPnIER_SDRXIE|USBEPnIER_IDTXIE|USBEPnIER_ODRXIE
            usb.ep0ier().modify(|_, w| unsafe {
                w.sdrxie().set_bit()  // Setup Data Received Interrupt Enable - CRITICAL!
                 .idtxie().set_bit()  // IN Data Transfer Complete Interrupt Enable
                 .odrxie().set_bit()  // OUT Data Received Interrupt Enable
            });

            // Enable global EP0 interrupt
            usb.ier().modify(|_, w| w.ep0ie().set_bit());

            // DEBUG: Verify EP0 interrupt configuration
            let ep0_ier = usb.ep0ier().read();
            info!("🔧 EP0_IER_CFG: SDRXIE={} IDTXIE={} ODRXIE={} EP0IE={}",
                   ep0_ier.sdrxie().bit_is_set(), ep0_ier.idtxie().bit_is_set(),
                   ep0_ier.odrxie().bit_is_set(), usb.ier().read().ep0ie().bit_is_set());
        }
        1 => {
            usb.ep1cfgr().modify(|_, w| unsafe {
                // HT32 EPnCFGR register structure according to documentation:
                // Bits [31] EPEN: Endpoint enable
                // Bits [29] EPTYPE: Transfer type (0=Control/Bulk/Interrupt, 1=Isochronous)
                // Bits [28] EPDIR: Direction (0=OUT, 1=IN)
                // Bits [27:24] EPADR: Endpoint address
                // Bits [16:10] EPLEN: Buffer length (4-byte aligned)
                // Bits [9:0] EPBUFA: Buffer offset address

                let aligned_max_packet_size = ((max_packet_size.min(64) + 3) / 4) * 4; // 4-byte aligned

                w.epbufa().bits(buffer_addr)
                 .eplen().bits((aligned_max_packet_size / 4) as u8) // Store as 4-byte units
                 .epadr().bits(ep_num as u8)
                 .eptype().bit(matches!(ep_type, EndpointType::Isochronous)) // 1=ISO, 0=CTRL/BULK/INTR
                 .epdir().bit(is_in)  // Set direction: 1=IN, 0=OUT
                 .epen().set_bit()
            });

            // Enable endpoint interrupt
            usb.ier().modify(|_, w| w.ep1ie().set_bit());
        }
        2 => {
            usb.ep2cfgr().modify(|_, w| unsafe {
                // Apply proper HT32 EPnCFGR register structure
                let aligned_max_packet_size = ((max_packet_size.min(64) + 3) / 4) * 4; // 4-byte aligned

                w.epbufa().bits(buffer_addr)
                 .eplen().bits((aligned_max_packet_size / 4) as u8) // Store as 4-byte units
                 .epadr().bits(ep_num as u8)
                 .eptype().bit(matches!(ep_type, EndpointType::Isochronous)) // 1=ISO, 0=CTRL/BULK/INTR
                 .epdir().bit(is_in)  // Set direction: 1=IN, 0=OUT
                 .epen().set_bit()
            });

            // Enable endpoint interrupt
            usb.ier().modify(|_, w| w.ep2ie().set_bit());
        }
        3 => {
            usb.ep3cfgr().modify(|_, w| unsafe {
                // Apply proper HT32 EPnCFGR register structure
                let aligned_max_packet_size = ((max_packet_size.min(64) + 3) / 4) * 4; // 4-byte aligned

                w.epbufa().bits(buffer_addr)
                 .eplen().bits((aligned_max_packet_size / 4) as u8) // Store as 4-byte units
                 .epadr().bits(ep_num as u8)
                 .eptype().bit(matches!(ep_type, EndpointType::Isochronous)) // 1=ISO, 0=CTRL/BULK/INTR
                 .epdir().bit(is_in)  // Set direction: 1=IN, 0=OUT
                 .epen().set_bit()
            });

            // Enable endpoint interrupt
            usb.ier().modify(|_, w| w.ep3ie().set_bit());
        }
        4..=7 => {
            // For endpoints 4-7, the approach would be similar but using EP4CFGR-EP7CFGR
            // This would need to be implemented when supporting more endpoints
            warn!("🔧 EP_CONFIG: Endpoint {} not yet implemented", ep_num);
        }
        _ => {
            error!("🔧 EP_CONFIG: Invalid endpoint number {}", ep_num);
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
    let buffer_addr = get_endpoint_buffer_addr(ep_num, false); // false = OUT direction
    let bytes_to_read = buf.len().min(MAX_PACKET_SIZE);

    // Get the actual data length from EPnTCR (Transfer Count Register)
    // EP0 has separate TXCNT/RXCNT fields, EP1-3 have combined TCNT field
    let data_len = match ep_num {
        0 => usb.ep0tcr().read().rxcnt().bits() as usize, // EP0 OUT for setup/data
        1 => usb.ep1tcr().read().tcnt().bits() as usize,
        2 => usb.ep2tcr().read().tcnt().bits() as usize,
        3 => usb.ep3tcr().read().tcnt().bits() as usize,
        _ => 0,
    };

    let actual_len = bytes_to_read.min(data_len);

    // Copy data from USB SRAM to the user buffer using proper hardware access
    let src_start = buffer_addr as usize;

    if src_start + actual_len <= EP_SRAM_SIZE {
        read_usb_sram_bytes(src_start, &mut buf[..actual_len]);
    } else {
        return Err(EndpointError::BufferOverflow);
    }

    // Set NAKRX to indicate data has been read
    match ep_num {
        0 => usb.ep0csr().modify(|_, w| w.nakrx().set_bit()),
        1 => usb.ep1csr().modify(|_, w| w.nakrx().set_bit()),
        2 => usb.ep2csr().modify(|_, w| w.nakrx().set_bit()),
        3 => usb.ep3csr().modify(|_, w| w.nakrx().set_bit()),
        _ => {}
    }

    Ok(actual_len)
}

/// Get endpoint buffer address in USB SRAM
/// EP0 buffer layout according to HT32F52352 USB requirements
/// - SETUP buffer: 8 bytes at offset 0x000
/// - TX buffer: 64 bytes at offset 0x008
/// - RX buffer: 64 bytes at offset 0x048
/// - Total EP0 allocation: 136 bytes (0x000-0x087)
const EP0_SETUP_OFFSET: u16 = 0x000;
const EP0_TX_OFFSET: u16 = 0x008;
const EP0_RX_OFFSET: u16 = 0x048;
const EP0_TOTAL_SIZE: u16 = 136; // 8 + 64 + 64

/// Get endpoint buffer address based on EP number and direction
/// For EP0, returns appropriate SETUP/TX/RX offset based on direction
/// For EP1-7, returns allocated buffer from remaining 960 bytes
fn get_endpoint_buffer_addr(ep_num: usize, is_in: bool) -> u16 {
    if ep_num == 0 {
        // EP0 has separate SETUP, TX, and RX buffers
        if is_in {
            EP0_TX_OFFSET // IN direction uses TX buffer
        } else {
            EP0_RX_OFFSET // OUT direction uses RX buffer
        }
    } else {
        // EP1-7 use remaining 960 bytes (1024 - 136 for EP0)
        // Start after EP0 allocation, distribute evenly
        let available_for_others = 960;
        let bytes_per_ep = available_for_others / 7; // ~137 bytes per endpoint
        EP0_TOTAL_SIZE + ((ep_num - 1) * bytes_per_ep) as u16
    }
}

/// Get EP0 SETUP buffer address for control transfers
fn get_ep0_setup_addr() -> u16 {
    EP0_SETUP_OFFSET
}

async fn write_endpoint_data(addr: EndpointAddress, buf: &[u8]) -> Result<(), EndpointError> {
    let usb = unsafe { &*pac::Usb::ptr() };
    let ep_num = addr.index();

    if buf.len() > MAX_PACKET_SIZE {
        return Err(EndpointError::BufferOverflow);
    }

    // Copy data to USB SRAM buffer
    let buffer_addr = get_endpoint_buffer_addr(ep_num, true); // true = IN direction

    // Copy data from user buffer to USB SRAM using proper hardware access
    let dst_start = buffer_addr as usize;
    let dst_end = dst_start + buf.len();

    if dst_end <= EP_SRAM_SIZE {
        write_usb_sram_bytes(dst_start, buf);
    } else {
        return Err(EndpointError::BufferOverflow);
    }

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

    debug!("📋 SETUP_WAIT: Waiting for setup packet");

    // Wait for setup packet interrupt with timeout to prevent hanging when no host is connected
    // For HT32F52352, setup packets are handled via interrupt and EP0 buffer
    let mut timeout = 5000; // 5 second timeout for setup packet (reasonable for test environment)
    while !usb.isr().read().ep0if().bit_is_set() && timeout > 0 {
        // Brief delay to prevent busy-waiting
        embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
        timeout -= 1;
    }

    // If timeout occurred, return empty setup packet (no host connected)
    if timeout == 0 {
        debug!("📋 SETUP_TIMEOUT: No setup packet received within timeout - no host connected");
        return [0u8; 8];
    }

    // Read setup packet from EP0 SETUP buffer at offset 0x000 in USB SRAM
    let mut packet = [0u8; 8];

    // Setup packets are always 8 bytes and start at EP0 SETUP buffer (address 0x000)
    let setup_addr = get_ep0_setup_addr() as usize;
    read_usb_sram_bytes(setup_addr, &mut packet);

    // Only log if we got a non-zero setup packet (host is actually communicating)
    if packet != [0u8; 8] {
        info!("📋 SETUP_PACKET: Setup packet received: [{:02X}, {:02X}, {:02X}, {:02X}, {:02X}, {:02X}, {:02X}, {:02X}]",
              packet[0], packet[1], packet[2], packet[3], packet[4], packet[5], packet[6], packet[7]);

        // Decode setup packet for debugging
        let bm_request_type = packet[0];
        let b_request = packet[1];
        let w_value = u16::from_le_bytes([packet[2], packet[3]]);
        let w_index = u16::from_le_bytes([packet[4], packet[5]]);
        let w_length = u16::from_le_bytes([packet[6], packet[7]]);

        info!("📋 SETUP_DECODE: type={:#02X} request={:#02X} value={:#04X} index={:#04X} length={}",
             bm_request_type, b_request, w_value, w_index, w_length);

        // Check for SET_CONFIGURATION request
        if bm_request_type == 0x00 && b_request == 0x09 { // Standard device request, SET_CONFIGURATION
            if !DEVICE_CONFIGURED.load(Ordering::Acquire) {
                DEVICE_CONFIGURED.store(true, Ordering::Release);
                DEVICE_CONFIG_SIGNAL.signal(());
                info!("🎯 SETUP_DECODE: SET_CONFIGURATION received, device configured, signaling endpoint waiters");
            }
        }
    } else {
        debug!("📋 SETUP_PACKET: No setup packet data available");
    }

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


async fn usb_reset() {
    let usb = unsafe { &*pac::Usb::ptr() };

    info!("🔄 USB_RESET: Starting USB reset slow path (DTRST already done in ISR)");

    // USB Reset - Clear CSR, except for DP pull up (DPPUEN)
    // This matches the ChibiOS implementation: USB->CSR &= USBCSR_DPPUEN;
    // Note: SRAMRSTC (DATA PID reset) already executed in ISR for optimal timing
    usb.csr().modify(|r, w| unsafe {
        // Preserve only DPPUEN bit, clear all other bits
        let dppuen_value = r.dppuen().bit();
        w.bits(0); // Clear all bits first
        w.dppuen().bit(dppuen_value) // Restore DPPUEN only
    });

    info!("🔄 USB_RESET: CSR cleared except DPPUEN");

    // Post reset initialization - reset endpoint memory allocation
    // In ChibiOS this sets usbp->epmem_next = 8;
    // For our implementation, this is handled by the endpoint buffer allocation system

    // 🔄 关键修正：EP0重新配置移除 - 避免与ISR中的DTRST操作冲突
    // configure_control_endpoint() 会在Driver::start()时调用，这里不需要重复
    // 这确保了DTRST操作只在ISR中发生一次
    info!("🔄 USB_RESET: EP0 configuration skipped - will be handled by Driver::start()");

    // Re-enable USB interrupts after reset
    // Matching ChibiOS: USBIER_UGIE | USBIER_SOFIE | USBIER_URSTIE | USBIER_RSMIE | USBIER_SUSPIE | USBIER_EP0IE
    usb.ier().modify(|_, w| {
        w.ugie().set_bit()     // USB global interrupt enable
         .sofie().set_bit()    // Start of Frame interrupt
         .urstie().set_bit()   // USB reset interrupt
         .rsmie().set_bit()    // Resume interrupt
         .suspie().set_bit()   // Suspend interrupt
         .ep0ie().set_bit()    // Endpoint 0 interrupt
    });

    info!("✅ USB_RESET: USB reset slow path completed (DTRST done only in ISR)");
}
// MAIN LOOP of async task 
async fn poll_usb_events(bus: &mut Bus<'_>) -> Event {
    // Wait for USB interrupt signal before checking events
    // This is CRITICAL - embassy-usb expects poll() to block until an event occurs
    USB_EVENT_SIGNAL.wait().await;
    debug!("🚀 POLL_USB_EVENTS: Interrupt signal received, checking atomic flags");

    // 🔴 关键：原子交换，读取旧值并清除标志 - 无锁操作
    // 这避免了在异步任务中直接访问硬件寄存器，防止竞争条件

    // 1. 优先检查Reset事件 - 最重要的枚举事件
    // 🔴 关键：慢速路径 - DTRST已在ISR中完成，这里只做协议栈状态管理
    if IRQ_RESET.load(Ordering::Acquire) {
        IRQ_RESET.store(false, Ordering::Release);
        info!("🔄 POLL_USB_EVENTS: USB reset detected via atomic flag (DTRST done in ISR)");

        // Call USB reset slow path - only handles protocol stack state management
        // DTRST operation already completed in ISR for optimal timing
        usb_reset().await;

        // Force clear all interrupt flags to prevent sticky flag issues
        let usb = unsafe { &*pac::Usb::ptr() };
        unsafe {
            usb.isr().write(|w| w.bits(0xFFFFFFFF));
        }

        info!("✅ POLL_USB_EVENTS: USB reset slow path handled, returning PowerDetected");
        return Event::PowerDetected;
    }

    // 2. 检查Resume事件
    if IRQ_RESUME.load(Ordering::Acquire) {
        IRQ_RESUME.store(false, Ordering::Release);
        info!("▶️  POLL_USB_EVENTS: USB resume detected via atomic flag");
        return Event::Resume;
    }

    // 3. 检查Suspend事件
    if IRQ_SUSPEND.load(Ordering::Acquire) {
        IRQ_SUSPEND.store(false, Ordering::Release);
        info!("⏸️  POLL_USB_EVENTS: USB suspend detected via atomic flag");
        return Event::Suspend;
    }

    // 4. 检查SOF事件 - 表示枚举成功
    if IRQ_SOF.load(Ordering::Acquire) {
        IRQ_SOF.store(false, Ordering::Release);
        debug!("⏱️  POLL_USB_EVENTS: USB SOF detected via atomic flag");
        // SOF事件是正常的，不需要返回特殊事件
        // 继续检查其他事件
    }

    // 5. 检查Endpoint中断事件
    let mut endpoint_event = false;

    if IRQ_EP0.load(Ordering::Acquire) {
        IRQ_EP0.store(false, Ordering::Release);
        info!("📋 POLL_USB_EVENTS: EP0 interrupt detected via atomic flag");
        endpoint_event = true;
    }

    if IRQ_EP1.load(Ordering::Acquire) {
        IRQ_EP1.store(false, Ordering::Release);
        info!("📥 POLL_USB_EVENTS: EP1 interrupt detected via atomic flag");
        endpoint_event = true;
    }

    if IRQ_EP2.load(Ordering::Acquire) {
        IRQ_EP2.store(false, Ordering::Release);
        info!("📥 POLL_USB_EVENTS: EP2 interrupt detected via atomic flag");
        endpoint_event = true;
    }

    if IRQ_EP3.load(Ordering::Acquire) {
        IRQ_EP3.store(false, Ordering::Release);
        info!("📥 POLL_USB_EVENTS: EP3 interrupt detected via atomic flag");
        endpoint_event = true;
    }

    // 如果有endpoint事件，继续正常处理
    // 设备配置现在通过SET_CONFIGURATION请求检测，而不是通过endpoint事件
    if endpoint_event {
        debug!("🔧 POLL_USB_EVENTS: Endpoint events processed, continuing poll");
        return Event::Suspend;
    }

    // For HT32F52352, we need to trigger PowerDetected once to enable the device
    // Check if we've already sent PowerDetected
    if !bus.power_detected_sent.load(Ordering::Relaxed) {
        info!("⚡ POLL_USB_EVENTS: Returning PowerDetected to trigger device enable");
        bus.power_detected_sent.store(true, Ordering::Relaxed);
        return Event::PowerDetected;
    }

    // 没有检测到事件 - 返回Suspend表示无活动
    // 减少日志频率避免spam
    static mut SUSPEND_COUNT: u32 = 0;
    unsafe {
        SUSPEND_COUNT += 1;
        if SUSPEND_COUNT % 100 == 1 {
            debug!("⏸️  POLL_USB_EVENTS: No atomic flags set, returning suspend ({})", SUSPEND_COUNT);
        }
    }
    Event::Suspend
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

    // 1. 显式设置 DPPUEN（连接）
    info!("🚀 USB_BUS_ENABLE: DPPUEN set_bit");
    usb.csr().modify(|r, w| unsafe {
        // 使用 RMW 方式，只修改 DPPUEN，并保留其他可能需要的位（如 PDWN）
        w.bits(0xFFFFFFFF);
        w.dppuen().set_bit() 
         // 如果需要，这里可以根据 r 来保留 r.pdwn().bit()
    });

    // 🔴 CRITICAL: Step 2 - Reset USB SRAM to reset DATA toggle states
    // This resets all endpoint states including DATA0/DATA1 sequences
    usb.csr().modify(|_, w| w.sramrstc().set_bit());

          // Note: In embassy, we might need a different approach for delays
    for _ in 0..1000 {
        cortex_m::asm::nop();
    }

    // Step 3: Clear SRAM reset
    usb.csr().modify(|_, w| w.sramrstc().clear_bit());

    // 2. 重新清除所有挂起的中断，为 Reset 做准备
    // info!("🚀 USB_BUS_ENABLE: clean all interrupt");
    // unsafe {
    //     usb.isr().write(|w| w.bits(0xFFFFFFFF));
    // }
    
    // 3. (可选但推荐) 确保 IER (中断使能寄存器) 处于正确状态
    // 如果 disable_usb_device 清除了 IER，这里需要恢复它。
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

/// USB SRAM base address - hardware endpoint buffer memory at 0x400AA000
/// This provides direct access to HT32 USB controller's 1024-byte endpoint SRAM
const USB_SRAM_BASE: *mut u32 = 0x400AA000 as *mut u32;

/// USB endpoint memory access functions - 32-bit word access with proper byte ordering
/// HT32 USB SRAM requires 32-bit word access for proper operation
fn read_usb_sram_word(offset: usize) -> u32 {
    unsafe {
        USB_SRAM_BASE.add(offset / 4).read_volatile()
    }
}

fn write_usb_sram_word(offset: usize, value: u32) {
    unsafe {
        USB_SRAM_BASE.add(offset / 4).write_volatile(value)
    }
}

/// Read bytes from USB SRAM with proper 8-bit access
fn read_usb_sram_bytes(offset: usize, buf: &mut [u8]) {
    for (i, byte) in buf.iter_mut().enumerate() {
        let word_offset = offset + i;
        let word_pos = word_offset % 4;
        let word = read_usb_sram_word(word_offset - word_pos);
        *byte = ((word >> (word_pos * 8)) & 0xFF) as u8;
    }
}

/// Write bytes to USB SRAM with proper 8-bit access
fn write_usb_sram_bytes(offset: usize, buf: &[u8]) {
    for (i, &byte) in buf.iter().enumerate() {
        let word_offset = offset + i;
        let word_pos = word_offset % 4;
        let word_addr = word_offset - word_pos;

        // Read current word, modify byte, write back
        let mut word = read_usb_sram_word(word_addr);
        word = (word & !(0xFF << (word_pos * 8))) | ((byte as u32) << (word_pos * 8));
        write_usb_sram_word(word_addr, word);
    }
}

/// USB event signal for ISR to Task communication - avoids critical section deadlock
static USB_EVENT_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Atomic flags for interrupt-to-async task bridging
/// These flags are set in the ISR and cleared in the async task to avoid race conditions
static IRQ_RESET: AtomicBool = AtomicBool::new(false);
static IRQ_SUSPEND: AtomicBool = AtomicBool::new(false);
static IRQ_RESUME: AtomicBool = AtomicBool::new(false);
static IRQ_SOF: AtomicBool = AtomicBool::new(false);
static IRQ_EP0: AtomicBool = AtomicBool::new(false);
static IRQ_EP1: AtomicBool = AtomicBool::new(false);
static IRQ_EP2: AtomicBool = AtomicBool::new(false);
static IRQ_EP3: AtomicBool = AtomicBool::new(false);

/// Device configuration state tracking
static DEVICE_CONFIGURED: AtomicBool = AtomicBool::new(false);
static DEVICE_CONFIG_SIGNAL: Signal<CriticalSectionRawMutex, ()> = Signal::new();

/// Wait for USB event and reset the signal
pub async fn wait_for_usb_event() {
    USB_EVENT_SIGNAL.wait().await;
}


/// USB interrupt handler
///
/// This function should be called from the global interrupt handler for USB interrupts
/// Uses atomic flags to bridge interrupt context to async tasks - fast ISR approach
pub unsafe fn on_usb_interrupt() {
    let usb = unsafe { &*pac::Usb::ptr() };
    let isr = usb.isr().read();
    let mut event_signaled = false;

    // Always log USB interrupts for debugging enumeration
    info!("🔌 USB_IRQ: ISR = {:#010x}", isr.bits());

    // Handle reset interrupt - CRITICAL for enumeration
    // 🔴 关键修复：在ISR中只执行最少的操作：DTRST和基本状态设置
    if isr.urstif().bit_is_set() {
        // 1. 设置软件状态标志 - 原子操作，无锁
        IRQ_RESET.store(true, Ordering::Release);

        // 2. 清除 URSTIF 硬件标志
        usb.isr().modify(|_, w| w.urstif().set_bit());

        // 3. 🔴 关键修复：立即执行 EP0 DATA PID 复位 (SRAMRSTC)
        // 这是唯一必须在ISR中完成的时序关键操作
        usb.csr().modify(|_, w| w.sramrstc().set_bit());

        usb.csr().modify(|_, w| w.sramrstc().clear_bit());

        event_signaled = true;
        info!("✅ USB_IRQ_RESET: Reset flag set and DTRST executed in ISR only");
        let csr_after_dtrst = usb.csr().read().bits();
        info!("🔍 CSR_AFTER_DTRST: {:#010x}", csr_after_dtrst);
    }

    // Handle suspend interrupt
    if isr.suspif().bit_is_set() {
        // 1. 设置软件状态标志
        IRQ_SUSPEND.store(true, Ordering::Release);

        // 2. 清除 SUSPIF 硬件标志
        usb.isr().modify(|_, w| w.suspif().set_bit());

        event_signaled = true;
        info!("✅ USB_IRQ_SUSPEND: Suspend flag set, deferring handling to async task");
    }

    // Handle resume interrupt
    if isr.rsmif().bit_is_set() {
        // 1. 设置软件状态标志
        IRQ_RESUME.store(true, Ordering::Release);

        // 2. 清除 RSMIF 硬件标志
        usb.isr().modify(|_, w| w.rsmif().set_bit());

        event_signaled = true;
        info!("✅ USB_IRQ_RESUME: Resume flag set, deferring handling to async task");
    }

    // Handle endpoint interrupts - CRITICAL for control transfers
    // 只设置标志位，具体处理留给异步任务
    for ep in 0..4 { // 只处理EP0-3，其他endpoint暂未实现
        let ep_flag = match ep {
            0 => isr.ep0if().bit_is_set(),
            1 => isr.ep1if().bit_is_set(),
            2 => isr.ep2if().bit_is_set(),
            3 => isr.ep3if().bit_is_set(),
            _ => false,
        };

        if ep_flag {
            // 设置对应endpoint的原子标志
            match ep {
                0 => IRQ_EP0.store(true, Ordering::Release),
                1 => IRQ_EP1.store(true, Ordering::Release),
                2 => IRQ_EP2.store(true, Ordering::Release),
                3 => IRQ_EP3.store(true, Ordering::Release),
                _ => {}
            }

            // 清除硬件endpoint中断标志
            match ep {
                0 => usb.isr().modify(|_, w| w.ep0if().set_bit()),
                1 => usb.isr().modify(|_, w| w.ep1if().set_bit()),
                2 => usb.isr().modify(|_, w| w.ep2if().set_bit()),
                3 => usb.isr().modify(|_, w| w.ep3if().set_bit()),
                _ => {}
            }

            event_signaled = true;
            info!("✅ USB_IRQ_EP{}: Endpoint {} flag set, deferring handling to async task", ep, ep);
        }
    }

    // Handle SOF (Start of Frame) interrupts - indicates successful enumeration
    if isr.sofif().bit_is_set() {
        // 设置SOF标志
        IRQ_SOF.store(true, Ordering::Release);
        usb.isr().modify(|_, w| w.sofif().set_bit());
        event_signaled = true;
        info!("✅ USB_IRQ_SOF: SOF flag set, enumeration successful");
    }

    // 最后唤醒异步任务处理具体逻辑
    // 这避免了在ISR中做复杂处理，提高了中断响应速度
    if event_signaled {
        USB_EVENT_SIGNAL.signal(());
        info!("🚀 USB_IRQ_SIGNAL: USB event signaled to embassy-usb stack");
    }
}
