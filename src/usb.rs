//! USB device driver for HT32F523xx.
//!
//! The HT32 controller has one bidirectional control endpoint, seven
//! direction-fixed data endpoints, and 1 KiB of endpoint SRAM. Endpoint CSR
//! status bits use toggle-on-write-one semantics; treating them as ordinary
//! read/write bits prevents NAK and STALL transitions from taking effect.

use core::future::poll_fn;
use core::marker::PhantomData;
use core::ptr::{read_volatile, write_volatile};
use core::sync::atomic::{AtomicBool, Ordering};
use core::task::Poll;

use embassy_sync::waitqueue::AtomicWaker;
use embassy_usb_driver::{
    Direction, EndpointAddress, EndpointAllocError, EndpointError, EndpointInfo, EndpointType,
    Event, Unsupported,
};

use crate::gpio::{Pin, mode};
use crate::pac;

#[cfg(feature = "defmt")]
use defmt::{info, warn};

#[cfg(not(feature = "defmt"))]
macro_rules! info {
    ($($arg:tt)*) => {};
}

#[cfg(not(feature = "defmt"))]
macro_rules! warn {
    ($($arg:tt)*) => {};
}

const EP_COUNT: usize = 8;
const MAX_PACKET_SIZE: u16 = 64;
const USB_SRAM_SIZE: usize = 1024;
const USB_SRAM_BASE: *mut u32 = 0x400A_A000 as *mut u32;

// EP0 SETUP is fixed at offset 0. EPBUFA points at the TX buffer and EPLEN is
// both the TX allocation size and the offset from TX to RX.
const EP0_SETUP_OFFSET: usize = 0;
const EP0_TX_OFFSET: usize = 8;
const EP0_RX_OFFSET: usize = EP0_TX_OFFSET + MAX_PACKET_SIZE as usize;
const DATA_EP_BASE: usize = EP0_RX_OFFSET + MAX_PACKET_SIZE as usize;

// USB global register bits.
const CSR_PDWN: u32 = 1 << 2;
const CSR_LPMODE: u32 = 1 << 3;
const CSR_ADRSET: u32 = 1 << 8;
const CSR_SRAMRSTC: u32 = 1 << 9;
const CSR_DPPUEN: u32 = 1 << 10;
const CSR_DPWKEN: u32 = 1 << 11;

const IER_UGIE: u32 = 1 << 0;
const IER_URSTIE: u32 = 1 << 2;
const IER_RSMIE: u32 = 1 << 3;
const IER_SUSPIE: u32 = 1 << 4;
const IER_EP0IE: u32 = 1 << 8;

const ISR_URSTIF: u32 = 1 << 2;
const ISR_RSMIF: u32 = 1 << 3;
const ISR_SUSPIF: u32 = 1 << 4;
const ISR_ESOFIF: u32 = 1 << 5;

// Endpoint registers and fields.
const EP_REG_STRIDE: usize = 0x14;
const EP_CSR_OFFSET: usize = 0x00;
const EP_IER_OFFSET: usize = 0x04;
const EP_ISR_OFFSET: usize = 0x08;
const EP_TCR_OFFSET: usize = 0x0c;
const EP_CFGR_OFFSET: usize = 0x10;

const EP_CSR_NAKTX: u32 = 1 << 1;
const EP_CSR_STLTX: u32 = 1 << 2;
const EP_CSR_NAKRX: u32 = 1 << 4;
const EP_CSR_STLRX: u32 = 1 << 5;

const EP_INT_ODRX: u32 = 1 << 1;
const EP_INT_IDTX: u32 = 1 << 4;
const EP_INT_SDRX: u32 = 1 << 9;

const EP_CFGR_EPEN: u32 = 1 << 31;
const EP_CFGR_EPTYPE: u32 = 1 << 29;
const EP_CFGR_EPDIR: u32 = 1 << 28;

const NEW_WAKER: AtomicWaker = AtomicWaker::new();
static BUS_WAKER: AtomicWaker = AtomicWaker::new();
static EP_IN_WAKERS: [AtomicWaker; EP_COUNT] = [NEW_WAKER; EP_COUNT];
static EP_OUT_WAKERS: [AtomicWaker; EP_COUNT] = [NEW_WAKER; EP_COUNT];
static EP_ENABLED_WAKERS: [AtomicWaker; EP_COUNT] = [NEW_WAKER; EP_COUNT];

