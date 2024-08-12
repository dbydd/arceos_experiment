use crate::{dma::DMA, err::*, OsDep};
use alloc::{
    borrow::ToOwned,
    boxed::Box,
    format,
    rc::Rc,
    vec::{self, Vec},
};
use axalloc::global_no_cache_allocator;
use axhal::{cpu::this_cpu_is_bsp, irq::IrqHandler, paging::PageSize};
use core::{
    alloc::Allocator,
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    f64::consts::E,
    num::NonZeroUsize,
    ops::{Deref, DerefMut},
    sync::atomic::{fence, Ordering},
};
use core::{
    cell::{Ref, RefMut},
    iter::Cycle,
};
use log::*;
use num_traits::FromPrimitive;
use spinlock::SpinNoIrq;
use xhci::{
    context::{EndpointState, EndpointType, Input, InputHandler, Slot, Slot64Byte},
    extended_capabilities::debug::Status,
    registers::PortRegisterSet,
    ring::trb::{
        command::{
            self, AddressDevice, Allowed, ConfigureEndpoint, EnableSlot, EvaluateContext, Noop,
        },
        event::{CommandCompletion, CompletionCode, TransferEvent},
        transfer::{self, DataStage, SetupStage, StatusStage, TransferType},
    },
};
use xhci_device::{DescriptorConfiguration, DescriptorInterface, DeviceAttached};

pub use xhci::ring::trb::transfer::Direction;
mod registers;
use registers::*;
mod context;
mod event;
pub(crate) mod ring;
pub(crate) mod xhci_device;
use self::{context::*, event::EventRing, ring::Ring};
use super::{
    usb::{
        self,
        descriptors::{self, desc_device, Descriptor, DescriptorType, RawDescriptorParser},
    },
    Controller, USBHostConfig,
};
use crate::host::device::*;
use alloc::sync::Arc;
use core::mem;
const ARM_IRQ_PCIE_HOST_INTA: usize = 143 + 32;
const XHCI_CONFIG_MAX_EVENTS_PER_INTR: usize = 16;
const TAG: &str = "[XHCI]";

// pub static mut drivers: Option<Arc<SpinNoIrq<USBDeviceDriverHidMouseExample>>> = None;

pub struct Xhci<O>
where
    O: OsDep,
{
    pub(super) config: USBHostConfig<O>,
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
    scratchpad_buf_arr: Option<ScratchpadBufferArray<O>>,
    cmd: Ring<O>,
    event: EventRing<O>,
    regs: Registers,
    pub dev_ctx: DeviceContextList<O>,
}

