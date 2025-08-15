use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use embassy_sync::waitqueue::AtomicWaker;
use embassy_usb_driver::{
    Direction, EndpointAddress, EndpointAllocError, EndpointError, EndpointInfo, EndpointType,
    Event, Unsupported,
};

const MAX_EP_COUNT: usize = 8;
const MAX_PACKET_SIZE: usize = 64;

pub struct Driver<'d> {
    phantom: PhantomData<&'d ()>,
    alloc_in: AtomicBool,
    alloc_out: AtomicBool,
}

impl<'d> Driver<'d> {
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
            alloc_in: AtomicBool::new(false),
            alloc_out: AtomicBool::new(false),
        }
    }
}

pub struct Bus<'d> {
    phantom: PhantomData<&'d ()>,
    ep_types: [Option<EndpointType>; MAX_EP_COUNT],
    ep_in_wakers: [AtomicWaker; MAX_EP_COUNT],
    ep_out_wakers: [AtomicWaker; MAX_EP_COUNT],
    bus_waker: AtomicWaker,
}

impl<'d> Bus<'d> {
    fn new() -> Self {
        const NEW_AW: AtomicWaker = AtomicWaker::new();
        Self {
            phantom: PhantomData,
            ep_types: [None; MAX_EP_COUNT],
            ep_in_wakers: [NEW_AW; MAX_EP_COUNT],
            ep_out_wakers: [NEW_AW; MAX_EP_COUNT], 
            bus_waker: AtomicWaker::new(),
        }
    }
}

pub struct ControlPipe<'d> {
    _phantom: PhantomData<&'d ()>,
}

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
        _interval: u8,
    ) -> Result<Self::EndpointIn, EndpointAllocError> {
        if self.alloc_in.load(Ordering::Relaxed) {
            return Err(EndpointAllocError);
        }
        self.alloc_in.store(true, Ordering::Relaxed);

        let addr = ep_addr.unwrap_or(EndpointAddress::from_parts(1, Direction::In));
        
        Ok(Endpoint {
            _phantom: PhantomData,
            info: EndpointInfo {
                addr,
                ep_type,
                max_packet_size: max_packet_size.min(MAX_PACKET_SIZE as u16),
                interval_ms: _interval,
            },
            _direction: PhantomData,
        })
    }

    fn alloc_endpoint_out(
        &mut self,
        ep_type: EndpointType,
        ep_addr: Option<EndpointAddress>,
        max_packet_size: u16,
        _interval: u8,
    ) -> Result<Self::EndpointOut, EndpointAllocError> {
        if self.alloc_out.load(Ordering::Relaxed) {
            return Err(EndpointAllocError);
        }
        self.alloc_out.store(true, Ordering::Relaxed);

        let addr = ep_addr.unwrap_or(EndpointAddress::from_parts(1, Direction::Out));
        
        Ok(Endpoint {
            _phantom: PhantomData,
            info: EndpointInfo {
                addr,
                ep_type,
                max_packet_size: max_packet_size.min(MAX_PACKET_SIZE as u16),
                interval_ms: _interval,
            },
            _direction: PhantomData,
        })
    }

    fn start(self, _control_max_packet_size: u16) -> (Self::Bus, Self::ControlPipe) {
        let bus = Bus::new();
        let ep = ControlPipe {
            _phantom: PhantomData,
        };

        (bus, ep)
    }
}

pub struct Endpoint<'d, D> {
    _phantom: PhantomData<&'d ()>,
    info: EndpointInfo,
    _direction: PhantomData<D>,
}

pub struct In;
pub struct Out;

impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, In> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
    }
}

impl<'d> embassy_usb_driver::Endpoint for Endpoint<'d, Out> {
    fn info(&self) -> &EndpointInfo {
        &self.info
    }

    async fn wait_enabled(&mut self) {
    }
}

impl<'d> embassy_usb_driver::EndpointOut for Endpoint<'d, Out> {
    async fn read(&mut self, buf: &mut [u8]) -> Result<usize, EndpointError> {
        if buf.len() > MAX_PACKET_SIZE {
            return Err(EndpointError::BufferOverflow);
        }
        Ok(0)
    }
}

impl<'d> embassy_usb_driver::EndpointIn for Endpoint<'d, In> {
    async fn write(&mut self, _buf: &[u8]) -> Result<(), EndpointError> {
        Ok(())
    }
}

impl<'d> embassy_usb_driver::ControlPipe for ControlPipe<'d> {
    fn max_packet_size(&self) -> usize {
        64
    }

    async fn setup(&mut self) -> [u8; 8] {
        [0; 8]
    }

    async fn data_out(&mut self, buf: &mut [u8], _first: bool, _last: bool) -> Result<usize, EndpointError> {
        Ok(buf.len().min(64))
    }

    async fn data_in(&mut self, _data: &[u8], _first: bool, _last: bool) -> Result<(), EndpointError> {
        Ok(())
    }

    async fn accept(&mut self) {
    }

    async fn reject(&mut self) {
    }

    async fn accept_set_address(&mut self, _addr: u8) {
    }
}

impl<'d> embassy_usb_driver::Bus for Bus<'d> {
    async fn poll(&mut self) -> Event {
        Event::PowerDetected
    }

    fn endpoint_set_stalled(&mut self, _ep_addr: EndpointAddress, _stalled: bool) {
    }

    fn endpoint_is_stalled(&mut self, _ep_addr: EndpointAddress) -> bool {
        false
    }

    fn endpoint_set_enabled(&mut self, _ep_addr: EndpointAddress, _enabled: bool) {
    }

    async fn enable(&mut self) {
    }

    async fn disable(&mut self) {
    }

    async fn remote_wakeup(&mut self) -> Result<(), Unsupported> {
        Err(Unsupported)
    }
}

static mut EP_MEMORY: [u8; 1024] = [0; 1024];

pub struct Config {
    pub vbus_detection: bool,
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