static BUS_RESET: AtomicBool = AtomicBool::new(false);
static BUS_SUSPEND: AtomicBool = AtomicBool::new(false);
static BUS_RESUME: AtomicBool = AtomicBool::new(false);
static EP0_SETUP: AtomicBool = AtomicBool::new(false);
static EP_IN_COMPLETE: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];
static EP_OUT_READY: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];
static EP_ENABLED: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];

/// USB DM pin. HT32F523xx exposes USB on the dedicated PC6 function.
pub type UsbDm<const PORT: char, const PIN: u8> = Pin<PORT, PIN, mode::AlternateFunction<0>>;

/// USB DP pin. HT32F523xx exposes USB on the dedicated PC7 function.
pub type UsbDp<const PORT: char, const PIN: u8> = Pin<PORT, PIN, mode::AlternateFunction<0>>;

/// Statically checked USB pin pair.
pub struct UsbPins<const DM_PORT: char, const DM_PIN: u8, const DP_PORT: char, const DP_PIN: u8> {
    pub dm: UsbDm<DM_PORT, DM_PIN>,
    pub dp: UsbDp<DP_PORT, DP_PIN>,
}

impl<const DM_PORT: char, const DM_PIN: u8, const DP_PORT: char, const DP_PIN: u8>
    UsbPins<DM_PORT, DM_PIN, DP_PORT, DP_PIN>
{
    pub fn new(dm: UsbDm<DM_PORT, DM_PIN>, dp: UsbDp<DP_PORT, DP_PIN>) -> Self {
        Self { dm, dp }
    }
}

/// USB peripheral ownership token.
pub struct Usb {
    _private: (),
}

impl Usb {
    pub fn new() -> Self {
        Self { _private: () }
    }
}

impl Default for Usb {
    fn default() -> Self {
        Self::new()
    }
}

/// Driver configuration.
pub struct Config {
    /// The current controller does not expose a usable VBUS detector.
    pub vbus_detection: bool,
    /// Reserved for a future board-specific VBUS detector.
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

/// Embassy USB driver.
pub struct Driver<'d> {
    _phantom: PhantomData<&'d mut Usb>,
    allocated_eps: u16,
}

impl<'d> Driver<'d> {
    pub fn new(_usb: Usb, config: Config) -> Self {
        if config.vbus_detection || config.enable_vbus_detect {
            warn!("HT32 USB VBUS detection is not implemented; using always-powered mode");
        }
        initialize_hardware();
        Self {
            _phantom: PhantomData,
            allocated_eps: 1,
        }
    }

    fn allocate_address(
        &mut self,
        requested: Option<EndpointAddress>,
        direction: Direction,
    ) -> Result<EndpointAddress, EndpointAllocError> {
        let addr = if let Some(addr) = requested {
            if addr.index() == 0 || addr.index() >= EP_COUNT || addr.direction() != direction {
                return Err(EndpointAllocError);
            }
            addr
        } else {
            let Some(index) = (1..EP_COUNT).find(|index| self.allocated_eps & (1 << index) == 0)
            else {
                return Err(EndpointAllocError);
            };
            EndpointAddress::from_parts(index, direction)
        };

        let mask = 1u16 << addr.index();
        if self.allocated_eps & mask != 0 {
            return Err(EndpointAllocError);
        }
        self.allocated_eps |= mask;
        Ok(addr)
    }
}

pub struct Bus<'d> {
    _phantom: PhantomData<&'d mut Usb>,
    power_detected: bool,
}

pub struct ControlPipe<'d> {
    _phantom: PhantomData<&'d mut Usb>,
    max_packet_size: u16,
}