impl<O> Controller<O> for Xhci<O>
where
    O: OsDep,
{
    fn new(config: USBHostConfig<O>) -> Result<Self>
    where
        Self: Sized,
    {
        let mmio_base = config.base_addr;
        debug!("{TAG} Base addr: {:?}", mmio_base);
        let mut regs = Registers::new_registers(mmio_base);

        // TODO: pcie 未配置，读不出来
        // let version = self.core_mut().regs.capability.hciversion.read_volatile();
        // info!("xhci version: {:x}", version.get());
        let hcsp1 = regs.regs.capability.hcsparams1.read_volatile();
        let max_slots = hcsp1.number_of_device_slots();
        let max_ports = hcsp1.number_of_ports();
        let max_irqs = hcsp1.number_of_interrupts();
        let page_size = regs.regs.operational.pagesize.read_volatile().get();
        debug!(
            "{TAG} Max_slots: {}, max_ports: {}, max_irqs: {}, page size: {}",
            max_slots, max_ports, max_irqs, page_size
        );

        let dev_ctx = DeviceContextList::new(max_slots, config.os.clone());

        // Create the command ring with 4096 / 16 (TRB size) entries, so that it uses all of the
        // DMA allocation (which is at least a 4k page).
        let entries_per_page = O::PAGE_SIZE / mem::size_of::<ring::TrbData>();
        let cmd = Ring::new(config.os.clone(), entries_per_page, true)?;
        let event = EventRing::new(config.os.clone())?;

        debug!("{TAG} ring size {}", cmd.len());

        let mut s = Self {
            config,
            max_slots,
            max_irqs,
            max_ports,
            scratchpad_buf_arr: None,
            cmd,
            event,
            regs,
            dev_ctx,
        };
        s.init()?;
        info!("{TAG} Init success");
        Ok(s)
    }

    fn poll(
        &mut self,
        arc: Arc<SpinNoIrq<Box<dyn Controller<O>>>>,
    ) -> Result<Vec<DeviceAttached<O>>> {
        let mut port_id_list = Vec::new();
        let port_len = self.regs().port_register_set.len();
        for i in 0..port_len {
            let portsc = &self.regs_mut().port_register_set.read_volatile_at(i).portsc;
            info!(
                "{TAG} Port {}: Enabled: {}, Connected: {}, Speed {}, Power {}",
                i,
                portsc.port_enabled_disabled(),
                portsc.current_connect_status(),
                portsc.port_speed(),
                portsc.port_power()
            );

            if !portsc.port_enabled_disabled() {
                continue;
            }

            port_id_list.push(i);
        }
        let mut device_list = Vec::new();
        for port_idx in port_id_list {
            let port_id = port_idx + 1;
            let slot = self.device_slot_assignment()?;
            let mut device = self.dev_ctx.new_slot(
                slot as usize,
                0,
                port_id,
                32,
                self.config.os.clone(),
                arc.clone(),
            )?;
            debug!("assign complete!");
            self.address_device(&device)?;

            self.print_context(&device);

            let packet_size0 = self.fetch_package_size0(&device)?;

            debug!("packet_size0: {}", packet_size0);

            self.set_ep0_packet_size(&device, packet_size0);
            let desc = self.fetch_device_desc(&device)?;
            let vid = desc.vendor;
            let pid = desc.product_id;

            info!("device found, pid: {pid:#X}, vid: {vid:#X}");

            device.device_desc = desc;

            for i in 0..device.device_desc.num_configurations {
                let config = self.fetch_config_desc(&device, i)?;
                debug!("{:#?}", config);
                device.configs.push(config)
            }

            self.set_configuration(&device, 0)?;

            device_list.push(device);
        }
        Ok(device_list)
    }

    fn post_cmd(&mut self, mut trb: command::Allowed) -> Result<CommandCompletion> {
        let addr = self.cmd.enque_command(trb);

        self.regs_mut().doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_stream_id(0);
            r.set_doorbell_target(0);
        });

        fence(Ordering::Release);

        let r = self.event_busy_wait_cmd(addr as _)?;

        /// update erdp
        self.regs_mut()
            .interrupter_register_set
            .interrupter_mut(0)
            .erdp
            .update_volatile(|f| {
                f.set_event_ring_dequeue_pointer(self.event.erdp());
            });

        Ok(r)
    }
    fn post_transfer(
        &mut self,
        setup: SetupStage,
        data: Option<DataStage>,
        status: StatusStage,
        device: &DeviceAttached<O>,
        dci: u8,
    ) -> Result {
        let mut trbs: Vec<transfer::Allowed> = Vec::new();
        trbs.push(setup.into());
        if let Some(data) = data {
            trbs.push(data.into());
        }
        trbs.push(status.into());
        let mut trb_pointers = Vec::new();
        {
            let ring = self.ep_ring_mut(device, dci);

            for trb in &mut trbs {
                if ring.cycle {
                    trb.set_cycle_bit();
                } else {
                    trb.clear_cycle_bit();
                }

                trb_pointers.push(ring.enque_trb(trb.into_raw()));
            }
        }
        if trb_pointers.len() == 2 {
            debug!(
                "[Transfer] >> setup@{:#X}, status@{:#X}",
                trb_pointers[0], trb_pointers[1]
            );
        } else {
            debug!(
                "[Transfer] >> setup@{:#X}, data@{:#X}, status@{:#X}",
                trb_pointers[0], trb_pointers[1], trb_pointers[2]
            );
        }

        fence(Ordering::Release);
        self.regs_mut()
            .doorbell
            .update_volatile_at(device.slot_id, |r| {
                r.set_doorbell_target(dci);
            });

        let r = self.event_busy_wait_transfer(*trb_pointers.last().unwrap() as _)?;
        Ok(())
    }

    fn post_transfer_normal(
        &mut self,
        data: &mut [u8],
        device: &DeviceAttached<O>,
        dci: u8,
    ) -> Result {
        let len = data.len();
        let mut buffer = DMA::new_vec(0u8, len, O::PAGE_SIZE, self.config.os.dma_alloc());
        buffer.copy_from_slice(data);
        let mut request = transfer::Normal::default();
        request
            .set_data_buffer_pointer(buffer.addr() as u64)
            .set_trb_transfer_length(len as _)
            .set_interrupt_on_completion();

        let ring = self.ep_ring_mut(device, dci);
        let addr = ring.enque_trb(transfer::Allowed::Normal(request).into_raw());

        fence(Ordering::Release);

        self.regs_mut()
            .doorbell
            .update_volatile_at(device.slot_id, |r| {
                r.set_doorbell_target(dci);
            });

        self.event_busy_wait_transfer(addr as _)?;

        data.copy_from_slice(&buffer);

        Ok(())
    }
}

