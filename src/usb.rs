//! USB device driver for HT32F523xx.
//!
//! The HT32 controller has one bidirectional control endpoint, seven
//! direction-fixed data endpoints, and 1 KiB of endpoint SRAM. Endpoint CSR
//! status bits use toggle-on-write-one semantics; treating them as ordinary
//! read/write bits prevents NAK and STALL transitions from taking effect.

use core::cell::UnsafeCell;
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
const DATA_EP_BASE: usize = EP0_TX_OFFSET + 2 * MAX_PACKET_SIZE as usize;
const _: () = assert!(DATA_EP_BASE + (EP_COUNT - 1) * MAX_PACKET_SIZE as usize <= USB_SRAM_SIZE);

// USB global register bits.
const CSR_FRES: u32 = 1 << 1;
const CSR_PDWN: u32 = 1 << 2;
const CSR_LPMODE: u32 = 1 << 3;
const CSR_DPPUEN: u32 = 1 << 10;

const IER_UGIE: u32 = 1 << 0;
const IER_URSTIE: u32 = 1 << 2;
const IER_RSMIE: u32 = 1 << 3;
const IER_SUSPIE: u32 = 1 << 4;
const IER_EP0IE: u32 = 1 << 8;

const ISR_SOFIF: u32 = 1 << 1;
const ISR_URSTIF: u32 = 1 << 2;
const ISR_RSMIF: u32 = 1 << 3;
const ISR_SUSPIF: u32 = 1 << 4;
const ISR_ENDPOINT_MASK: u32 = 0xff << 8;
const ISR_W1C_MASK: u32 = ISR_SOFIF | ISR_URSTIF | ISR_RSMIF | ISR_SUSPIF | ISR_ENDPOINT_MASK;
// Endpoint registers and fields.
const EP_REG_STRIDE: usize = 0x14;
const EP_CSR_OFFSET: usize = 0x00;
const EP_IER_OFFSET: usize = 0x04;
const EP_ISR_OFFSET: usize = 0x08;
const EP_TCR_OFFSET: usize = 0x0c;
const EP_CFGR_OFFSET: usize = 0x10;

const EP_CSR_DTGTX: u32 = 1 << 0;
const EP_CSR_NAKTX: u32 = 1 << 1;
const EP_CSR_STLTX: u32 = 1 << 2;
const EP_CSR_DTGRX: u32 = 1 << 3;
const EP_CSR_NAKRX: u32 = 1 << 4;
const EP_CSR_STLRX: u32 = 1 << 5;

const EP_INT_ODRX: u32 = 1 << 1;
const EP_INT_ODOV: u32 = 1 << 2;
const EP_INT_IDTX: u32 = 1 << 4;
const EP_INT_UER: u32 = 1 << 7;
const EP_INT_SDRX: u32 = 1 << 9;
const EP_INT_SDER: u32 = 1 << 10;
const EP_INT_ZLRX: u32 = 1 << 11;
const EP0_INTERRUPT_MASK: u32 = 0x0fff;
const DATA_EP_INTERRUPT_MASK: u32 = 0x00ff;

const EP_CFGR_EPEN: u32 = 1 << 31;
const EP_CFGR_EPTYPE: u32 = 1 << 29;
const EP_CFGR_EPDIR: u32 = 1 << 28;

static BUS_WAKER: AtomicWaker = AtomicWaker::new();
static EP_IN_WAKERS: [AtomicWaker; EP_COUNT] = [const { AtomicWaker::new() }; EP_COUNT];
static EP_OUT_WAKERS: [AtomicWaker; EP_COUNT] = [const { AtomicWaker::new() }; EP_COUNT];
static EP_ENABLED_WAKERS: [AtomicWaker; EP_COUNT] = [const { AtomicWaker::new() }; EP_COUNT];