pub struct Endpoint<'d, D> {
    _phantom: PhantomData<&'d mut Usb>,
    _direction: PhantomData<D>,
    info: EndpointInfo,
}

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
        validate_packet_size(max_packet_size)?;
        let addr = self.allocate_address(ep_addr, Direction::In)?;
        configure_data_endpoint(addr, ep_type, max_packet_size);
        Ok(Endpoint {
            _phantom: PhantomData,
            _direction: PhantomData,
            info: EndpointInfo {
                addr,
                ep_type,
                max_packet_size,
                interval_ms: interval,
            },
        })
    }

    fn alloc_endpoint_out(
        &mut self,
        ep_type: EndpointType,
        ep_addr: Option<EndpointAddress>,
        max_packet_size: u16,
        interval: u8,
    ) -> Result<Self::EndpointOut, EndpointAllocError> {
        validate_packet_size(max_packet_size)?;
        let addr = self.allocate_address(ep_addr, Direction::Out)?;
        configure_data_endpoint(addr, ep_type, max_packet_size);
        Ok(Endpoint {
            _phantom: PhantomData,
            _direction: PhantomData,
            info: EndpointInfo {
                addr,
                ep_type,
                max_packet_size,
                interval_ms: interval,
            },
        })
    }

    fn start(self, control_max_packet_size: u16) -> (Self::Bus, Self::ControlPipe) {
        let control_max_packet_size = control_max_packet_size.min(MAX_PACKET_SIZE);
        configure_control_endpoint(control_max_packet_size);
        (
            Bus {
                _phantom: PhantomData,
                power_detected: false,
            },
            ControlPipe {
                _phantom: PhantomData,
                max_packet_size: control_max_packet_size,
            },
        )
    }
}

impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, In> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
        wait_endpoint_enabled(self.info.addr.index()).await;
    }
}

impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, Out> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
        wait_endpoint_enabled(self.info.addr.index()).await;
    }
}

impl<'d> embassy_usb_driver::EndpointIn for Endpoint<'d, In> {
    async fn write(&mut self, data: &[u8]) -> Result<(), EndpointError> {
        let ep = self.info.addr.index();
        if data.len() > self.info.max_packet_size as usize {
            return Err(EndpointError::BufferOverflow);
        }
        if !EP_ENABLED[ep].load(Ordering::Acquire) {
            return Err(EndpointError::Disabled);
        }

        EP_IN_COMPLETE[ep].store(false, Ordering::Release);
        write_usb_sram(endpoint_buffer_offset(ep), data);
        write_ep_reg(ep, EP_TCR_OFFSET, data.len() as u32);
        set_ep_status(ep, EP_CSR_NAKTX, false);
        wait_endpoint_transfer(ep, true).await
    }
}

impl<'d> embassy_usb_driver::EndpointOut for Endpoint<'d, Out> {
    async fn read(&mut self, data: &mut [u8]) -> Result<usize, EndpointError> {
        let ep = self.info.addr.index();
        if data.len() < self.info.max_packet_size as usize {
            return Err(EndpointError::BufferOverflow);
        }
        if !EP_ENABLED[ep].load(Ordering::Acquire) {
            return Err(EndpointError::Disabled);
        }

        EP_OUT_READY[ep].store(false, Ordering::Release);
        set_ep_status(ep, EP_CSR_NAKRX, false);
        wait_endpoint_transfer(ep, false).await?;

        let len = read_ep_reg(ep, EP_TCR_OFFSET) as usize;
        if len > data.len() {
            return Err(EndpointError::BufferOverflow);
        }
        read_usb_sram(endpoint_buffer_offset(ep), &mut data[..len]);
        Ok(len)
    }
}

impl<'d> embassy_usb_driver::ControlPipe for ControlPipe<'d> {
    fn max_packet_size(&self) -> usize {
        self.max_packet_size as usize
    }

    async fn setup(&mut self) -> [u8; 8] {
        poll_fn(|cx| {
            EP_OUT_WAKERS[0].register(cx.waker());
            if take_flag(&EP0_SETUP) {
                Poll::Ready(())
            } else {
                Poll::Pending
            }
        })
        .await;

        let mut packet = [0; 8];
        read_usb_sram(EP0_SETUP_OFFSET, &mut packet);
        packet
    }

    async fn data_out(
        &mut self,
        data: &mut [u8],
        _first: bool,
        _last: bool,
    ) -> Result<usize, EndpointError> {
        EP_OUT_READY[0].store(false, Ordering::Release);
        set_ep_status(0, EP_CSR_NAKRX, false);
        wait_control_transfer(false).await?;

        let len = (read_ep_reg(0, EP_TCR_OFFSET) >> 16) as usize & 0x7f;
        if len > data.len() {
            return Err(EndpointError::BufferOverflow);
        }
        read_usb_sram(EP0_RX_OFFSET, &mut data[..len]);
        Ok(len)
    }

