use core::{
    alloc::{Allocator, Layout},
    borrow::BorrowMut,
    mem::MaybeUninit,
    time::Duration,
};

use aarch64_cpu::asm::barrier::{self, SY};
use alloc::{borrow::ToOwned, boxed::Box, sync::Arc, vec, vec::Vec};
use axalloc::{global_no_cache_allocator, GlobalNoCacheAllocator};
use axhal::time::busy_wait;
use axtask::sleep;
use bit_field::BitField;
use log::{debug, error};
use num_derive::FromPrimitive;
use num_traits::{FromPrimitive, ToPrimitive};
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::{
    context::{
        Device, Device64Byte, DeviceHandler, EndpointHandler, EndpointState, EndpointType,
        Input64Byte, InputHandler, Slot, SlotHandler,
    },
    extended_capabilities::debug::ContextPointer,
    ring::trb::{
        command::{self, ConfigureEndpoint, EvaluateContext},
        event::{CommandCompletion, CompletionCode, TransferEvent},
        transfer::{self, Allowed, DataStage, Direction, SetupStage, StatusStage, TransferType},
    },
};

use crate::host::structures::{
    descriptor::{self, RawDescriptorParser},
    reset_port,
    transfer_ring::TransferRing,
    xhci_event_manager::{self, EventManager, EVENT_MANAGER},
    PortLinkState,
};

use super::{
    descriptor::{Descriptor, Endpoint},
    dump_port_status, registers,
    xhci_command_manager::{CommandResult, COMMAND_MANAGER},
    xhci_slot_manager::SLOT_MANAGER,
};

struct TransferableEndpoint {
    endpoint: Endpoint,
    transfer: Box<TransferRing, GlobalNoCacheAllocator>,
}

pub struct XHCIUSBDevice {
    pub input: PageBox<Input64Byte>,
    pub output: PageBox<Device64Byte>,
    pub transfer_ring_control: Box<TransferRing, GlobalNoCacheAllocator>,
    pub non_ep0_endpoints: Vec<TransferableEndpoint>,
    pub device_desc: Option<descriptor::Device>,
    pub slot_id: u8,
    pub port_id: u8,
}

impl XHCIUSBDevice {
    pub fn new(port_id: u8) -> Result<Self, ()> {
        debug!("new device! port:{}", port_id);

        Ok({
            let xhciusbdevice: _ = Self {
                transfer_ring_control: Box::new_in(
                    TransferRing::new(),
                    global_no_cache_allocator(),
                ),
                port_id,
                slot_id: 0,
                input: PageBox::alloc_4k_zeroed_page_for_single_item(),
                output: PageBox::alloc_4k_zeroed_page_for_single_item(),
                non_ep0_endpoints: Vec::new(),
                device_desc: None,
            };

            xhciusbdevice
        })
    }

    pub fn initialize(&mut self) {
        debug!("initialize/enum this device! port={}", self.port_id);

        self.enable_slot();
        self.slot_ctx_init();
        self.config_endpoint_0();
        self.assign_device();
        self.address_device(false);
        let max_packet_size = self.get_max_packet_size();
        debug!("get max packet size: {}", max_packet_size);

        self.set_endpoint_speed(max_packet_size);
        self.evaluate_context_enable_ep0();
        self.fetch_dev_desc();
    }

    pub fn configure(&mut self) {
        let fetch_config_desc = self.fetch_config_desc();
        self.desc_to_endpoints(fetch_config_desc);
        self.input.device_mut().slot_mut().set_context_entries(31);
        self.configure_endpoints()
    }

    fn configure_endpoints(&mut self) {
        COMMAND_MANAGER.get().unwrap().lock().config_endpoint(
            self.slot_id,
            self.input.virt_addr(),
            false,
        );

        xhci_event_manager::handle_event();

        debug!("configure endpoint complete")
    }

