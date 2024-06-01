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
use log::{debug, error};
use num_traits::ToPrimitive;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::{
    context::{
        Device, Device64Byte, DeviceHandler, EndpointState, EndpointType, Input64Byte,
        InputHandler, Slot, SlotHandler,
    },
    extended_capabilities::debug::ContextPointer,
    ring::trb::{
        command::{self, ConfigureEndpoint, EvaluateContext},
        event::{CommandCompletion, CompletionCode, TransferEvent},
        transfer::{self, Allowed, DataStage, Direction, SetupStage, StatusStage, TransferType},
    },
};

use crate::host::structures::{
    descriptor, reset_port, transfer_ring::TransferRing, xhci_event_manager, PortLinkState,
};

use super::{
    dump_port_status, registers,
    xhci_command_manager::{CommandResult, COMMAND_MANAGER},
    xhci_slot_manager::SLOT_MANAGER,
};

pub struct XHCIUSBDevice {
    input: PageBox<Input64Byte>,
    output: PageBox<Device64Byte>,
    transfer_ring: Box<TransferRing, GlobalNoCacheAllocator>,
    slot_id: u8,
    port_id: u8,
}

impl XHCIUSBDevice {
    pub fn new(port_id: u8) -> Result<Self, ()> {
        debug!("new device! port:{}", port_id);

        Ok({
            let xhciusbdevice: _ = Self {
                transfer_ring: Box::new_in(TransferRing::new(), global_no_cache_allocator()),
                port_id,
                slot_id: 0,
                input: PageBox::alloc_4k_zeroed_page_for_single_item(),
                output: PageBox::alloc_4k_zeroed_page_for_single_item(),
            };

            xhciusbdevice
        })
    }

    pub fn initialize(&mut self) {
        debug!("initialize/enum this device! port={}", self.port_id);

        // self.address_device(true);
        self.enable_slot();
        self.slot_ctx_init();
        self.config_endpoint_0();
        // self.check_input();
        self.assign_device();
        self.address_device(false);
        self.dump_ep0();
        dump_port_status(self.port_id as usize);
        // only available after address device
        // sleep(Duration::from_millis(100));
        // let get_descriptor = self.get_descriptor(); //damn, just assume speed is same lowest!
        // debug!("get desc: {:?}", get_descriptor);
        // dump_port_status(self.port_id as usize);
        // // self.check_endpoint();
        // // sleep(Duration::from_millis(2));

        // self.set_endpoint_speed(get_descriptor.max_packet_size()); //just let it be lowest speed!
        // self.evaluate_context_enable_ep0();
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

    fn slot_ctx_init(&mut self) {
        debug!("init input ctx");
        self.dump_ep0();
        let input_control = self.input.control_mut();
        // input_control.set_drop_context_flag(0);
        input_control.set_add_context_flag(0);
        input_control.set_add_context_flag(1);

        let slot = self.input.device_mut().slot_mut();
        debug!("root port id: {}", self.port_id);
        slot.set_root_hub_port_number(self.port_id);
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
        let transfer_addr = self.transfer_ring.get_ring_addr().as_usize() as u64;
        debug!("address of transfer ring: {:x}", transfer_addr);
        endpoint_mut.set_tr_dequeue_pointer(transfer_addr);
        if (self.transfer_ring.cycle_state() != 0) {
            endpoint_mut.set_dequeue_cycle_state();
        } else {
            endpoint_mut.clear_dequeue_cycle_state();
        }
        endpoint_mut.set_interval(0);
        endpoint_mut.set_max_primary_streams(0);
        endpoint_mut.set_mult(0);
        endpoint_mut.set_error_count(3);
        // ep_0.set_endpoint_state(EndpointState::Disabled);

        //confitional compile needed
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
        // debug!("input state: {:?}", self.input.dump_device_state());
    }

    fn enqueue_trb_to_transfer(
        &mut self,
        trb: transfer::Allowed,
        endpoint_id: u8,
    ) -> Result<[u32; 4], ()> {
        self.transfer_ring.enqueue(trb);
        barrier::dmb(SY);

        // self.optional_resume_port_state();

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
        self.transfer_ring.enqueue_trbs(&trbs);
        barrier::dmb(SY);

        debug!("doorbell ing");
        registers::handle(|r| {
            r.doorbell.update_volatile_at(slot_id as usize, |doorbell| {
                doorbell.set_doorbell_target(endpoint_id_dci - 1); //assume 1
            })
        });

        // let mut ret = Vec::with_capacity(size);
        // let mut mark = 0;
        // while let handle_event = xhci_event_manager::handle_event() {
        //     if handle_event.is_ok() {
        //         debug!(
        //             "interrupt handler complete! mark={mark} result = {:?}",
        //             handle_event
        //         );
        //         ret.push(handle_event.unwrap());
        //         mark += 1;
        //         if mark >= size {
        //             break;
        //         }
        //     }
        // }

        debug!("waiting for event");
        while let handle_event = xhci_event_manager::handle_event() {
            if handle_event.is_ok() {
                debug!("interrupt handler complete! result = {:?}", handle_event);
                return handle_event;
            }
        }
        Err(())
    }

    fn get_descriptor(&mut self) -> PageBox<super::descriptor::Device> {
        debug!("get descriptor!");
        self.dump_ep0();

        let buffer = PageBox::from(descriptor::Device::default());
        let mut has_data_stage = false;
        let get_output = &mut self.output;
        // debug!("device output ctx: {:?}", get_output); //目前卡在这里

        // let doorbell_id: u8 = {
        //     let endpoint = get_input.ep(1);
        //     let addr = endpoint.as_ref().as_ptr().addr();
        //     let endpoint_type = endpoint.endpoint_type();
        //     ((addr & 0x7f) * 2
        //         + match endpoint_type {
        //             EndpointType::BulkOut => 0,
        //             _ => 1,
        //         }) as u8
        // };
        let doorbell_id = 1;

        debug!("doorbell id: {}", doorbell_id);
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
            doorbell_id,
            self.slot_id,
        );
        debug!("getted! buffer:{:?}", buffer);

        debug!("return!");
        buffer
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