    async fn data_in(
        &mut self,
        data: &[u8],
        _first: bool,
        last: bool,
    ) -> Result<(), EndpointError> {
        if data.len() > self.max_packet_size as usize {
            return Err(EndpointError::BufferOverflow);
        }

        EP_IN_COMPLETE[0].store(false, Ordering::Release);
        write_usb_sram(EP0_TX_OFFSET, data);
        write_ep_reg(0, EP_TCR_OFFSET, data.len() as u32);
        set_ep_status(0, EP_CSR_NAKTX, false);
        wait_control_transfer(true).await?;

        if last {
            // Arm the zero-length OUT status stage after the last IN packet.
            EP_OUT_READY[0].store(false, Ordering::Release);
            set_ep_status(0, EP_CSR_NAKRX, false);
        }
        Ok(())
    }

    async fn accept(&mut self) {
        // A control OUT request (or a no-data request) is accepted by sending
        // an IN zero-length status packet.
        EP_IN_COMPLETE[0].store(false, Ordering::Release);
        write_ep_reg(0, EP_TCR_OFFSET, 0);
        set_ep_status(0, EP_CSR_NAKTX, false);
        let _ = wait_control_transfer(true).await;
    }

    async fn reject(&mut self) {
        set_ep_status(0, EP_CSR_STLTX, true);
        set_ep_status(0, EP_CSR_STLRX, true);
    }

    async fn accept_set_address(&mut self, addr: u8) {
        self.accept().await;
        set_device_address(addr);
    }
}

impl<'d> embassy_usb_driver::Bus for Bus<'d> {
    async fn poll(&mut self) -> Event {
        if !self.power_detected {
            self.power_detected = true;
            return Event::PowerDetected;
        }

        poll_fn(|cx| {
            BUS_WAKER.register(cx.waker());
            if take_flag(&BUS_RESET) {
                reset_hardware();
                Poll::Ready(Event::Reset)
            } else if take_flag(&BUS_SUSPEND) {
                Poll::Ready(Event::Suspend)
            } else if take_flag(&BUS_RESUME) {
                Poll::Ready(Event::Resume)
            } else {
                Poll::Pending
            }
        })
        .await
    }

    fn endpoint_set_stalled(&mut self, addr: EndpointAddress, stalled: bool) {
        let bit = if addr.is_in() {
            EP_CSR_STLTX
        } else {
            EP_CSR_STLRX
        };
        set_ep_status(addr.index(), bit, stalled);
    }

    fn endpoint_is_stalled(&mut self, addr: EndpointAddress) -> bool {
        let bit = if addr.is_in() {
            EP_CSR_STLTX
        } else {
            EP_CSR_STLRX
        };
        read_ep_reg(addr.index(), EP_CSR_OFFSET) & bit != 0
    }

    fn endpoint_set_enabled(&mut self, addr: EndpointAddress, enabled: bool) {
        let ep = addr.index();
        modify_ep_reg(ep, EP_CFGR_OFFSET, |value| {
            if enabled {
                value | EP_CFGR_EPEN
            } else {
                value & !EP_CFGR_EPEN
            }
        });
        EP_ENABLED[ep].store(enabled, Ordering::Release);
        if !enabled {
            EP_IN_COMPLETE[ep].store(false, Ordering::Release);
            EP_OUT_READY[ep].store(false, Ordering::Release);
        }
        EP_ENABLED_WAKERS[ep].wake();
        EP_IN_WAKERS[ep].wake();
        EP_OUT_WAKERS[ep].wake();
    }

    async fn enable(&mut self) {
        let usb = usb_regs();
        usb.csr().modify(|_, w| w.dppuen().set_bit());
        info!("USB connected");
    }

    async fn disable(&mut self) {
        let usb = usb_regs();
        usb.csr().modify(|_, w| w.dppuen().clear_bit());
        unsafe { usb.ier().write(|w| w.bits(0)) };
        for ep in 1..EP_COUNT {
            EP_ENABLED[ep].store(false, Ordering::Release);
            EP_ENABLED_WAKERS[ep].wake();
            EP_IN_WAKERS[ep].wake();
            EP_OUT_WAKERS[ep].wake();
        }
    }