    fn desc_to_endpoints(&mut self, descriptors: Vec<Descriptor>) {
        let collect = descriptors
            .iter()
            .filter_map(|desc| {
                if let Descriptor::Endpoint(e) = desc {
                    // let d = DoorbellWriter::new(f.slot_number(), e.doorbell_value());
                    // let s = transfer::Sender::new(d);
                    // Some(endpoint::NonDefault::new(*e, s))
                    Some({
                        let mut transferable_endpoint = TransferableEndpoint {
                            endpoint: e.clone(),
                            transfer: Box::new_in(TransferRing::new(), global_no_cache_allocator()),
                        };
                        transferable_endpoint.init_context(self);
                        transferable_endpoint
                    })
                } else {
                    None
                }
            })
            .collect();

        self.non_ep0_endpoints = collect;
    }

    fn enable_slot(&mut self) {
        match COMMAND_MANAGER.get().unwrap().lock().enable_slot() {
            CommandResult::Success(succedd_trb) => {
                debug!("enable slot success! {:?}", succedd_trb);
                self.slot_id = succedd_trb.slot_id();
            }
            //需要让device分配在指定的内存空间中
            err => debug!("failed to enable slot"),
        }
    }
    fn fetch_dev_desc(&mut self) {
        let buffer = PageBox::new_slice(0u8, 4096);
        let (setup, data, status) = Self::construct_trbs_for_getting_descriptors(
            &buffer,
            DescTyIdx::new(descriptor::Type::Device, 0),
        );
        self.enque_trbs_to_transger(vec![setup, data, status], 1, self.slot_id);
        debug!("fetched!");
        let descriptors = RawDescriptorParser::new(buffer).parse();
        debug!("dev descriptors: {:?}", descriptors);
        match descriptors[0] {
            Descriptor::Device(dev) => self.device_desc = { Some(dev) },
            other => error!(
                "fetch device descriptor encounted some issue, fetch value: {:?}",
                other
            ),
        }
    }

    fn fetch_config_desc(&mut self) -> Vec<Descriptor> {
        let buffer = PageBox::new_slice(0u8, 4096);
        let (setup, data, status) = Self::construct_trbs_for_getting_descriptors(
            &buffer,
            DescTyIdx::new(descriptor::Type::Configuration, 0),
        );
        self.enque_trbs_to_transger(vec![setup, data, status], 1, self.slot_id);
        debug!("fetched!");
        let descriptors = RawDescriptorParser::new(buffer).parse();
        debug!("config descriptors: {:?}", descriptors);
        descriptors
    }

    fn construct_trbs_for_getting_descriptors<T: ?Sized>(
        b: &PageBox<T>,
        t: DescTyIdx,
    ) -> (transfer::Allowed, transfer::Allowed, transfer::Allowed) {
        let setup = *transfer::SetupStage::default()
            .set_request_type(0b1000_0000)
            .set_request(6u8) //get_desc
            .set_value(t.bits())
            .set_length(b.bytes().as_usize().try_into().unwrap())
            .set_transfer_type(TransferType::In);

        let data = *transfer::DataStage::default()
            .set_data_buffer_pointer(b.virt_addr().as_usize() as u64)
            .set_trb_transfer_length(b.bytes().as_usize().try_into().unwrap())
            .set_direction(Direction::In);

        let status = *transfer::StatusStage::default().set_interrupt_on_completion();

        (setup.into(), data.into(), status.into())
    }

    fn slot_ctx_init(&mut self) {
        debug!("init input ctx");
        self.dump_ep0();
        let input_control = self.input.control_mut();
        // input_control.set_drop_context_flag(0);
        input_control.set_add_context_flag(0);
        input_control.set_add_context_flag(1);

        let slot = self.input.device_mut().slot_mut();
        debug!("root port id: {}", self.port_id);
        slot.set_root_hub_port_number(self.port_id + 1);
        slot.set_speed(registers::handle(|r| {
            r.port_register_set
                .read_volatile_at((self.port_id).into())
                .portsc
                .port_speed()
        }));
        slot.set_route_string(0);
        slot.set_context_entries(1);
    }

    fn get_max_len(&mut self) -> u16 {
        let psi = registers::handle(|r| {
            r.port_register_set
                .read_volatile_at((self.port_id).into())
                .portsc
                .port_speed()
        });

        match psi {
            1 | 3 => 64,
            2 => 8,
            4 => 512,
            _ => {
                // unimplemented!("PSI: {}", psi)
                error!("unimpl PSI: {}", psi);
                8
            }
        }
    }