impl<O> Xhci<O>
where
    O: OsDep,
{
    fn init(&mut self) -> Result {
        self.chip_hardware_reset()?;
        self.set_max_device_slots()?;
        self.set_dcbaap()?;
        self.set_cmd_ring()?;
        self.init_ir()?;

        self.setup_scratchpads();
        self.start()?;

        self.test_cmd()?;
        self.reset_ports();
        Ok(())
    }

    fn test_cmd(&mut self) -> Result {
        debug!("{TAG} Test command ring");
        for _ in 0..3 {
            let completion = self.post_cmd(Allowed::Noop(Noop::new()))?;
        }
        debug!("{TAG} Command ring ok");
        Ok(())
    }
    fn check_slot(&self, slot: u8) -> Result {
        if slot > self.max_slots {
            return Err(Error::Param(format!(
                "slot {} > max {}",
                slot, self.max_slots
            )));
        }
        Ok(())
    }

    fn print_context(&self, device: &DeviceAttached<O>) {
        let dev = &self.dev_ctx.device_out_context_list[device.slot_id];
        debug!("slot {} {:?}", device.slot_id, dev.slot().slot_state());
        for i in 1..32 {
            if let EndpointState::Disabled = dev.endpoint(i).endpoint_state() {
                continue;
            }
            debug!("  ep dci {}: {:?}", i, dev.endpoint(i).endpoint_state());
        }
    }
    fn device_slot_assignment(&mut self) -> Result<u8> {
        // enable slot
        let mut cmd = EnableSlot::new();
        let slot_type = {
            // TODO: PCI未初始化，读不出来
            // let mut regs = self.regs.lock();
            // match regs.supported_protocol(port) {
            //     Some(p) => p.header.read_volatile().protocol_slot_type(),
            //     None => {
            //         warn!(
            //             "{TAG} Failed to find supported protocol information for port {}",
            //             port
            //         );
            //         0
            //     }
            // }
            0
        };
        cmd.set_slot_type(slot_type);

        let cmd = Allowed::EnableSlot(EnableSlot::new());

        let result = self.post_cmd(cmd)?;

        let slot_id = result.slot_id();
        debug!("new slot id: {slot_id}");
        // self.dev_ctx.new_slot(slot_id as usize, 0, port_idx, 16).unwrap(); //assume 16
        Ok(slot_id)
    }

    fn append_port_to_route_string(route_string: u32, port_id: usize) -> u32 {
        let mut route_string = route_string;
        for tier in 0..5 {
            if route_string & (0x0f << (tier * 4)) == 0 {
                if tier < 5 {
                    route_string |= (port_id as u32) << (tier * 4);
                    return route_string;
                }
            }
        }

        route_string
    }

    pub fn address_device(&mut self, device: &DeviceAttached<O>) -> Result {
        let slot_id = device.slot_id;
        let port_idx = device.port_id - 1;
        let port_speed = self.get_speed(port_idx);
        let max_packet_size = self.get_default_max_packet_size(port_idx);
        let dci = 1;

        let transfer_ring_0_addr = self.ep_ring_mut(device, dci).register();
        let ring_cycle_bit = self.ep_ring_mut(device, dci).cycle;
        let context_addr = {
            let context_mut = self
                .dev_ctx
                .device_input_context_list
                .get_mut(slot_id)
                .unwrap()
                .deref_mut();

            let control_context = context_mut.control_mut();
            control_context.set_add_context_flag(0);
            control_context.set_add_context_flag(1);
            for i in 2..32 {
                control_context.clear_drop_context_flag(i);
            }

            let slot_context = context_mut.device_mut().slot_mut();
            slot_context.clear_multi_tt();
            slot_context.clear_hub();
            slot_context.set_route_string(Self::append_port_to_route_string(0, device.port_id)); // for now, not support more hub ,so hardcode as 0.//TODO: generate route string
            slot_context.set_context_entries(1);
            slot_context.set_max_exit_latency(0);
            slot_context.set_root_hub_port_number(device.port_id as _); //todo: to use port number
            slot_context.set_number_of_ports(0);
            slot_context.set_parent_hub_slot_id(0);
            slot_context.set_tt_think_time(0);
            slot_context.set_interrupter_target(0);
            slot_context.set_speed(port_speed);

            let endpoint_0 = context_mut.device_mut().endpoint_mut(dci as _);
            endpoint_0.set_endpoint_type(xhci::context::EndpointType::Control);
            endpoint_0.set_max_packet_size(max_packet_size);
            endpoint_0.set_max_burst_size(0);
            endpoint_0.set_error_count(3);
            endpoint_0.set_tr_dequeue_pointer(transfer_ring_0_addr);
            if ring_cycle_bit {
                endpoint_0.set_dequeue_cycle_state();
            } else {
                endpoint_0.clear_dequeue_cycle_state();
            }
            endpoint_0.set_interval(0);
            endpoint_0.set_max_primary_streams(0);
            endpoint_0.set_mult(0);
            endpoint_0.set_error_count(3);

            (context_mut as *const Input<16>).addr() as u64
        };

        fence(Ordering::Release);

        let result = self.post_cmd(Allowed::AddressDevice(
            *AddressDevice::new()
                .set_slot_id(slot_id as _)
                .set_input_context_pointer(context_addr),
        ))?;

        debug!("address slot [{}] ok", slot_id);

        Ok(())
    }
    fn setup_scratchpads(&mut self) {
        let scratchpad_buf_arr = {
            let buf_count = {
                let count = self
                    .regs()
                    .capability
                    .hcsparams2
                    .read_volatile()
                    .max_scratchpad_buffers();
                debug!("{TAG} Scratch buf count: {}", count);
                count
            };
            if buf_count == 0 {
                return;
            }
            let scratchpad_buf_arr = ScratchpadBufferArray::new(buf_count, self.config.os.clone());

            self.dev_ctx.dcbaa[0] = scratchpad_buf_arr.register() as u64;

            debug!(
                "{TAG} Setting up {} scratchpads, at {:#0x}",
                buf_count,
                scratchpad_buf_arr.register()
            );
            scratchpad_buf_arr
        };

        self.scratchpad_buf_arr = Some(scratchpad_buf_arr);
    }

    fn ep_ring_mut(&mut self, device: &DeviceAttached<O>, dci: u8) -> &mut Ring<O> {
        &mut self.dev_ctx.transfer_rings[device.slot_id][dci as usize - 1]
    }
    fn ep_ring(&self, device: &DeviceAttached<O>, dci: u8) -> &Ring<O> {
        &self.dev_ctx.transfer_rings[device.slot_id][dci as usize - 1]
    }
    fn control_transfer<T: ?Sized>(
        &mut self,
        dev: &DeviceAttached<O>,
        dci: u8,
        buffer: Option<&DMA<T, O::DMA>>,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        direction: Direction,
    ) -> Result {
        let transfer_type = if buffer.is_some() {
            match direction {
                Direction::Out => TransferType::Out,
                Direction::In => TransferType::In,
            }
        } else {
            TransferType::No
        };

        let mut len = 0;
        let data = if let Some(buffer) = buffer {
            let mut data = transfer::DataStage::default();
            len = buffer.length_for_bytes();
            data.set_data_buffer_pointer(buffer.addr() as u64)
                .set_trb_transfer_length(len as _)
                .set_direction(direction);
            Some(data)
        } else {
            None
        };
        let mut setup = transfer::SetupStage::default();
        setup
            .set_request_type(request_type)
            .set_request(request)
            .set_value(value)
            .set_index(index)
            .set_length(len as _)
            .set_transfer_type(transfer_type);

        debug!("{:#?}", setup);

        let mut status = transfer::StatusStage::default();

        status.set_interrupt_on_completion();

        self.post_transfer(setup, data, status, dev, dci)?;

        Ok(())
    }

    fn update_erdp(&mut self) {
        self.regs_mut()
            .interrupter_register_set
            .interrupter_mut(0)
            .erdp
            .update_volatile(|f| {
                f.set_event_ring_dequeue_pointer(self.event.erdp());
            });
    }

    fn event_busy_wait_transfer(&mut self, addr: u64) -> Result<TransferEvent> {
        debug!("Wait result @{addr:#X}");
        loop {
            if let Some((event, cycle)) = self.event.next() {
                self.update_erdp();

                match event {
                    xhci::ring::trb::event::Allowed::TransferEvent(c) => {
                        let code = c.completion_code().unwrap();
                        debug!(
                            "[Transfer] << {code:#?} @{:#X} got result, cycle {}, len {}",
                            c.trb_pointer(),
                            c.cycle_bit(),
                            c.trb_transfer_length()
                        );

                        if c.trb_pointer() != addr {
                            debug!("  @{:#X} != @{:#X}", c.trb_pointer(), addr);
                            // return Err(Error::Pip);
                            continue;
                        }
                        if CompletionCode::Success == code || CompletionCode::ShortPacket == code {
                            return Ok(c);
                        }
                        return Err(Error::CMD(code));
                    }
                    _ => warn!("event: {:?}", event),
                }
            }
        }
    }
    fn event_busy_wait_cmd(&mut self, addr: u64) -> Result<CommandCompletion> {
        debug!("Wait result");
        loop {
            if let Some((event, cycle)) = self.event.next() {
                match event {
                    xhci::ring::trb::event::Allowed::CommandCompletion(c) => {
                        let mut code = CompletionCode::Invalid;
                        if let Ok(c) = c.completion_code() {
                            code = c;
                        } else {
                            continue;
                        }
                        debug!(
                            "[CMD] << {code:#?} @{:X} got result, cycle {}",
                            c.command_trb_pointer(),
                            c.cycle_bit()
                        );
                        if c.command_trb_pointer() != addr {
                            continue;
                        }

                        if let CompletionCode::Success = code {
                            return Ok(c);
                        }
                        return Err(Error::CMD(code));
                    }
                    _ => warn!("event: {:?}", event),
                }
            }
        }
    }

    fn regs(&self) -> &RegistersBase {
        &self.regs.regs
    }

    fn regs_mut(&mut self) -> &mut RegistersBase {
        &mut self.regs.regs
    }

    fn chip_hardware_reset(&mut self) -> Result {
        debug!("{TAG} Reset begin");
        debug!("{TAG} Stop");

        self.regs_mut().operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });
        debug!("{TAG} Until halt");
        while !self.regs().operational.usbsts.read_volatile().hc_halted() {}
        debug!("{TAG} Halted");

        let mut o = &mut self.regs_mut().operational;
        // debug!("xhci stat: {:?}", o.usbsts.read_volatile());

        debug!("{TAG} Wait for ready...");
        while o.usbsts.read_volatile().controller_not_ready() {}
        debug!("{TAG} Ready");

        o.usbcmd.update_volatile(|f| {
            f.set_host_controller_reset();
        });

        while o.usbcmd.read_volatile().host_controller_reset() {}

        debug!("{TAG} Reset HC");

        while self
            .regs()
            .operational
            .usbcmd
            .read_volatile()
            .host_controller_reset()
            || self
                .regs()
                .operational
                .usbsts
                .read_volatile()
                .controller_not_ready()
        {}

        info!("{TAG} XCHI reset ok");
        Ok(())
    }

    fn set_max_device_slots(&mut self) -> Result {
        let max_slots = self.max_slots;
        debug!("{TAG} Setting enabled slots to {}.", max_slots);
        self.regs_mut().operational.config.update_volatile(|r| {
            r.set_max_device_slots_enabled(max_slots);
        });
        Ok(())
    }

    fn set_dcbaap(&mut self) -> Result {
        let dcbaap = self.dev_ctx.dcbaap();
        debug!("{TAG} Writing DCBAAP: {:X}", dcbaap);
        self.regs_mut().operational.dcbaap.update_volatile(|r| {
            r.set(dcbaap as u64);
        });
        Ok(())
    }

    fn set_cmd_ring(&mut self) -> Result {
        let crcr = self.cmd.register();
        let cycle = self.cmd.cycle;

        let regs = self.regs_mut();

        debug!("{TAG} Writing CRCR: {:X}", crcr);
        regs.operational.crcr.update_volatile(|r| {
            r.set_command_ring_pointer(crcr);
            if cycle {
                r.set_ring_cycle_state();
            } else {
                r.clear_ring_cycle_state();
            }
        });

        Ok(())
    }

    fn start(&mut self) -> Result {
        let regs = self.regs_mut();
        debug!("{TAG} Start run");
        regs.operational.usbcmd.update_volatile(|r| {
            r.set_run_stop();
        });

        while regs.operational.usbsts.read_volatile().hc_halted() {}

        info!("{TAG} Is running");

        regs.doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_stream_id(0);
            r.set_doorbell_target(0);
        });

        Ok(())
    }

    fn init_ir(&mut self) -> Result {
        debug!("{TAG} Disable interrupts");
        let regs = &mut self.regs.regs;

        regs.operational.usbcmd.update_volatile(|r| {
            r.clear_interrupter_enable();
        });

        let mut ir0 = regs.interrupter_register_set.interrupter_mut(0);
        {
            debug!("{TAG} Writing ERSTZ");
            ir0.erstsz.update_volatile(|r| r.set(1));

            let erdp = self.event.erdp();
            debug!("{TAG} Writing ERDP: {:X}", erdp);

            ir0.erdp.update_volatile(|r| {
                r.set_event_ring_dequeue_pointer(erdp);
            });

            let erstba = self.event.erstba();
            debug!("{TAG} Writing ERSTBA: {:X}", erstba);

            ir0.erstba.update_volatile(|r| {
                r.set(erstba);
            });
            ir0.imod.update_volatile(|im| {
                im.set_interrupt_moderation_interval(0);
                im.set_interrupt_moderation_counter(0);
            });

            debug!("{TAG} Enabling primary interrupter.");
            ir0.iman.update_volatile(|im| {
                im.set_interrupt_enable();
            });
        }

        // };

        // self.setup_scratchpads(buf_count);

        Ok(())
    }

    fn get_speed(&self, port: usize) -> u8 {
        self.regs()
            .port_register_set
            .read_volatile_at(port)
            .portsc
            .port_speed()
    }

    fn get_default_max_packet_size(&self, port: usize) -> u16 {
        match self.get_speed(port) {
            1 | 3 => 64,
            2 => 8,
            4 => 512,
            v => unimplemented!("PSI: {}", v),
        }
    }

    fn reset_cic(&mut self) {
        let regs = self.regs_mut();
        let cic = regs
            .capability
            .hccparams2
            .read_volatile()
            .configuration_information_capability();
        regs.operational.config.update_volatile(|r| {
            if cic {
                r.set_configuration_information_enable();
            } else {
                r.clear_configuration_information_enable();
            }
        });
    }

    fn reset_ports(&mut self) {
        let regs = self.regs_mut();
        let port_len = regs.port_register_set.len();

        for i in 0..port_len {
            debug!("{TAG} Port {} start reset", i,);
            regs.port_register_set.update_volatile_at(i, |port| {
                port.portsc.set_0_port_enabled_disabled();
                port.portsc.set_port_reset();
            });

            while regs
                .port_register_set
                .read_volatile_at(i)
                .portsc
                .port_reset()
            {}

            debug!("{TAG} Port {} reset ok", i);
        }
    }

    fn fetch_device_desc(
        &mut self,
        dev: &DeviceAttached<O>,
    ) -> Result<descriptors::desc_device::Device> {
        let mut buffer = DMA::new_singleton_page4k(
            descriptors::desc_device::Device::default(),
            self.config.os.dma_alloc(),
        );
        self.control_transfer(
            dev,
            1,
            Some(&mut buffer),
            0x80,
            6,
            DescriptorType::Device.forLowBit(0).bits(),
            0,
            Direction::In,
        )?;

        Ok(buffer.clone())
    }
    fn fetch_package_size0(&mut self, dev: &DeviceAttached<O>) -> Result<u16> {
        let mut buffer = DMA::new_vec(0u8, 8, 64, self.config.os.dma_alloc());
        self.control_transfer(
            dev,
            1,
            Some(&mut buffer),
            0x80,
            6,
            DescriptorType::Device.forLowBit(0).bits(),
            0,
            Direction::In,
        )?;
        let mut data = [0u8; 18];
        data[..8].copy_from_slice(&buffer);

        if let Ok(descriptors::Descriptor::Device(dev)) = descriptors::Descriptor::from_slice(&data)
        {
            return Ok(dev.max_packet_size());
        }
        Ok(8)
    }
    fn fetch_config_desc(
        &mut self,
        dev: &DeviceAttached<O>,
        index: u8,
    ) -> Result<DescriptorConfiguration> {
        let mut buffer = DMA::new_vec(
            0u8,
            PageSize::Size4K.into(),
            PageSize::Size4K.into(),
            self.config.os.dma_alloc(),
        );
        self.control_transfer(
            dev,
            1,
            Some(&mut buffer),
            0x80,
            6,
            DescriptorType::Configuration.forLowBit(index).bits(),
            0,
            Direction::In,
        )?;

        let mut config = None;
        let mut offset = 0;

        while offset < buffer.length_for_bytes() {
            let len = buffer[offset] as usize;
            if len == 0 {
                break;
            }

            let raw = &buffer[offset..offset + len];
            offset += len;
            if let Ok(desc) = Descriptor::from_slice(raw) {
                match desc {
                    Descriptor::Configuration(c) => {
                        if config.is_some() {
                            break;
                        }
                        config = Some(DescriptorConfiguration {
                            data: c,
                            interfaces: Vec::new(),
                        })
                    }
                    Descriptor::Interface(i) => {
                        if let Some(config) = &mut config {
                            config.interfaces.push(DescriptorInterface {
                                data: i,
                                endpoints: Vec::new(),
                            })
                        }
                    }
                    Descriptor::Endpoint(e) => {
                        if let Some(config) = &mut config {
                            if let Some(interface) = config.interfaces.last_mut() {
                                interface.endpoints.push(e);
                            }
                        }
                    }
                    _ => debug!("{:#?}", desc),
                }
            } else {
                break;
            }
        }

        match config {
            Some(config) => Ok(config),
            None => Err(Error::Unknown(format!("config not found"))),
        }
    }

    fn set_ep0_packet_size(&mut self, dev: &DeviceAttached<O>, max_packet_size: u16) -> Result {
        let addr = {
            let input = self.dev_ctx.device_input_context_list[dev.port_id as usize].deref_mut();
            input
                .device_mut()
                .endpoint_mut(1) //dci=1: endpoint 0
                .set_max_packet_size(max_packet_size);

            debug!(
                "CMD: evaluating context for set endpoint0 packet size {}",
                max_packet_size
            );
            (input as *mut Input<16>).addr() as u64
        };
        self.post_cmd(Allowed::EvaluateContext(
            *EvaluateContext::default()
                .set_slot_id(dev.slot_id as _)
                .set_input_context_pointer(addr),
        ))?;

        Ok(())
    }

    fn set_configuration(&mut self, device: &DeviceAttached<O>, config_idx: usize) -> Result {
        let config = &device.configs[config_idx];
        let config_val = config.data.config_val();
        let interface = device.current_interface();
        let input_addr = {
            {
                let input = self.dev_ctx.device_input_context_list[device.slot_id].deref_mut();
                {
                    let control_mut = input.control_mut();
                    control_mut.set_add_context_flag(0);
                    control_mut.set_configuration_value(config_val);

                    control_mut.set_interface_number(interface.data.interface_number);
                    control_mut.set_alternate_setting(interface.data.alternate_setting);
                }
                let mut entries = 1;
                if let Some(config) = device.configs.last() {
                    if let Some(interface) = config.interfaces.last() {
                        if let Some(ep) = interface.endpoints.last() {
                            entries = ep.doorbell_value_aka_dci();
                        }
                    }
                }

                input
                    .device_mut()
                    .slot_mut()
                    .set_context_entries(entries as u8);
            }
            for ep in &interface.endpoints {
                let dci = ep.doorbell_value_aka_dci() as usize;
                let max_packet_size = ep.max_packet_size;
                let ring_addr = self.ep_ring(device, dci as _).register();

                let input = self.dev_ctx.device_input_context_list[device.slot_id].deref_mut();
                let control_mut = input.control_mut();
                debug!("init ep {} {:?}", dci, ep.endpoint_type());
                control_mut.set_add_context_flag(dci);
                let ep_mut = input.device_mut().endpoint_mut(dci);
                ep_mut.set_interval(3);
                ep_mut.set_endpoint_type(ep.endpoint_type());
                ep_mut.set_tr_dequeue_pointer(ring_addr);
                ep_mut.set_max_packet_size(max_packet_size);
                ep_mut.set_error_count(3);
                ep_mut.set_dequeue_cycle_state();
                let endpoint_type = ep.endpoint_type();
                match endpoint_type {
                    EndpointType::Control => {}
                    EndpointType::BulkOut | EndpointType::BulkIn => {
                        ep_mut.set_max_burst_size(0);
                        ep_mut.set_max_primary_streams(0);
                    }
                    EndpointType::IsochOut
                    | EndpointType::IsochIn
                    | EndpointType::InterruptOut
                    | EndpointType::InterruptIn => {
                        //init for isoch/interrupt
                        ep_mut.set_max_packet_size(max_packet_size & 0x7ff); //refer xhci page 162
                        ep_mut.set_max_burst_size(
                            ((max_packet_size & 0x1800) >> 11).try_into().unwrap(),
                        );
                        ep_mut.set_mult(0); //always 0 for interrupt

                        if let EndpointType::IsochOut | EndpointType::IsochIn = endpoint_type {
                            ep_mut.set_error_count(0);
                        }

                        ep_mut.set_tr_dequeue_pointer(ring_addr);
                        ep_mut.set_max_endpoint_service_time_interval_payload_low(4);
                        //best guess?
                    }
                    EndpointType::NotValid => unreachable!("Not Valid Endpoint should not exist."),
                }
            }

            let input = self.dev_ctx.device_input_context_list[device.slot_id].deref_mut();
            (input as *const Input<16>).addr() as u64
        };

        self.post_cmd(Allowed::ConfigureEndpoint(
            *ConfigureEndpoint::default()
                .set_slot_id(device.slot_id as _)
                .set_input_context_pointer(input_addr),
        ))?;

        self.print_context(&device);

        // debug!("set config {}", config_val);
        // self.control_transfer::<u8>(device, 1, None, 0, 0x09, config_val as _, 0, Direction::Out)?;

        // debug!("set interface {}", interface.data.interface);
        // self.control_transfer::<u8>(
        //     device,
        //     1,
        //     None,
        //     1,
        //     0x09,
        //     interface.data.alternate_setting as _,
        //     interface.data.interface_number as _,
        //     Direction::Out,
        // )?;

        Ok(())
    }
}