    async fn remote_wakeup(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported)
    }
}

/// Initialize USB using a statically typed pin pair.
pub fn init_usb_with_pins<
    const DM_PORT: char,
    const DM_PIN: u8,
    const DP_PORT: char,
    const DP_PIN: u8,
>(
    usb: Usb,
    pins: UsbPins<DM_PORT, DM_PIN, DP_PORT, DP_PIN>,
    config: Config,
) -> Driver<'static> {
    // These are the only USB pins on HT32F52342/52. Keep the generic type in
    // the public API but reject an invalid board mapping at startup.
    assert!(DM_PORT == 'C' && DM_PIN == 6 && DP_PORT == 'C' && DP_PIN == 7);
    let _pins = pins;
    Driver::new(usb, config)
}

fn validate_packet_size(max_packet_size: u16) -> Result<(), EndpointAllocError> {
    if max_packet_size == 0 || max_packet_size > MAX_PACKET_SIZE {
        Err(EndpointAllocError)
    } else {
        Ok(())
    }
}

fn initialize_hardware() {
    let usb = usb_regs();

    // Follow the vendor power-up sequence but stay disconnected until
    // embassy-usb receives PowerDetected and calls Bus::enable().
    unsafe {
        usb.csr()
            .write(|w| w.bits(CSR_DPWKEN | CSR_LPMODE | CSR_PDWN));
        usb.isr().write(|w| w.bits(!ISR_ESOFIF));
    }
    usb.csr().modify(|_, w| w.dpwken().clear_bit());

    configure_control_endpoint(MAX_PACKET_SIZE);
    unsafe {
        usb.ier()
            .write(|w| w.bits(IER_UGIE | IER_URSTIE | IER_RSMIE | IER_SUSPIE | IER_EP0IE));
        cortex_m::peripheral::NVIC::unpend(pac::Interrupt::USB);
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::USB);
    }
}

fn reset_hardware() {
    let usb = usb_regs();
    let pull_up = usb.csr().read().bits() & CSR_DPPUEN;

    unsafe {
        usb.csr().write(|w| w.bits(pull_up));
        usb.devar().write(|w| w.bits(0));
    }
    usb.csr().modify(|_, w| w.sramrstc().set_bit());
    usb.csr().modify(|_, w| w.sramrstc().clear_bit());

    EP0_SETUP.store(false, Ordering::Release);
    for ep in 0..EP_COUNT {
        EP_IN_COMPLETE[ep].store(false, Ordering::Release);
        EP_OUT_READY[ep].store(false, Ordering::Release);
        if ep != 0 {
            EP_ENABLED[ep].store(false, Ordering::Release);
            modify_ep_reg(ep, EP_CFGR_OFFSET, |value| value & !EP_CFGR_EPEN);
            EP_ENABLED_WAKERS[ep].wake();
        }
        EP_IN_WAKERS[ep].wake();
        EP_OUT_WAKERS[ep].wake();
    }

    configure_control_endpoint(MAX_PACKET_SIZE);
    unsafe {
        usb.ier()
            .write(|w| w.bits(IER_UGIE | IER_URSTIE | IER_RSMIE | IER_SUSPIE | IER_EP0IE));
    }
}

fn configure_control_endpoint(max_packet_size: u16) {
    // The SETUP area is fixed at SRAM offset zero. Hardware TX begins at
    // EPBUFA=8 and RX begins at EPBUFA+EPLEN=72.
    let cfgr = EP_CFGR_EPEN | ((max_packet_size as u32) << 10) | EP0_TX_OFFSET as u32;
    write_ep_reg(0, EP_CFGR_OFFSET, cfgr);
    write_ep_reg(0, EP_IER_OFFSET, EP_INT_SDRX | EP_INT_IDTX | EP_INT_ODRX);
    write_ep_reg(0, EP_ISR_OFFSET, 0x0fff);
    EP_ENABLED[0].store(true, Ordering::Release);
}