    fn config_endpoint_0(&mut self) {
        debug!("begin config endpoint 0 and assign dev!");

        let s = self.get_max_len();
        debug!("config ep0");
        self.dump_ep0();

        let endpoint_mut = self.input.device_mut().endpoint_mut(1);
        endpoint_mut.set_endpoint_type(EndpointType::Control);
        endpoint_mut.set_max_packet_size(s);
        endpoint_mut.set_max_burst_size(0);
        let transfer_addr = self.transfer_ring_control.get_ring_addr().as_usize() as u64;
        debug!("address of transfer ring: {:x}", transfer_addr);
        endpoint_mut.set_tr_dequeue_pointer(transfer_addr);
        if (self.transfer_ring_control.cycle_state() != 0) {
            endpoint_mut.set_dequeue_cycle_state();
        } else {
            endpoint_mut.clear_dequeue_cycle_state();
        }
        endpoint_mut.set_interval(0);
        endpoint_mut.set_max_primary_streams(0);
        endpoint_mut.set_mult(0);
        endpoint_mut.set_error_count(3);
    }

    fn dump_ep0(&mut self) {
        debug!(
            "endpoint 0 state: {:?}, slot state: {:?}",
            self.input.device_mut().endpoint(1).endpoint_state(),
            self.input.device_mut().slot().slot_state()
        )
    }

    pub fn assign_device(&mut self) {
        let virt_addr = self.output.virt_addr();
        debug!(
            "assigning device into dcbaa, slot number= {},output addr: {:x}",
            self.slot_id, virt_addr
        );
        SLOT_MANAGER
            .get()
            .unwrap()
            .lock()
            .assign_device(self.slot_id, virt_addr);

        barrier::dmb(SY);
    }

    fn address_device(&mut self, bsr: bool) {
        debug!("addressing device");
        let input_addr = self.input.virt_addr();
        debug!("address to input {:?}, check 64 alignment!", input_addr);
        assert!(input_addr.is_aligned(64usize), "input not aligned to 64!");
        match COMMAND_MANAGER
            .get()
            .unwrap()
            .lock()
            .address_device(input_addr, self.slot_id, bsr)
        {
            CommandResult::Success(trb) => {
                debug!("addressed device at slot id {}", self.slot_id);
                debug!("command result {:?}", trb);
            }
            err => error!("error while address device at slot id {}", self.slot_id),
        }

        debug!("assert ep0 running!");
        self.dump_ep0();
    }
    fn check_input(&mut self) {
        debug!("input addr: {:x}", self.input.virt_addr());
    }

    fn enqueue_trb_to_transfer(
        &mut self,
        trb: transfer::Allowed,
        endpoint_id: u8,
    ) -> Result<[u32; 4], ()> {
        self.transfer_ring_control.enqueue(trb);
        barrier::dmb(SY);

        self.dump_ep0();
        dump_port_status(self.port_id as usize);
        debug!("doorbell ing slot {} target {}", self.slot_id, endpoint_id);
        registers::handle(|r| {
            r.doorbell
                .update_volatile_at(self.slot_id as usize, |doorbell| {
                    doorbell.set_doorbell_target(endpoint_id); //assume 1
                })
        });

        while let handle_event = xhci_event_manager::handle_event() {
            if handle_event.is_ok() {
                debug!("interrupt handler complete! result = {:?}", handle_event);
                return handle_event;
            }
        }
        Err(())
    }

    fn enque_trbs_to_transger(
        &mut self,
        trbs: Vec<transfer::Allowed>,
        endpoint_id_dci: u8,
        slot_id: u8,
    ) -> Result<[u32; 4], ()> {
        let size = trbs.len();
        self.transfer_ring_control.enqueue_trbs(&trbs);
        barrier::dmb(SY);

        debug!("doorbell ing");
        registers::handle(|r| {
            r.doorbell.update_volatile_at(slot_id as usize, |doorbell| {
                doorbell.set_doorbell_target(endpoint_id_dci); //assume 1
            })
        });

        debug!("waiting for event");
        while let handle_event = xhci_event_manager::handle_event() {
            if handle_event.is_ok() {
                debug!("interrupt handler complete! result = {:?}", handle_event);
                return handle_event;
            }
        }
        Err(())
    }