static BUS_RESET: AtomicBool = AtomicBool::new(false);
static BUS_SUSPEND: AtomicBool = AtomicBool::new(false);
static BUS_RESUME: AtomicBool = AtomicBool::new(false);
static EP0_SETUP: AtomicBool = AtomicBool::new(false);
static EP_IN_COMPLETE: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];
static EP_OUT_READY: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];
static EP_ENABLED: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];
static EP_TRANSFER_ERROR: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];
static EP_BUFFER_OVERFLOW: [AtomicBool; EP_COUNT] = [const { AtomicBool::new(false) }; EP_COUNT];
static USB_TAKEN: AtomicBool = AtomicBool::new(false);

struct SetupPacket(UnsafeCell<[u8; 8]>);

// Access is serialized by `critical_section`; the USB ISR is the sole writer.
unsafe impl Sync for SetupPacket {}

static SETUP_PACKET: SetupPacket = SetupPacket(UnsafeCell::new([0; 8]));

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
    pub(crate) fn new() -> Self {
        critical_section::with(|_| {
            assert!(
                !USB_TAKEN.load(Ordering::Acquire),
                "USB peripheral token has already been taken"
            );
            USB_TAKEN.store(true, Ordering::Release);
        });
        Self { _private: () }
    }
}

/// Driver configuration.
#[derive(Default)]
pub struct Config {
    /// The current controller does not expose a usable VBUS detector.
    pub vbus_detection: bool,
    /// Reserved for a future board-specific VBUS detector.
    pub enable_vbus_detect: bool,
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
        crate::gpio::configure_usb_pins();
        reset_software_state();
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
        ep_type: EndpointType,
    ) -> Result<EndpointAddress, EndpointAllocError> {
        // EP1..EP3 support only bulk/interrupt. Isochronous transfers are a
        // hardware capability of EP4..EP7.
        let first_usable = if ep_type == EndpointType::Isochronous {
            4
        } else {
            1
        };
        let addr = if let Some(addr) = requested {
            if addr.index() < first_usable
                || addr.index() >= EP_COUNT
                || addr.direction() != direction
            {
                return Err(EndpointAllocError);
            }
            addr
        } else {
            let Some(index) =
                (first_usable..EP_COUNT).find(|index| self.allocated_eps & (1 << index) == 0)
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
    control_max_packet_size: u16,
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
        validate_packet_size(ep_type, max_packet_size)?;
        let addr = self.allocate_address(ep_addr, Direction::In, ep_type)?;
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
        validate_packet_size(ep_type, max_packet_size)?;
        let addr = self.allocate_address(ep_addr, Direction::Out, ep_type)?;
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
        assert!(
            matches!(control_max_packet_size, 8 | 16 | 32 | 64),
            "USB control max packet size must be 8, 16, 32, or 64 bytes"
        );
        configure_control_endpoint(control_max_packet_size);
        (
            Bus {
                _phantom: PhantomData,
                power_detected: false,
                control_max_packet_size,
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
        info!("USB IN submit ep={} len={}", ep, data.len());
        set_ep_status(ep, EP_CSR_NAKTX, false);
        let result = wait_endpoint_transfer(ep, true).await;
        info!("USB IN complete ep={} ok={}", ep, result.is_ok());
        result
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

        // CLEAR_FEATURE(ENDPOINT_HALT) may already have armed OUT. Preserve a
        // packet that arrived between clearing STALL and starting this read.
        if !EP_OUT_READY[ep].load(Ordering::Acquire) {
            set_ep_status(ep, EP_CSR_NAKRX, false);
        }
        wait_endpoint_transfer(ep, false).await?;

        let len = endpoint_received_len(ep);
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
        let packet = read_setup_packet();
        info!(
            "USB setup bm={=u8:02x} req={=u8:02x} value={=u16:04x} index={=u16:04x} len={=u16}",
            packet[0],
            packet[1],
            u16::from_le_bytes([packet[2], packet[3]]),
            u16::from_le_bytes([packet[4], packet[5]]),
            u16::from_le_bytes([packet[6], packet[7]])
        );
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
        read_usb_sram(ep0_rx_offset(self.max_packet_size), &mut data[..len]);
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
        // HT32 latches DEVAR for the status stage (USB_EARLY_SET_ADDRESS).
        // Program it before transmitting the zero-length status packet.
        set_device_address(addr);
        self.accept().await;
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
                reset_hardware(self.control_max_packet_size);
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
        let ep = addr.index();
        let (stall_bit, toggle_bit) = if addr.is_in() {
            (EP_CSR_STLTX, EP_CSR_DTGTX)
        } else {
            (EP_CSR_STLRX, EP_CSR_DTGRX)
        };
        set_ep_status(ep, stall_bit, stalled);
        if !stalled {
            // CLEAR_FEATURE(ENDPOINT_HALT) restarts the data toggle at DATA0.
            set_ep_status(ep, toggle_bit, false);
            if addr.is_out() {
                EP_OUT_READY[ep].store(false, Ordering::Release);
                set_ep_status(ep, EP_CSR_NAKRX, false);
            }
        }
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
        let usb = usb_regs();
        unsafe {
            usb.ier().modify(|r, w| {
                let bit = IER_EP0IE << ep;
                w.bits(if enabled {
                    r.bits() | bit
                } else {
                    r.bits() & !bit
                })
            });
        }
        EP_ENABLED[ep].store(enabled, Ordering::Release);
        EP_IN_COMPLETE[ep].store(false, Ordering::Release);
        EP_OUT_READY[ep].store(false, Ordering::Release);
        EP_TRANSFER_ERROR[ep].store(false, Ordering::Release);
        EP_BUFFER_OVERFLOW[ep].store(false, Ordering::Release);
        set_ep_status(ep, EP_CSR_DTGTX, false);
        set_ep_status(ep, EP_CSR_DTGRX, false);
        if enabled {
            set_ep_status(ep, EP_CSR_NAKTX, true);
            set_ep_status(ep, EP_CSR_NAKRX, true);
        }
        EP_ENABLED_WAKERS[ep].wake();
        EP_IN_WAKERS[ep].wake();
        EP_OUT_WAKERS[ep].wake();
        info!("USB endpoint {=u8:02x} enabled={}", u8::from(addr), enabled);
    }

    async fn enable(&mut self) {
        let usb = usb_regs();
        unsafe {
            usb.ier()
                .write(|w| w.bits(IER_UGIE | IER_URSTIE | IER_RSMIE | IER_SUSPIE | IER_EP0IE));
            cortex_m::peripheral::NVIC::unpend(pac::Interrupt::USB);
            cortex_m::peripheral::NVIC::unmask(pac::Interrupt::USB);
        }
        usb.csr().modify(|_, w| w.dppuen().set_bit());
        info!("USB connected");
    }

    async fn disable(&mut self) {
        let usb = usb_regs();
        usb.csr().modify(|_, w| w.dppuen().clear_bit());
        unsafe { usb.ier().write(|w| w.bits(0)) };
        cortex_m::peripheral::NVIC::mask(pac::Interrupt::USB);
        self.power_detected = false;
        for ep in 1..EP_COUNT {
            modify_ep_reg(ep, EP_CFGR_OFFSET, |value| value & !EP_CFGR_EPEN);
            EP_ENABLED[ep].store(false, Ordering::Release);
            EP_IN_COMPLETE[ep].store(false, Ordering::Release);
            EP_OUT_READY[ep].store(false, Ordering::Release);
            EP_TRANSFER_ERROR[ep].store(false, Ordering::Release);
            EP_BUFFER_OVERFLOW[ep].store(false, Ordering::Release);
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

fn validate_packet_size(
    ep_type: EndpointType,
    max_packet_size: u16,
) -> Result<(), EndpointAllocError> {
    let supported = match ep_type {
        EndpointType::Control => false,
        EndpointType::Bulk => matches!(max_packet_size, 8 | 16 | 32 | 64),
        EndpointType::Interrupt | EndpointType::Isochronous => {
            max_packet_size != 0 && max_packet_size <= MAX_PACKET_SIZE
        }
    };
    supported.then_some(()).ok_or(EndpointAllocError)
}

fn reset_software_state() {
    BUS_RESET.store(false, Ordering::Release);
    BUS_SUSPEND.store(false, Ordering::Release);
    BUS_RESUME.store(false, Ordering::Release);
    EP0_SETUP.store(false, Ordering::Release);
    for ep in 0..EP_COUNT {
        EP_IN_COMPLETE[ep].store(false, Ordering::Release);
        EP_OUT_READY[ep].store(false, Ordering::Release);
        EP_ENABLED[ep].store(ep == 0, Ordering::Release);
        EP_TRANSFER_ERROR[ep].store(false, Ordering::Release);
        EP_BUFFER_OVERFLOW[ep].store(false, Ordering::Release);
    }
}

fn initialize_hardware() {
    let usb = usb_regs();

    // Reset the controller while the PHY is powered down, then explicitly
    // leave reset and power-down. DPPUEN remains clear until Bus::enable().
    // Keeping PDWN/LPMODE set here leaves the PHY disconnected according to
    // USBCSR, even if the pull-up is enabled later.
    unsafe {
        usb.csr().write(|w| w.bits(CSR_FRES | CSR_PDWN));
        usb.csr().write(|w| w.bits(0));
        // Clear only documented W1C flags. ESOFIF has different write
        // semantics and is neither enabled nor consumed by this driver.
        usb.isr().write(|w| w.bits(ISR_W1C_MASK));
    }

    configure_control_endpoint(MAX_PACKET_SIZE);
    unsafe {
        usb.ier()
            .write(|w| w.bits(IER_UGIE | IER_URSTIE | IER_RSMIE | IER_SUSPIE | IER_EP0IE));
        cortex_m::peripheral::NVIC::unpend(pac::Interrupt::USB);
        cortex_m::peripheral::NVIC::unmask(pac::Interrupt::USB);
    }
}

fn reset_hardware(control_max_packet_size: u16) {
    let usb = usb_regs();
    let pull_up = usb.csr().read().bits() & CSR_DPPUEN;

    unsafe {
        usb.csr().write(|w| w.bits(pull_up));
        usb.devar().write(|w| w.bits(0));
    }
    EP0_SETUP.store(false, Ordering::Release);
    for ep in 0..EP_COUNT {
        EP_IN_COMPLETE[ep].store(false, Ordering::Release);
        EP_OUT_READY[ep].store(false, Ordering::Release);
        EP_TRANSFER_ERROR[ep].store(false, Ordering::Release);
        EP_BUFFER_OVERFLOW[ep].store(false, Ordering::Release);
        if ep != 0 {
            EP_ENABLED[ep].store(false, Ordering::Release);
            modify_ep_reg(ep, EP_CFGR_OFFSET, |value| value & !EP_CFGR_EPEN);
            EP_ENABLED_WAKERS[ep].wake();
        }
        EP_IN_WAKERS[ep].wake();
        EP_OUT_WAKERS[ep].wake();
    }

    configure_control_endpoint(control_max_packet_size);
    unsafe {
        usb.ier()
            .write(|w| w.bits(IER_UGIE | IER_URSTIE | IER_RSMIE | IER_SUSPIE | IER_EP0IE));
    }
}

fn configure_control_endpoint(max_packet_size: u16) {
    // The SETUP area is fixed at SRAM offset zero. Hardware TX begins at
    // EPBUFA=8 and RX begins at EPBUFA+EPLEN.
    let cfgr = EP_CFGR_EPEN | ((max_packet_size as u32) << 10) | EP0_TX_OFFSET as u32;
    write_ep_reg(0, EP_CFGR_OFFSET, cfgr);
    write_ep_reg(
        0,
        EP_IER_OFFSET,
        EP_INT_SDRX
            | EP_INT_IDTX
            | EP_INT_ODRX
            | EP_INT_ODOV
            | EP_INT_UER
            | EP_INT_SDER
            | EP_INT_ZLRX,
    );
    write_ep_reg(0, EP_ISR_OFFSET, EP0_INTERRUPT_MASK);
    set_ep_status(0, EP_CSR_NAKTX, true);
    set_ep_status(0, EP_CSR_NAKRX, true);
    EP_ENABLED[0].store(true, Ordering::Release);
}

fn configure_data_endpoint(addr: EndpointAddress, ep_type: EndpointType, max_packet_size: u16) {
    let ep = addr.index();
    let allocation_size = align4(max_packet_size);
    let mut cfgr = (endpoint_buffer_offset(ep) as u32)
        | ((allocation_size as u32) << 10)
        | ((ep as u32) << 24);
    if addr.is_in() {
        cfgr |= EP_CFGR_EPDIR;
    }
    if ep_type == EndpointType::Isochronous {
        cfgr |= EP_CFGR_EPTYPE;
    }

    write_ep_reg(ep, EP_CFGR_OFFSET, cfgr);
    write_ep_reg(
        ep,
        EP_IER_OFFSET,
        EP_INT_ODRX | EP_INT_IDTX | EP_INT_ODOV | EP_INT_UER,
    );
    write_ep_reg(ep, EP_ISR_OFFSET, DATA_EP_INTERRUPT_MASK);
    set_ep_status(ep, EP_CSR_NAKTX, true);
    set_ep_status(ep, EP_CSR_NAKRX, true);
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
        } else if take_flag(&EP_BUFFER_OVERFLOW[ep]) {
            Poll::Ready(Err(EndpointError::BufferOverflow))
        } else if take_flag(&EP_TRANSFER_ERROR[ep]) {
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
        } else if take_flag(&EP_BUFFER_OVERFLOW[0]) {
            Poll::Ready(Err(EndpointError::BufferOverflow))
        } else if take_flag(&EP_TRANSFER_ERROR[0]) {
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

const fn ep0_rx_offset(max_packet_size: u16) -> usize {
    EP0_TX_OFFSET + max_packet_size as usize
}

const fn align4(value: u16) -> u16 {
    (value + 3) & !3
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

fn capture_setup_packet() {
    let mut packet = [0; 8];
    read_usb_sram(EP0_SETUP_OFFSET, &mut packet);
    critical_section::with(|_| unsafe {
        *SETUP_PACKET.0.get() = packet;
    });
}

fn read_setup_packet() -> [u8; 8] {
    critical_section::with(|_| unsafe { *SETUP_PACKET.0.get() })
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

fn endpoint_interrupt_mask(ep: usize) -> u32 {
    if ep == 0 {
        EP0_INTERRUPT_MASK
    } else {
        DATA_EP_INTERRUPT_MASK
    }
}

fn endpoint_received_len(ep: usize) -> usize {
    let count = read_ep_reg(ep, EP_TCR_OFFSET);
    if ep <= 3 {
        // EP1..EP3 expose TCNT[8:0].
        (count & 0x01ff) as usize
    } else {
        // EP4..EP7 are configured single-buffered, so TCNT0[9:0] is used.
        (count & 0x03ff) as usize
    }
}

fn set_low_power(enabled: bool) {
    let usb = usb_regs();
    unsafe {
        usb.csr().modify(|r, w| {
            let low_power = CSR_LPMODE | CSR_PDWN;
            let value = if enabled {
                r.bits() | low_power
            } else {
                r.bits() & !low_power
            };
            w.bits(value)
        });
    }
}

fn handle_endpoint_interrupt(ep: usize) {
    let raw_flags = read_ep_reg(ep, EP_ISR_OFFSET) & endpoint_interrupt_mask(ep);
    let flags = raw_flags & read_ep_reg(ep, EP_IER_OFFSET);

    if ep == 0 && flags & EP_INT_SDRX != 0 && flags & EP_INT_SDER == 0 {
        // Snapshot SETUP before acknowledging SDRX. This prevents a later
        // packet from producing a mixed eight-byte request in task context.
        capture_setup_packet();
    }

    // Endpoint interrupt flags are W1C and must be cleared before the global
    // EPn flag. Clear even disabled/unexpected flags to avoid an IRQ storm.
    if raw_flags != 0 {
        write_ep_reg(ep, EP_ISR_OFFSET, raw_flags);
    }
    unsafe {
        usb_regs().isr().write(|w| w.bits(IER_EP0IE << ep));
    }

    if ep == 0 && flags & EP_INT_SDER != 0 {
        EP0_SETUP.store(false, Ordering::Release);
        EP_TRANSFER_ERROR[0].store(true, Ordering::Release);
        EP_OUT_WAKERS[0].wake();
        EP_IN_WAKERS[0].wake();
        return;
    }
    if ep == 0 && flags & EP_INT_SDRX != 0 {
        EP_TRANSFER_ERROR[0].store(false, Ordering::Release);
        EP_BUFFER_OVERFLOW[0].store(false, Ordering::Release);
        EP0_SETUP.store(true, Ordering::Release);
        EP_IN_COMPLETE[0].store(false, Ordering::Release);
        EP_OUT_READY[0].store(false, Ordering::Release);
        EP_OUT_WAKERS[0].wake();
        EP_IN_WAKERS[0].wake();
        // SDRX invalidates completion flags from the interrupted control
        // transfer, even if hardware reported them in the same IRQ snapshot.
        return;
    }
    if flags & EP_INT_ODRX != 0 || (ep == 0 && flags & EP_INT_ZLRX != 0) {
        EP_OUT_READY[ep].store(true, Ordering::Release);
        EP_OUT_WAKERS[ep].wake();
    }
    if flags & EP_INT_IDTX != 0 {
        info!("USB IRQ IDTX ep={} flags={=u32:08x}", ep, flags);
        EP_IN_COMPLETE[ep].store(true, Ordering::Release);
        EP_IN_WAKERS[ep].wake();
    }
    if flags & EP_INT_ODOV != 0 {
        EP_BUFFER_OVERFLOW[ep].store(true, Ordering::Release);
        EP_OUT_WAKERS[ep].wake();
    }
    if flags & (EP_INT_UER | EP_INT_SDER) != 0 {
        EP_TRANSFER_ERROR[ep].store(true, Ordering::Release);
        EP_IN_WAKERS[ep].wake();
        EP_OUT_WAKERS[ep].wake();
    }
}

/// Interrupt handler type for applications that use a custom binding layer.
pub struct InterruptHandler;

impl InterruptHandler {
    /// Run the USB interrupt service routine.
    ///
    /// # Safety
    /// Must only be called from the USB interrupt context.
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
    let endpoint_pending = (pending >> 8) & 0xff;

    if pending & ISR_URSTIF != 0 {
        set_low_power(false);
        BUS_SUSPEND.store(false, Ordering::Release);
        BUS_RESUME.store(false, Ordering::Release);
        BUS_RESET.store(true, Ordering::Release);
        unsafe { usb.isr().write(|w| w.bits(ISR_URSTIF)) };
        // A reset invalidates all endpoint completions in this IRQ snapshot.
        // Acknowledge them without publishing stale transfer results.
        for ep in 0..EP_COUNT {
            if endpoint_pending & (1 << ep) != 0 {
                let raw_flags = read_ep_reg(ep, EP_ISR_OFFSET) & endpoint_interrupt_mask(ep);
                if raw_flags != 0 {
                    write_ep_reg(ep, EP_ISR_OFFSET, raw_flags);
                }
                unsafe { usb.isr().write(|w| w.bits(IER_EP0IE << ep)) };
            }
        }
        BUS_WAKER.wake();
        return;
    }
    if pending & ISR_SUSPIF != 0 {
        set_low_power(true);
        BUS_RESUME.store(false, Ordering::Release);
        BUS_SUSPEND.store(true, Ordering::Release);
        unsafe { usb.isr().write(|w| w.bits(ISR_SUSPIF)) };
        BUS_WAKER.wake();
    }
    if pending & ISR_RSMIF != 0 {
        set_low_power(false);
        BUS_SUSPEND.store(false, Ordering::Release);
        BUS_RESUME.store(true, Ordering::Release);
        unsafe { usb.isr().write(|w| w.bits(ISR_RSMIF)) };
        BUS_WAKER.wake();
    }

    for ep in 0..EP_COUNT {
        if endpoint_pending & (1 << ep) != 0 {
            handle_endpoint_interrupt(ep);
        }
    }
}