fn configure_data_endpoint(addr: EndpointAddress, ep_type: EndpointType, max_packet_size: u16) {
    let ep = addr.index();
    let mut cfgr = (endpoint_buffer_offset(ep) as u32)
        | ((max_packet_size as u32) << 10)
        | ((ep as u32) << 24);
    if addr.is_in() {
        cfgr |= EP_CFGR_EPDIR;
    }
    if ep_type == EndpointType::Isochronous {
        cfgr |= EP_CFGR_EPTYPE;
    }

    write_ep_reg(ep, EP_CFGR_OFFSET, cfgr);
    write_ep_reg(ep, EP_IER_OFFSET, EP_INT_ODRX | EP_INT_IDTX);
    write_ep_reg(ep, EP_ISR_OFFSET, 0x0fff);

    let usb = usb_regs();
    unsafe {
        usb.ier()
            .modify(|r, w| w.bits(r.bits() | (IER_EP0IE << ep)));
    }
}

fn set_device_address(addr: u8) {
    let usb = usb_regs();
    usb.csr().modify(|_, w| w.adrset().set_bit());
    unsafe { usb.devar().write(|w| w.bits((addr & 0x7f) as u32)) };
}

async fn wait_endpoint_enabled(ep: usize) {
    poll_fn(|cx| {
        EP_ENABLED_WAKERS[ep].register(cx.waker());
        if EP_ENABLED[ep].load(Ordering::Acquire) {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await
}

async fn wait_endpoint_transfer(ep: usize, input: bool) -> Result<(), EndpointError> {
    let flag = if input {
        &EP_IN_COMPLETE[ep]
    } else {
        &EP_OUT_READY[ep]
    };
    let waker = if input {
        &EP_IN_WAKERS[ep]
    } else {
        &EP_OUT_WAKERS[ep]
    };

    poll_fn(|cx| {
        waker.register(cx.waker());
        if !EP_ENABLED[ep].load(Ordering::Acquire) {
            Poll::Ready(Err(EndpointError::Disabled))
        } else if take_flag(flag) {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    })
    .await
}

async fn wait_control_transfer(input: bool) -> Result<(), EndpointError> {
    let flag = if input {
        &EP_IN_COMPLETE[0]
    } else {
        &EP_OUT_READY[0]
    };
    let waker = if input {
        &EP_IN_WAKERS[0]
    } else {
        &EP_OUT_WAKERS[0]
    };

    poll_fn(|cx| {
        waker.register(cx.waker());
        if EP0_SETUP.load(Ordering::Acquire) {
            Poll::Ready(Err(EndpointError::Disabled))
        } else if take_flag(flag) {
            Poll::Ready(Ok(()))
        } else {
            Poll::Pending
        }
    })
    .await
}

fn endpoint_buffer_offset(ep: usize) -> usize {
    let offset = DATA_EP_BASE + (ep - 1) * MAX_PACKET_SIZE as usize;
    debug_assert!(offset + MAX_PACKET_SIZE as usize <= USB_SRAM_SIZE);
    offset
}

// Cortex-M0+ has no atomic read-modify-write instructions. The target's
// AtomicBool therefore exposes load/store but not swap; consume an IRQ flag in
// a short critical section so a newly arriving event cannot be lost.
fn take_flag(flag: &AtomicBool) -> bool {
    critical_section::with(|_| {
        let value = flag.load(Ordering::Acquire);
        flag.store(false, Ordering::Release);
        value
    })
}

fn read_usb_sram(offset: usize, data: &mut [u8]) {
    debug_assert!(offset + data.len() <= USB_SRAM_SIZE);
    for (chunk_index, chunk) in data.chunks_mut(4).enumerate() {
        let word = unsafe { read_volatile(USB_SRAM_BASE.add(offset / 4 + chunk_index)) };
        let bytes = word.to_le_bytes();
        chunk.copy_from_slice(&bytes[..chunk.len()]);
    }
}

fn write_usb_sram(offset: usize, data: &[u8]) {
    debug_assert!(offset + data.len() <= USB_SRAM_SIZE);
    for (chunk_index, chunk) in data.chunks(4).enumerate() {
        let mut bytes = [0; 4];
        bytes[..chunk.len()].copy_from_slice(chunk);
        unsafe {
            write_volatile(
                USB_SRAM_BASE.add(offset / 4 + chunk_index),
                u32::from_le_bytes(bytes),
            )
        };
    }
}

fn usb_regs() -> &'static pac::usb::RegisterBlock {
    unsafe { &*pac::Usb::ptr() }
}

fn ep_reg(ep: usize, offset: usize) -> *mut u32 {
    debug_assert!(ep < EP_COUNT);
    (pac::Usb::ptr() as usize + 0x14 + ep * EP_REG_STRIDE + offset) as *mut u32
}

fn read_ep_reg(ep: usize, offset: usize) -> u32 {
    unsafe { read_volatile(ep_reg(ep, offset)) }
}

fn write_ep_reg(ep: usize, offset: usize, value: u32) {
    unsafe { write_volatile(ep_reg(ep, offset), value) }
}

fn modify_ep_reg(ep: usize, offset: usize, f: impl FnOnce(u32) -> u32) {
    let value = read_ep_reg(ep, offset);
    write_ep_reg(ep, offset, f(value));
}

/// Endpoint CSR status bits are toggle-on-write-one. Writing a conventional
/// read/modify/write value toggles every currently-set status bit and corrupts
/// endpoint state, so only the one bit that needs changing may be written.
fn set_ep_status(ep: usize, bit: u32, desired: bool) {
    let current = read_ep_reg(ep, EP_CSR_OFFSET) & bit != 0;
    if current != desired {
        write_ep_reg(ep, EP_CSR_OFFSET, bit);
    }
}

fn handle_endpoint_interrupt(ep: usize) {
    let flags = read_ep_reg(ep, EP_ISR_OFFSET) & read_ep_reg(ep, EP_IER_OFFSET);
    if flags == 0 {
        return;
    }

    // Endpoint interrupt flags are W1C and must be cleared before the global
    // EPn flag. A global ISR read/modify/write would clear unrelated events.
    write_ep_reg(ep, EP_ISR_OFFSET, flags);

    if ep == 0 && flags & EP_INT_SDRX != 0 {
        EP0_SETUP.store(true, Ordering::Release);
        EP_IN_COMPLETE[0].store(false, Ordering::Release);
        EP_OUT_READY[0].store(false, Ordering::Release);
        EP_OUT_WAKERS[0].wake();
        EP_IN_WAKERS[0].wake();
    }
    if flags & EP_INT_ODRX != 0 {
        EP_OUT_READY[ep].store(true, Ordering::Release);
        EP_OUT_WAKERS[ep].wake();
    }
    if flags & EP_INT_IDTX != 0 {
        EP_IN_COMPLETE[ep].store(true, Ordering::Release);
        EP_IN_WAKERS[ep].wake();
    }

    unsafe {
        usb_regs().isr().write(|w| w.bits(IER_EP0IE << ep));
    }
}

/// Interrupt handler type for applications that use a custom binding layer.
pub struct InterruptHandler;

impl InterruptHandler {
    pub unsafe fn on_interrupt() {
        unsafe { on_usb_interrupt() }
    }
}

/// USB interrupt entry point used by `src/interrupt.rs`.
///
/// # Safety
/// Must only be called from the USB IRQ handler.
pub unsafe fn on_usb_interrupt() {
    let usb = usb_regs();
    let pending = usb.isr().read().bits() & usb.ier().read().bits();

    if pending & ISR_URSTIF != 0 {
        BUS_RESET.store(true, Ordering::Release);
        unsafe { usb.isr().write(|w| w.bits(ISR_URSTIF)) };
        BUS_WAKER.wake();
    }
    if pending & ISR_SUSPIF != 0 {
        BUS_SUSPEND.store(true, Ordering::Release);
        unsafe { usb.isr().write(|w| w.bits(ISR_SUSPIF)) };
        BUS_WAKER.wake();
    }
    if pending & ISR_RSMIF != 0 {
        BUS_RESUME.store(true, Ordering::Release);
        unsafe { usb.isr().write(|w| w.bits(ISR_RSMIF)) };
        BUS_WAKER.wake();
    }

    let endpoint_pending = (pending >> 8) & 0xff;
    for ep in 0..EP_COUNT {
        if endpoint_pending & (1 << ep) != 0 {
            handle_endpoint_interrupt(ep);
        }
    }
}