    fn get_max_packet_size(&mut self) -> u16 {
        debug!("get descriptor!");
        self.dump_ep0();

        let buffer = PageBox::from(descriptor::Device::default());
        let mut has_data_stage = false;
        let get_output = &mut self.output;
        let endpoint_id_dci = 1; //TODO modify, calculate endpoint //Default ep0

        debug!("doorbell id: {}", endpoint_id_dci);
        let setup_stage = Allowed::SetupStage(
            *SetupStage::default()
                .set_transfer_type(TransferType::In)
                .clear_interrupt_on_completion()
                .set_request_type(0x80)
                .set_request(6)
                .set_value(0x0100)
                .set_index(0)
                .set_length(8),
        );

        let data_stage = Allowed::DataStage(
            *DataStage::default()
                .set_direction(Direction::In)
                .set_trb_transfer_length(8)
                .clear_interrupt_on_completion()
                .set_data_buffer_pointer(buffer.virt_addr().as_usize() as u64),
        );

        let status_stage =
            transfer::Allowed::StatusStage(*StatusStage::default().set_interrupt_on_completion());

        self.enque_trbs_to_transger(
            vec![setup_stage, data_stage, status_stage],
            endpoint_id_dci,
            self.slot_id,
        );
        debug!("getted! buffer:{:?}", *buffer);

        debug!("return!");
        buffer.max_packet_size()
    }

    fn set_endpoint_speed(&mut self, speed: u16) {
        let mut binding = &mut self.input;
        let ep_0 = binding.device_mut().endpoint_mut(1);

        ep_0.set_max_packet_size(speed);
    }

    fn evaluate_context_enable_ep0(&mut self) {
        debug!("eval ctx and enable ep0!");
        let input = &mut self.input;
        match COMMAND_MANAGER
            .get()
            .unwrap()
            .lock()
            .evaluate_context(self.slot_id, input.virt_addr())
        {
            CommandResult::Success(cmp) => {
                debug!("success! complete code: {:?}", cmp);
            }
            CommandResult::OtherButSuccess(but) => {
                debug!("success! but: {:?}", but);
            }
            other_error => error!("error! {:?}", other_error),
        }
    }
}

impl TransferableEndpoint {
    fn calculate_dci(&self) -> u8 {
        let a = self.endpoint.endpoint_address;
        2 * a.get_bits(0..=3) + a.get_bit(7) as u8
    }

    pub fn init_context(&mut self, dev: &mut XHCIUSBDevice) {
        self.set_add_context_flag(dev);
        self.set_interval(dev, dev.port_id);
        self.init_for_endpoint_type(dev);
    }

    fn init_for_endpoint_type(&mut self, dev: &mut XHCIUSBDevice) {
        let endpoint_type = self.endpoint.endpoint_type();
        self.endpoint_context(dev).set_endpoint_type(endpoint_type);

        // TODO: This initializes the context only for USB2. Branch if the version of a device is
        // USB3.
        match endpoint_type {
            EndpointType::Control => self.init_for_control(dev),
            EndpointType::BulkOut | EndpointType::BulkIn => self.init_for_bulk(dev),
            EndpointType::IsochOut
            | EndpointType::IsochIn
            | EndpointType::InterruptOut
            | EndpointType::InterruptIn => self.init_for_isoch_or_interrupt(dev),
            EndpointType::NotValid => unreachable!("Not Valid Endpoint should not exist."),
        }
    }

    fn init_for_isoch_or_interrupt(&mut self, dev: &mut XHCIUSBDevice) {
        let t = self.endpoint.endpoint_type();
        assert!(
            self.is_isoch_or_interrupt(),
            "Not the Isochronous or the Interrupt Endpoint."
        );

        let sz = self.endpoint.max_packet_size;
        let a = self.transfer.get_ring_addr();
        let c = self.endpoint_context(dev);

        c.set_max_packet_size(sz & 0x7ff);
        c.set_max_burst_size(((sz & 0x1800) >> 11).try_into().unwrap());
        c.set_mult(0);

        if let EndpointType::IsochOut | EndpointType::IsochIn = t {
            c.set_error_count(0);
        } else {
            c.set_error_count(3);
        }
        c.set_tr_dequeue_pointer(a.as_usize() as u64);
        c.set_dequeue_cycle_state();
    }

    fn is_isoch_or_interrupt(&self) -> bool {
        let t = self.endpoint.endpoint_type();
        [
            EndpointType::IsochOut,
            EndpointType::IsochIn,
            EndpointType::InterruptOut,
            EndpointType::InterruptIn,
        ]
        .contains(&t)
    }

    fn init_for_bulk(&mut self, dev: &mut XHCIUSBDevice) {
        assert!(self.is_bulk(), "Not the Bulk Endpoint.");

        let sz = self.endpoint.max_packet_size;
        let a = self.transfer.get_ring_addr();
        let c = self.endpoint_context(dev);

        c.set_max_packet_size(sz);
        c.set_max_burst_size(0);
        c.set_error_count(3);
        c.set_max_primary_streams(0);
        c.set_tr_dequeue_pointer(a.as_usize() as u64);
        c.set_dequeue_cycle_state();
    }

    fn is_bulk(&self) -> bool {
        let t = self.endpoint.endpoint_type();

        [EndpointType::BulkOut, EndpointType::BulkIn].contains(&t)
    }

    fn init_for_control(&mut self, dev: &mut XHCIUSBDevice) {
        assert_eq!(
            self.endpoint.endpoint_type(),
            EndpointType::Control,
            "Not the Control Endpoint."
        );

        let size = self.endpoint.max_packet_size;
        let a = self.transfer.get_ring_addr();
        let c = self.endpoint_context(dev);

        c.set_max_packet_size(size);
        c.set_error_count(3);
        c.set_tr_dequeue_pointer(a.as_usize() as u64);
        c.set_dequeue_cycle_state();
    }

    fn set_add_context_flag(&self, dev: &mut XHCIUSBDevice) {
        let dci: usize = self.calculate_dci().into();
        let c = dev.input.control_mut();

        c.set_add_context_flag(0);
        c.clear_add_context_flag(1); // See xHCI dev manual 4.6.6.
        c.set_add_context_flag(dci);
    }

    fn set_interval(&self, dev: &mut XHCIUSBDevice, port_number: u8) {
        let port_speed = PortSpeed::get(port_number);
        let endpoint_type = self.endpoint.endpoint_type();
        let interval = self.endpoint.interval;

        let i = if let PortSpeed::FullSpeed | PortSpeed::LowSpeed = port_speed {
            if let EndpointType::IsochOut | EndpointType::IsochIn = endpoint_type {
                interval + 2
            } else {
                interval + 3
            }
        } else {
            interval - 1
        };

        self.endpoint_context(dev).set_interval(i);
    }

    fn endpoint_context<'a>(&self, dev: &'a mut XHCIUSBDevice) -> &'a mut dyn EndpointHandler {
        let ep_i: usize = self.endpoint.endpoint_address.get_bits(0..=3).into();
        let is_input: usize = self.endpoint.endpoint_address.get_bit(7) as _;
        let dpi = 2 * ep_i + is_input;

        dev.input.device_mut().endpoint_mut(dpi)
    }
}

pub(crate) struct DescTyIdx {
    ty: descriptor::Type,
    i: u8,
}
impl DescTyIdx {
    pub(crate) fn new(ty: descriptor::Type, i: u8) -> Self {
        Self { ty, i }
    }
    pub(crate) fn bits(self) -> u16 {
        (self.ty as u16) << 8 | u16::from(self.i)
    }
}

#[derive(Copy, Clone, FromPrimitive)]
enum PortSpeed {
    FullSpeed = 1,
    LowSpeed = 2,
    HighSpeed = 3,
    SuperSpeed = 4,
    SuperSpeedPlus = 5,
}

impl PortSpeed {
    pub fn get(port_number: u8) -> Self {
        FromPrimitive::from_u8(registers::handle(|r| {
            r.port_register_set
                .read_volatile_at((port_number).into())
                .portsc
                .port_speed()
        }))
        .unwrap()
    }
}
