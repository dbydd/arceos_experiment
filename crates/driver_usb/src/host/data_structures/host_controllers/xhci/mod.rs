use alloc::{borrow::ToOwned, boxed::Box, sync::Arc, vec, vec::Vec};
use axhal::time;
use context::{DeviceContextList, ScratchpadBufferArray};
use core::{
    alloc::{Allocator, Layout},
    mem::{self, MaybeUninit},
    num::NonZeroUsize,
    ops::DerefMut,
    sync::atomic::{fence, Ordering},
};
use event_ring::EventRing;
use log::{debug, error, info, trace, warn};
use ring::Ring;
use spinlock::SpinNoIrq;
use xhci::{
    accessor::Mapper,
    context::{DeviceHandler, EndpointState, EndpointType, Input, InputHandler, SlotHandler},
    extended_capabilities::XhciSupportedProtocol,
    ring::trb::{
        command::{self, ConfigureEndpoint, ResetEndpoint, StopEndpoint},
        event::{self, CommandCompletion, CompletionCode, HostController},
        transfer::{self, Direction, Isoch, Normal, TransferType},
    },
    ExtendedCapability,
};

use crate::{
    abstractions::{dma::DMA, PlatformAbstractions},
    err::Error,
    glue::{
        driver_independent_device_instance::DriverIndependentDeviceInstance,
        ucb::{CompleteCode, TransferEventCompleteCode, UCB},
    },
    host::data_structures::MightBeInited,
    usb::{
        descriptors::{
            desc_configuration,
            desc_endpoint::Endpoint,
            topological_desc::{
                TopologicalUSBDescriptorConfiguration, TopologicalUSBDescriptorEndpoint,
                TopologicalUSBDescriptorFunction,
            },
            USBStandardDescriptorTypes,
        },
        operation::{Configuration, Debugop, ExtraStep},
        transfer::control::{
            bRequest, bmRequestType, ControlTransfer, DataTransferType, Recipient, StandardbRequest,
        },
        urb,
    },
    USBSystemConfig,
};

use super::Controller;

mod context;
mod event_ring;
mod ring;

pub type RegistersBase = xhci::Registers<MemMapper>;
pub type RegistersExtList = xhci::extended_capabilities::List<MemMapper>;
pub type SupportedProtocol = XhciSupportedProtocol<MemMapper>;

const TAG: &str = "[XHCI]";

#[derive(Clone)]
pub struct MemMapper;
impl Mapper for MemMapper {
    unsafe fn map(&mut self, phys_start: usize, bytes: usize) -> NonZeroUsize {
        return NonZeroUsize::new_unchecked(phys_start);
    }
    fn unmap(&mut self, virt_start: usize, bytes: usize) {}
}
pub struct XHCI<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    pub regs: RegistersBase,
    pub ext_list: Option<RegistersExtList>,
    max_slots: u8,
    max_ports: u8,
    max_irqs: u16,
    scratchpad_buf_arr: Option<ScratchpadBufferArray<O>>,
    cmd: Ring<O>,
    event: EventRing<O>,
    pub dev_ctx: DeviceContextList<O>,
}

impl<O> XHCI<O>
where
    O: PlatformAbstractions,
{
    pub fn supported_protocol(&mut self, port: usize) -> Option<SupportedProtocol> {
        debug!("[XHCI] Find port {} protocol", port);

        if let Some(ext_list) = &mut self.ext_list {
            ext_list
                .into_iter()
                .filter_map(|one| {
                    if let Ok(ExtendedCapability::XhciSupportedProtocol(protcol)) = one {
                        return Some(protcol);
                    }
                    None
                })
                .find(|p| {
                    let head = p.header.read_volatile();
                    let port_range = head.compatible_port_offset() as usize
                        ..head.compatible_port_count() as usize;
                    port_range.contains(&port)
                })
        } else {
            None
        }
    }

    fn chip_hardware_reset(&mut self) -> &mut Self {
        debug!("{TAG} Reset begin");
        debug!("{TAG} Stop");

        self.regs.operational.usbcmd.update_volatile(|c| {
            c.clear_run_stop();
        });
        debug!("{TAG} Until halt");
        while !self.regs.operational.usbsts.read_volatile().hc_halted() {}
        debug!("{TAG} Halted");

        let mut o = &mut self.regs.operational;
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
            .regs
            .operational
            .usbcmd
            .read_volatile()
            .host_controller_reset()
            || self
                .regs
                .operational
                .usbsts
                .read_volatile()
                .controller_not_ready()
        {}

        info!("{TAG} XCHI reset ok");
        self
    }

    fn set_max_device_slots(&mut self) -> &mut Self {
        let max_slots = self.max_slots;
        debug!("{TAG} Setting enabled slots to {}.", max_slots);
        self.regs.operational.config.update_volatile(|r| {
            r.set_max_device_slots_enabled(max_slots);
        });
        self
    }

    fn set_dcbaap(&mut self) -> &mut Self {
        let dcbaap = O::map_virt_to_phys(self.dev_ctx.dcbaap().into());
        debug!("{TAG} Writing DCBAAP: {:X}", dcbaap);
        self.regs.operational.dcbaap.update_volatile(|r| {
            r.set(dcbaap as u64);
        });
        self
    }

    fn set_cmd_ring(&mut self) -> &mut Self {
        let crcr = self.cmd.register();
        let cycle = self.cmd.cycle;

        let regs = &mut self.regs;

        debug!("{TAG} Writing CRCR: {:X}", crcr);
        regs.operational.crcr.update_volatile(|r| {
            r.set_command_ring_pointer(crcr);
            if cycle {
                r.set_ring_cycle_state();
            } else {
                r.clear_ring_cycle_state();
            }
        });

        self
    }

    fn start(&mut self) -> &mut Self {
        let regs = &mut self.regs;
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

        self
    }

    fn disable_ir(&mut self) -> &mut Self {
        debug!("{TAG} Disable interrupts");
        let regs = &mut self.regs;
        regs.operational.usbcmd.update_volatile(|r| {
            r.clear_interrupter_enable();
        });

        regs.interrupter_register_set
            .interrupter_mut(0)
            .iman
            .update_volatile(|r| {
                r.clear_interrupt_enable();
            });
        self
    }

    fn init_ir(&mut self) -> &mut Self {
        debug!("{TAG} Disable interrupts");
        let regs = &mut self.regs;

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

        self
    }

    fn get_speed(&self, port: usize) -> u8 {
        self.regs
            .port_register_set
            .read_volatile_at(port)
            .portsc
            .port_speed()
    }

    fn parse_default_max_packet_size_from_port(&self, port: usize) -> u16 {
        match self.get_speed(port) {
            1 | 3 => 64,
            2 => 8,
            4 => 512,
            v => unimplemented!("PSI: {}", v),
        }
    }

    fn reset_cic(&mut self) -> &mut Self {
        let regs = &mut self.regs;
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
        self
    }

    fn reset_ports(&mut self) -> &mut Self {
        let regs = &mut self.regs;
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
        self
    }

    fn setup_scratchpads(&mut self) -> &mut Self {
        let scratchpad_buf_arr = {
            let buf_count = {
                let count = self
                    .regs
                    .capability
                    .hcsparams2
                    .read_volatile()
                    .max_scratchpad_buffers();
                debug!("{TAG} Scratch buf count: {}", count);
                count
            };
            if buf_count == 0 {
                error!("buf count=0,is it a error?");
                return self;
            }
            let scratchpad_buf_arr =
                ScratchpadBufferArray::new(buf_count, self.config.lock().os.clone());

            self.dev_ctx.dcbaa[0] = scratchpad_buf_arr.register();

            debug!(
                "{TAG} Setting up {} scratchpads, at {:#0x}",
                buf_count,
                scratchpad_buf_arr.register()
            );
            scratchpad_buf_arr
        };

        self.scratchpad_buf_arr = Some(scratchpad_buf_arr);
        self
    }

    fn test_cmd(&mut self) -> &mut Self {
        //TODO:assert like this in runtime if build with debug mode?
        debug!("{TAG} Test command ring");
        for _ in 0..3 {
            let completion = self
                .post_cmd(command::Allowed::Noop(command::Noop::new()))
                .unwrap();
        }
        debug!("{TAG} Command ring ok");
        self
    }

    fn post_cmd(&mut self, mut trb: command::Allowed) -> crate::err::Result<CommandCompletion> {
        let addr = self.cmd.enque_command(trb);

        self.regs.doorbell.update_volatile_at(0, |r| {
            r.set_doorbell_stream_id(0);
            r.set_doorbell_target(0);
        });

        fence(Ordering::Release);

        let r = self.event_busy_wait_cmd(O::map_virt_to_phys(addr.into()) as _)?;

        /// update erdp
        self.regs
            .interrupter_register_set
            .interrupter_mut(0)
            .erdp
            .update_volatile(|f| {
                f.set_event_ring_dequeue_pointer(self.event.erdp());
            });

        Ok(r)
    }

    fn event_busy_wait_cmd(&mut self, addr: u64) -> crate::err::Result<CommandCompletion> {
        debug!("Wait result");
        loop {
            if let Some((event, cycle)) = self.event.next() {
                match event {
                    event::Allowed::CommandCompletion(c) => {
                        let mut code = CompletionCode::Invalid;
                        if let Ok(c) = c.completion_code() {
                            code = c;
                        } else {
                            continue;
                        }
                        trace!(
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
                    _ => warn!("warn: event: {:?}", event),
                }
            }
        }
    }

    fn trace_dump_context(&self, slot_id: usize) {
        // let dev = &mut self.dev_ctx.device_out_context_list[slot_id];
        // trace!(
        //     "slot {} {:?}",
        //     slot_id,
        //     &mut dev.access().slot().slot_state()
        // );
        // trace!("device context state:{:#?}", &**dev.)
        warn!("fix this by impl debug fot these structures")
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

    fn ep_ring_mut(&mut self, device_slot_id: usize, dci: u8) -> &mut Ring<O> {
        trace!("fetch transfer ring at slot{}-dci{}", device_slot_id, dci);
        &mut self.dev_ctx.transfer_rings[device_slot_id][dci as usize - 1]
    }

    fn update_erdp(&mut self) {
        self.regs
            .interrupter_register_set
            .interrupter_mut(0)
            .erdp
            .update_volatile(|f| {
                f.set_event_ring_dequeue_pointer(self.event.erdp());
            });
    }

    fn event_busy_wait_transfer(&mut self, addr: u64) -> crate::err::Result<event::TransferEvent> {
        trace!("Wait result @{addr:#X}");
        loop {
            // sleep(Duration::from_millis(2));
            if let Some((event, cycle)) = self.event.next() {
                debug!("got event:{:?} cycle:{:?}", event, cycle);
                self.update_erdp();

                match event {
                    event::Allowed::TransferEvent(c) => {
                        let code = c.completion_code().unwrap();
                        trace!(
                            "[Transfer] << {code:#?} @{:#X} got result{}, cycle {}, len {}",
                            c.trb_pointer(),
                            code as usize,
                            c.cycle_bit(),
                            c.trb_transfer_length()
                        );

                        // if c.trb_pointer() != addr {
                        //     debug!("  @{:#X} != @{:#X}", c.trb_pointer(), addr);
                        //     // return Err(Error::Pip);
                        //     continue;
                        // }
                        trace!("code:{:?},pointer:{:x}", code, c.trb_pointer());

                        match code {
                            CompletionCode::Success
                            | CompletionCode::ShortPacket
                            | CompletionCode::RingOverrun => {
                                return Ok(c);
                            }
                            err => {
                                return Err(Error::CMD(code));
                            }
                        }
                    }
                    _ => warn!("event: {:?}", event),
                }
            }
        }
    }

    fn switch_interface(
        &mut self,
        device_slot_id: usize,
        interface: usize,
        alternative: usize,
    ) -> crate::err::Result<UCB<O>> {
        self.control_transfer(
            device_slot_id,
            ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::Out,
                    DataTransferType::Standard,
                    Recipient::Interface,
                ),
                request: bRequest::Generic(StandardbRequest::SetInterface),
                index: interface as _,
                value: alternative as _,
                data: None,
                report: true,
            },
        )
    }

    fn disable_endpoint(
        &mut self,
        device_slot_id: usize,
        dci: usize,
    ) -> crate::err::Result<UCB<O>> {
        let addr = {
            let ctx_in = self
                .dev_ctx
                .device_input_context_list
                .get_mut(device_slot_id)
                .unwrap();
            ctx_in.copy_from_output(
                self.dev_ctx
                    .device_out_context_list
                    .get(device_slot_id)
                    .unwrap(),
            );

            ctx_in.access().control_mut().clear_all_drop_flag();
            ctx_in.access().control_mut().clear_all_nonep0_add_flag();
            ctx_in.access().control_mut().set_add_context_flag(dci);
            ctx_in.access().control_mut().set_drop_context_flag(dci);

            // trace!("input context:{:#?}", ctx_in.);
            warn!("fix input dump!");

            ctx_in.addr()
        };

        let command_completion = self
            .post_cmd(command::Allowed::ConfigureEndpoint(
                *ConfigureEndpoint::default()
                    .set_input_context_pointer(addr as _)
                    .set_slot_id(device_slot_id as _),
            ))
            .unwrap();

        match command_completion.completion_code() {
            Ok(ok) => match ok {
                CompletionCode::Success => Ok(UCB::<O>::new(CompleteCode::Event(
                    TransferEventCompleteCode::Success,
                ))),
                other => panic!("err:{:?}", other),
            },
            Err(err) => Ok(UCB::new(CompleteCode::Event(
                TransferEventCompleteCode::Unknown(err),
            ))),
        }
    }

    fn re_enable_endpoint(
        &mut self,
        device_slot_id: usize,
        desc: Endpoint,
    ) -> crate::err::Result<UCB<O>> {
        trace!(
            "re enable endpoint, desc is : {:#?},endpoint type is {:?}",
            desc,
            desc.endpoint_type()
        );
        let dci = desc.doorbell_value_aka_dci() as _;

        {
            let ctx_in = self
                .dev_ctx
                .device_input_context_list
                .get_mut(device_slot_id)
                .unwrap();
            ctx_in.copy_from_output(
                self.dev_ctx
                    .device_out_context_list
                    .get(device_slot_id)
                    .unwrap(),
            );
            ctx_in.access().device_mut().fill_endpoints_with_0()
        }

        // //TODO: Allocate StreamRings
        // let ring_addr = self
        //     .ep_ring_mut(device_slot_id, dci as _)
        //     .reset()
        //     .register();
        if EndpointState::Disabled
            != self
                .dev_ctx
                .device_out_context_list
                .get_mut(device_slot_id)
                .unwrap()
                .access()
                .endpoint(dci)
                .endpoint_state()
        {
            trace!("endpoint need restart!");
            let command_completion = self
                .post_cmd(command::Allowed::StopEndpoint(
                    *StopEndpoint::default()
                        .set_endpoint_id(dci as _)
                        .set_slot_id(device_slot_id as _),
                ))
                .unwrap();

            match command_completion.completion_code() {
                Ok(ok) => match ok {
                    CompletionCode::Success => {
                        trace!("after stopped it, check it's state!");
                        self.trace_dump_context(device_slot_id);
                    }
                    other => panic!("err:{:?}", other),
                },
                Err(err) => panic!("err:{:?}", err),
            }
        }

        let ctx_addr = {
            Self::setup_endpoint(self, &desc, device_slot_id, true);
            let dev = self
                .dev_ctx
                .device_input_context_list
                .get_mut(device_slot_id)
                .unwrap();
            dev.access().control_mut().clear_all_nonep0_add_flag();
            dev.access().control_mut().clear_all_drop_flag();
            dev.access().control_mut().set_add_context_flag(dci);
            dev.access().control_mut().set_drop_context_flag(dci);

            // trace!("device input context state:{:#?}", &**dev);

            dev.addr()
        };

        let command_completion = self
            .post_cmd(command::Allowed::ConfigureEndpoint(
                *ConfigureEndpoint::default()
                    .set_input_context_pointer(ctx_addr as _)
                    .set_slot_id(device_slot_id as _),
            ))
            .unwrap();

        match command_completion.completion_code() {
            Ok(ok) => match ok {
                CompletionCode::Success => {
                    self.trace_dump_context(device_slot_id);
                    self.prepare_transfer_normal(device_slot_id, dci as _);
                    Ok(UCB::<O>::new(CompleteCode::Event(
                        TransferEventCompleteCode::Success,
                    )))
                }
                other => panic!("err:{:?}", other),
            },
            Err(err) => Ok(UCB::new(CompleteCode::Event(
                TransferEventCompleteCode::Unknown(err),
            ))),
        }
        // let command_completion = self
        //     .post_cmd(command::Allowed::ResetEndpoint(
        //         *ResetEndpoint::new()
        //             .set_endpoint_id(dci as _)
        //             .set_slot_id(device_slot_id as _),
        //     ))
        //     .unwrap();

        // self.trace_dump_context(device_slot_id);
    }

    fn setup_device(
        &mut self,
        device_slot_id: usize,
        configure: &TopologicalUSBDescriptorConfiguration,
    ) -> crate::err::Result<UCB<O>> {
        let input = &mut self.dev_ctx.device_input_context_list[device_slot_id];
        input.access().control_mut().clear_add_context_flag(1);
        /**
        A device can support multiple configurations. Within each configuration can be multiple
        interfaces, each possibly having alternate settings. These interfaces can pertain to different
        functions that co-reside in the same composite device. Several independent video functions can
        exist in the same device. Interfaces that belong to the same video function are grouped into a
        Video Interface Collection described by an Interface Association Descriptor. If the device
        contains multiple independent video functions, there must be multiple Video Interface
        Collections (and hence multiple Interface Association Descriptors), each providing full access to
        their associated video function.
                        */
        configure.child.iter().for_each(|func| match func {
            TopologicalUSBDescriptorFunction::InterfaceAssociation(assoc) => {
                // todo!("enumrate complex device!")

                // let (interface0, attributes, endpoints) =
                assoc
                    .1
                    .iter()
                    .flat_map(|f| match f {
                        TopologicalUSBDescriptorFunction::InterfaceAssociation(_) => {
                            panic!("anyone could help this guy????")
                        }
                        TopologicalUSBDescriptorFunction::Interface(interface) => interface,
                    })
                    .filter(|(i, _, _)| i.interface_number == 0)
                    .for_each(|(_, _, endpoints)| {
                        {
                            let input = &mut self.dev_ctx.device_input_context_list[device_slot_id];

                            let entries = endpoints
                                .iter()
                                .filter_map(|endpoint| {
                                    if let TopologicalUSBDescriptorEndpoint::Standard(ep) = endpoint
                                    {
                                        Some(ep)
                                    } else {
                                        None
                                    }
                                })
                                .map(|endpoint| endpoint.doorbell_value_aka_dci())
                                .max()
                                .unwrap_or(1)
                                .max(input.access().device().slot().context_entries() as u32);

                            input
                                .access()
                                .device_mut()
                                .slot_mut()
                                .set_context_entries(entries as u8);
                        }

                        // trace!("endpoints:{:#?}", endpoints);

                        for item in endpoints {
                            if let TopologicalUSBDescriptorEndpoint::Standard(ep) = item {
                                self.setup_endpoint(ep, device_slot_id, true)
                            }
                        }
                        // }
                    });

                let input_addr = O::map_virt_to_phys(
                    {
                        let input = &mut self.dev_ctx.device_input_context_list[device_slot_id];
                        let control_mut = input.access().control_mut();
                        control_mut.set_add_context_flag(0);
                        control_mut.set_configuration_value(0);

                        control_mut.set_interface_number(0); //
                        control_mut.set_alternate_setting(0); //always exist
                                                              // trace!("device context state:{:#?}", input);
                                                              //TODO: 这玩意根本没修改
                                                              // (input as *const Input<16>).addr()
                        input.addr()
                    }
                    .into(),
                ) as u64;

                let command_completion = self
                    .post_cmd(command::Allowed::ConfigureEndpoint(
                        *command::ConfigureEndpoint::default()
                            .set_slot_id(device_slot_id as _)
                            .set_input_context_pointer(input_addr),
                    ))
                    .unwrap();

                // self.trace_dump_context(device_slot_id);
                match command_completion.completion_code() {
                    Ok(ok) => match ok {
                        CompletionCode::Success => {
                            UCB::<O>::new(CompleteCode::Event(TransferEventCompleteCode::Success))
                        }
                        other => panic!("err:{:?}", other),
                    },
                    Err(err) => {
                        UCB::new(CompleteCode::Event(TransferEventCompleteCode::Unknown(err)))
                    }
                };
            }
            TopologicalUSBDescriptorFunction::Interface(interfaces) => {
                let (interface0, attributes, endpoints) = interfaces.first().unwrap();
                let input_addr = O::map_virt_to_phys({
                    let ret = {
                        let input = &mut self.dev_ctx.device_input_context_list[device_slot_id];
                        {
                            let control_mut = input.access().control_mut();
                            control_mut.set_add_context_flag(0);
                            control_mut.set_configuration_value(configure.data.config_val());

                            control_mut.set_interface_number(interface0.interface_number);
                            control_mut.set_alternate_setting(interface0.alternate_setting);
                        }

                        let entries = endpoints
                            .iter()
                            .filter_map(|endpoint| {
                                if let TopologicalUSBDescriptorEndpoint::Standard(ep) = endpoint {
                                    Some(ep)
                                } else {
                                    None
                                }
                            })
                            .map(|endpoint| endpoint.doorbell_value_aka_dci())
                            .max()
                            .unwrap_or(1);

                        input
                            .access()
                            .device_mut()
                            .slot_mut()
                            .set_context_entries(entries as u8);
                        input.addr()
                    }
                    .into();

                    // debug!("endpoints:{:#?}", interface.endpoints);

                    for item in endpoints {
                        if let TopologicalUSBDescriptorEndpoint::Standard(ep) = item {
                            self.setup_endpoint(ep, device_slot_id, true);
                        }
                    }

                    ret
                }) as u64;

                let command_completion = self
                    .post_cmd(command::Allowed::ConfigureEndpoint(
                        *command::ConfigureEndpoint::default()
                            .set_slot_id(device_slot_id as _)
                            .set_input_context_pointer(input_addr),
                    ))
                    .unwrap();

                self.trace_dump_context(device_slot_id);
                match command_completion.completion_code() {
                    Ok(ok) => match ok {
                        CompletionCode::Success => {
                            UCB::<O>::new(CompleteCode::Event(TransferEventCompleteCode::Success))
                        }
                        other => panic!("err:{:?}", other),
                    },
                    Err(err) => {
                        UCB::new(CompleteCode::Event(TransferEventCompleteCode::Unknown(err)))
                    }
                };
            }
        });

        //TODO: Improve
        Ok(UCB::new(CompleteCode::Event(
            TransferEventCompleteCode::Success,
        )))
    }

    fn setup_endpoint(&mut self, ep: &Endpoint, device_slot_id: usize, extra_step: bool) {
        let dci = ep.doorbell_value_aka_dci() as usize;
        let max_packet_size = ep.max_packet_size;
        let ep_ring_mut = self.ep_ring_mut(device_slot_id, dci as _);
        let ring_addr = ep_ring_mut.register();

        let mut input = &mut self.dev_ctx.device_input_context_list[device_slot_id];
        let control_mut = input.access().control_mut();
        debug!("init ep {} {:?}", dci, ep.endpoint_type());
        control_mut.set_add_context_flag(dci);

        {
            let slot_mut = input.access().device_mut().slot_mut();
            if slot_mut.context_entries() < ep.doorbell_value_aka_dci() as _ {
                slot_mut.set_context_entries(ep.doorbell_value_aka_dci() as _);
            }
        }

        let ep_mut = input.access().device_mut().endpoint_mut(dci);
        ep_mut.set_interval(ep.interval - 1);
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
                ep_mut.set_max_burst_size(((max_packet_size & 0x1800) >> 11).try_into().unwrap());
                ep_mut.set_mult(0); //always 0 for interrupt

                if let EndpointType::IsochOut | EndpointType::IsochIn = endpoint_type {
                    ep_mut.set_error_count(0);
                }

                ep_mut.set_tr_dequeue_pointer(ring_addr);
                ep_mut.set_max_endpoint_service_time_interval_payload_low(4);
                //best guess?

                if extra_step {
                    // ep_mut.set_max_endpoint_service_time_interval_payload_low(16);
                    // ep_mut.set_average_trb_length(16);
                    // ep_mut.set_endpoint_state(EndpointState::Stopped)
                    ep_mut.set_interval(1);
                }
            }
            EndpointType::NotValid => {
                unreachable!("Not Valid Endpoint should not exist.")
            }
        }
    }

    fn prepare_transfer_normal(&mut self, device_slot_id: usize, dci: u8) {
        //in our code , the init state of transfer ring always has ccs = 0, so we use ccs =1 to fill transfer ring
        let mut normal = transfer::Normal::default();
        normal.set_cycle_bit();
        let ring = self.ep_ring_mut(device_slot_id, dci);
        ring.enque_trbs(vec![normal.into_raw(); ring.len() - 1]); //the last is link trb
    }
}

impl<O> Controller<O> for XHCI<O>
where
    O: PlatformAbstractions,
{
    fn new(config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> Self
    where
        Self: Sized,
    {
        let mmio_base = config.lock().base_addr.clone().into();
        unsafe {
            debug!("new registers!");
            let regs = RegistersBase::new(mmio_base, MemMapper);
            let ext_list =
                RegistersExtList::new(mmio_base, regs.capability.hccparams1.read(), MemMapper);

            // TODO: pcie 未配置，读不出来
            let version = regs.capability.hciversion.read_volatile();
            info!("xhci version: {:x}", version.get());
            let hcsp1 = regs.capability.hcsparams1.read_volatile();
            let max_slots = hcsp1.number_of_device_slots();
            let max_ports = hcsp1.number_of_ports();
            let max_irqs = hcsp1.number_of_interrupts();
            let page_size = regs.operational.pagesize.read_volatile().get();
            let ctxsize = regs.capability.hccparams1.read_volatile().context_size();
            debug!(
                "{TAG} Max_slots: {}, max_ports: {}, max_irqs: {}, page size: {}, ctx size: {}",
                max_slots,
                max_ports,
                max_irqs,
                page_size,
                if ctxsize { 64 } else { 32 }
            );

            let dev_ctx = DeviceContextList::new(max_slots, config.clone(), ctxsize);

            // Create the command ring with 4096 / 16 (TRB size) entries, so that it uses all of the
            // DMA allocation (which is at least a 4k page).
            let entries_per_page = O::PAGE_SIZE / mem::size_of::<ring::TrbData>();
            let cmd = Ring::new(config.lock().os.clone(), entries_per_page, true).unwrap();
            let event = EventRing::new(config.lock().os.clone()).unwrap();

            debug!("{TAG} ring size {}", cmd.len());
            debug!("created xhci structure!");

            Self {
                regs,
                ext_list,
                config: config.clone(),
                max_slots: max_slots,
                max_ports: max_ports,
                max_irqs: max_irqs,
                scratchpad_buf_arr: None,
                cmd: cmd,
                event: event,
                dev_ctx: dev_ctx,
            }
        }
    }

    fn init(&mut self) {
        self.chip_hardware_reset()
            .set_max_device_slots()
            .set_dcbaap()
            .set_cmd_ring()
            .init_ir()
            .setup_scratchpads()
            // .disable_ir()
            .start()
            .test_cmd()
            .reset_ports();
    }

    fn probe(&mut self) -> Vec<usize> {
        let mut founded = Vec::new();

        {
            let mut port_id_list = Vec::new();
            let port_len = self.regs.port_register_set.len();
            for i in 0..port_len {
                let portsc = &self.regs.port_register_set.read_volatile_at(i).portsc;
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

                debug!("continue!");

                port_id_list.push(i);
            }

            for port_idx in port_id_list {
                let port_id = port_idx + 1;
                //↓
                let slot_id = self.device_slot_assignment();
                self.dev_ctx.new_slot(slot_id as usize, 0, port_id, 32); //TODO:  we still need to imple the non root hub
                debug!("assign complete!");
                //↓
                self.address_device(slot_id, port_id);
                // self.trace_dump_context(slot_id);
                //↓
                let packet_size0 = self.control_fetch_control_point_packet_size(slot_id);
                trace!("packet_size0: {}", packet_size0);
                //↓
                self.set_ep0_packet_size(slot_id, packet_size0 as _);
                founded.push(slot_id)
            }
        }

        founded
    }

    fn control_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: ControlTransfer,
    ) -> crate::err::Result<UCB<O>> {
        let direction = urb_req.request_type.direction.clone();
        let buffer = urb_req.data;

        let mut len = 0;
        let data = if let Some((addr, length)) = buffer {
            let mut data = transfer::DataStage::default();
            len = length;
            data.set_data_buffer_pointer(addr as u64)
                .set_trb_transfer_length(len as _)
                .set_direction(direction);
            Some(data)
        } else {
            None
        };

        let setup = *transfer::SetupStage::default()
            .set_request_type(urb_req.request_type.into())
            .set_request(match urb_req.request {
                bRequest::Generic(generic) => generic as u8,
                bRequest::DriverSpec(spec) => spec,
            })
            .set_value(urb_req.value)
            .set_index(urb_req.index)
            .set_transfer_type({
                if buffer.is_some() {
                    match direction {
                        Direction::In => TransferType::In,
                        Direction::Out => TransferType::Out,
                    }
                } else {
                    TransferType::No
                }
            })
            .set_length(len as u16);
        trace!("{:#?}", setup);

        let mut status = *transfer::StatusStage::default().set_interrupt_on_completion();

        if urb_req.report {
            status.set_direction();
        }

        //=====post!=======
        let mut trbs: Vec<transfer::Allowed> = Vec::new();

        trbs.push(setup.into());
        if let Some(data) = data {
            trbs.push(data.into());
        }
        trbs.push(status.into());

        let mut trb_pointers = Vec::new();

        {
            let ring = self.ep_ring_mut(dev_slot_id, 1);
            for trb in trbs {
                trb_pointers.push(ring.enque_transfer(trb));
            }
        }

        if trb_pointers.len() == 2 {
            trace!(
                "[Transfer] >> setup@{:#X}, status@{:#X}",
                trb_pointers[0],
                trb_pointers[1]
            );
        } else {
            trace!(
                "[Transfer] >> setup@{:#X}, data@{:#X}, status@{:#X}",
                trb_pointers[0],
                trb_pointers[1],
                trb_pointers[2]
            );
        }

        fence(Ordering::Release);
        self.regs.doorbell.update_volatile_at(dev_slot_id, |r| {
            r.set_doorbell_target(1);
        });

        match self.event_busy_wait_transfer(*trb_pointers.last().unwrap() as _) {
            Ok(complete) => match complete.completion_code() {
                Ok(complete) => match complete {
                    CompletionCode::Success => Ok(UCB::new(CompleteCode::Event(
                        TransferEventCompleteCode::Success,
                    ))),
                    err => panic!("{:?}", err),
                },
                Err(fail) => Ok(UCB::new(CompleteCode::Event(
                    TransferEventCompleteCode::Unknown(fail),
                ))),
            },
            Err(Error::CMD(err)) if let CompletionCode::StallError = err => Ok(UCB::new(
                CompleteCode::Event(TransferEventCompleteCode::Stall),
            )),
            any => panic!("impossible:{:?}", any),
        }
    }

    fn configure_device(
        &mut self,
        dev_slot_id: usize,
        urb_req: Configuration,
        dev: Option<&mut DriverIndependentDeviceInstance<O>>,
    ) -> crate::err::Result<UCB<O>> {
        match urb_req {
            Configuration::SetupDevice(config) => self.setup_device(dev_slot_id, &config),
            Configuration::SwitchInterface(interface, alternative) => {
                let switch_interface = self.switch_interface(dev_slot_id, interface, alternative);
                if switch_interface.is_ok() && dev.is_some() {
                    let dev = dev.unwrap();
                    dev.interface_val = interface;
                    dev.current_alternative_interface_value = alternative;
                }
                switch_interface
            }
            Configuration::SwitchConfig(_, _) => todo!(),
            Configuration::ReEnableEndpoint(desc) => self.re_enable_endpoint(dev_slot_id, desc),
            Configuration::DisableEndpoint(idx) => self.disable_endpoint(dev_slot_id, idx),
        }
    }

    fn device_slot_assignment(&mut self) -> usize {
        // enable slot
        let result = self
            .post_cmd(command::Allowed::EnableSlot(
                *command::EnableSlot::default().set_slot_type({
                    {
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
                    }
                }),
            ))
            .unwrap();

        let slot_id = result.slot_id();
        trace!("assigned slot id: {slot_id}");
        slot_id as usize
    }

    fn address_device(&mut self, slot_id: usize, port_id: usize) {
        let port_idx = port_id - 1;
        let port_speed = self.get_speed(port_idx);
        let max_packet_size = self.parse_default_max_packet_size_from_port(port_idx);
        let dci = 1;

        let transfer_ring_0_addr = self.ep_ring_mut(slot_id, dci).register();
        let ring_cycle_bit = self.ep_ring_mut(slot_id, dci).cycle;
        let context_addr = O::map_virt_to_phys({
            let context_mut = self
                .dev_ctx
                .device_input_context_list
                .get_mut(slot_id)
                .unwrap();

            let control_context = context_mut.access().control_mut();
            control_context.set_add_context_flag(0);
            control_context.set_add_context_flag(1);
            for i in 2..32 {
                control_context.clear_drop_context_flag(i);
            }

            let slot_context = context_mut.access().device_mut().slot_mut();
            slot_context.clear_multi_tt();
            slot_context.clear_hub();
            slot_context.set_route_string(0); // for now, not support more hub ,so hardcode as 0.//TODO: generate route string
            slot_context.set_context_entries(1);
            slot_context.set_max_exit_latency(0);
            slot_context.set_root_hub_port_number(port_id as _); //todo: to use port number
            slot_context.set_number_of_ports(0);
            slot_context.set_parent_hub_slot_id(0);
            slot_context.set_tt_think_time(0);
            slot_context.set_interrupter_target(0);
            slot_context.set_speed(port_speed);

            debug!("device port: {:?}", slot_context.root_hub_port_number());

            let endpoint_0 = context_mut.access().device_mut().endpoint_mut(dci as _);
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

            context_mut.addr().into()
        }) as u64;

        fence(Ordering::Release);

        let result = self
            .post_cmd(command::Allowed::AddressDevice(
                *command::AddressDevice::new()
                    .set_slot_id(slot_id as _)
                    .set_input_context_pointer(context_addr),
            ))
            .unwrap();

        trace!("address slot [{}] ok", slot_id);
    }

    fn control_fetch_control_point_packet_size(&mut self, slot_id: usize) -> u8 {
        trace!("control_fetch_control_point_packet_size");
        let mut buffer = DMA::new_vec(0u8, 8, 64, self.config.lock().os.dma_alloc());
        self.control_transfer(
            slot_id,
            ControlTransfer {
                request_type: bmRequestType::new(
                    Direction::In,
                    DataTransferType::Standard,
                    Recipient::Device,
                ),
                request: StandardbRequest::GetDescriptor.into(),
                index: 0,
                value: crate::usb::descriptors::construct_control_transfer_type(
                    USBStandardDescriptorTypes::Device as u8,
                    0,
                )
                .bits(),
                data: Some((buffer.addr() as usize, buffer.length_for_bytes())),
                report: false,
            },
        )
        .unwrap();

        let mut data = [0u8; 8];
        data[..8].copy_from_slice(&buffer);
        trace!("got {:?}", data);
        data.last()
            .and_then(|len| Some(if *len == 0 { 8u8 } else { *len }))
            .unwrap()
    }

    fn set_ep0_packet_size(&mut self, dev_slot_id: usize, max_packet_size: u16) {
        let addr = O::map_virt_to_phys({
            let input = &mut self.dev_ctx.device_input_context_list[dev_slot_id as usize];
            input
                .access()
                .device_mut()
                .endpoint_mut(1) //dci=1: endpoint 0
                .set_max_packet_size(max_packet_size);

            debug!(
                "CMD: evaluating context for set endpoint0 packet size {}",
                max_packet_size
            );
            input.addr().into()
        }) as u64;
        self.post_cmd(command::Allowed::EvaluateContext(
            *command::EvaluateContext::default()
                .set_slot_id(dev_slot_id as _)
                .set_input_context_pointer(addr),
        ))
        .unwrap();
    }

    fn interrupt_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: crate::usb::transfer::interrupt::InterruptTransfer,
    ) -> crate::err::Result<UCB<O>> {
        let (addr, len) = urb_req.buffer_addr_len;
        let enqued_transfer = self
            .ep_ring_mut(dev_slot_id, urb_req.endpoint_id as _)
            .enque_transfer(transfer::Allowed::Normal(
                *Normal::new()
                    .set_data_buffer_pointer(addr as _)
                    .set_trb_transfer_length(len as _)
                    .set_interrupter_target(0)
                    .set_interrupt_on_short_packet()
                    .set_interrupt_on_completion(),
            ));
        self.regs.doorbell.update_volatile_at(dev_slot_id, |r| {
            r.set_doorbell_target(urb_req.endpoint_id as _);
        });

        let transfer_event = self.event_busy_wait_transfer(enqued_transfer as _).unwrap();
        match transfer_event.completion_code() {
            Ok(complete) => match complete {
                CompletionCode::Success | CompletionCode::ShortPacket => {
                    trace!("ok! return a success ucb!");
                    Ok(UCB::new(CompleteCode::Event(
                        TransferEventCompleteCode::Success,
                    )))
                }
                err => panic!("{:?}", err),
            },
            Err(fail) => Ok(UCB::new(CompleteCode::Event(
                TransferEventCompleteCode::Unknown(fail),
            ))),
        }
    }

    fn extra_step(&mut self, dev_slot_id: usize, urb_req: ExtraStep) -> crate::err::Result<UCB<O>> {
        match urb_req {
            ExtraStep::PrepareForTransfer(dci) => {
                if dci > 1 {
                    self.prepare_transfer_normal(dev_slot_id, dci as u8);
                    Ok(UCB::<O>::new(CompleteCode::Event(
                        TransferEventCompleteCode::Success,
                    )))
                } else {
                    Err(Error::DontDoThatOnControlPipe)
                }
            }
        }
    }

    fn isoch_transfer(
        &mut self,
        dev_slot_id: usize,
        urb_req: crate::usb::transfer::isoch::IsochTransfer,
    ) -> crate::err::Result<UCB<O>> {
        let urb_max_packet_count = urb_req.max_packet;
        let (buffer_addr, total_len) = urb_req.buffer_addr_len;
        let isoch_id = urb_req.endpoint_id;
        let mut transfer_size = urb_req.transfer_size_bytes;
        let ring_len = self
            .ep_ring_mut(dev_slot_id, urb_req.endpoint_id as _)
            .len();

        let endpoint = self
            .dev_ctx
            .device_out_context_list
            .get_mut(dev_slot_id)
            .unwrap()
            .access()
            .endpoint(urb_req.endpoint_id);
        let max_packet_size = endpoint.max_packet_size() as usize;
        let max_burst_size = endpoint.max_burst_size();
        let actual_burst_payload_per_trb = (max_burst_size as usize + 1) * max_packet_size;
        let actual_trb_count =
            (transfer_size / actual_burst_payload_per_trb).min(urb_max_packet_count) + 1;
        //just for high speed, superspeed also should condiser mult size
        trace!(
            "each transfer size:{transfer_size},each trb len: {actual_burst_payload_per_trb}, trb count per transfer:{actual_trb_count}"
        );
        assert!(transfer_size <= total_len);
        assert!(actual_trb_count + 1 < ring_len); // +1 is for link trb

        //basiclly, each burst is a td, but we were testing under high_speed_device, so each td only invoves 1 trb
        //calculate formular is (DIV_ROUND_UP(len + (addr & (1<<16)),1<<16))
        //wtf is addr?

        let mut collect = (0..actual_trb_count)
            .map(|i| i * actual_burst_payload_per_trb + buffer_addr)
            .enumerate()
            .map(|(i, addr)| {
                transfer::Allowed::Isoch({
                    let mut isoch = Isoch::new();
                    isoch
                        .set_start_isoch_asap() // only for first trb in td, if set, then ignore frame id
                        .set_data_buffer_pointer(addr as _)
                        .set_transfer_last_burst_packet_count(max_burst_size)
                        .set_trb_transfer_length(actual_burst_payload_per_trb as _)
                        .set_block_event_interrupt()
                        .set_interrupt_on_completion()
                        .set_interrupt_on_short_packet();
                    // .set_frame_id(value)
                    // .set_td_size_or_tbc((request_times - 1 - i) as _);
                    isoch
                })
            })
            .collect::<Vec<_>>();

        let enqued_trbs = self
            .ep_ring_mut(dev_slot_id, isoch_id as _)
            .enque_transfers(collect);

        self.regs.doorbell.update_volatile_at(dev_slot_id, |r| {
            r.set_doorbell_target(urb_req.endpoint_id as _);
        });

        loop {
            match self.event_busy_wait_transfer(enqued_trbs as _) {
                Ok(transfer_event) => {
                    match transfer_event.completion_code() {
                        Ok(complete) => match complete {
                            CompletionCode::Success | CompletionCode::ShortPacket => {
                                trace!("success")
                            }
                            CompletionCode::RingOverrun => {
                                break;
                            }
                            err => panic!("{:?}", err),
                        },
                        Err(fail) => {
                            return Ok(UCB::new(CompleteCode::Event(
                                TransferEventCompleteCode::Unknown(fail),
                            )))
                        }
                    };
                }
                Err(err) => {
                    panic!("{:#?}", err)
                }
            };
        }

        Ok(UCB::new(CompleteCode::Event(
            TransferEventCompleteCode::Success,
        )))
    }

    fn debug_op(&mut self, dev_slot_id: usize, debug_op: Debugop) -> crate::err::Result<UCB<O>> {
        match debug_op {
            Debugop::DumpDevice => {
                // debug!(
                //     "debug dump device:{:#?}",
                //     &*self.dev_ctx.device_out_context_list[dev_slot_id]
                // );
                warn!("fix this data dump!");
            }
            Debugop::DumpConfigAndInterface => {
                // self.config.lock().os.dma_alloc().allocate(Layout::)
                let mut buffer =
                    DMA::new_vec(0u8, 1, 16, self.config.clone().lock().os.dma_alloc());

                self.control_transfer(
                    dev_slot_id,
                    ControlTransfer {
                        request_type: bmRequestType::new(
                            Direction::In,
                            DataTransferType::Standard,
                            Recipient::Device,
                        ),
                        request: bRequest::Generic(StandardbRequest::GetConfiguration),
                        index: 0,
                        value: 0,
                        data: Some(O::map_addr_len_tuple(buffer.addr_len_tuple())),
                        report: false,
                    },
                );

                debug!("current config value:{:?}", buffer.to_vec());

                buffer.fill(0);

                // self.control_transfer(
                //     dev_slot_id,
                //     ControlTransfer {
                //         request_type: bmRequestType::new(
                //             Direction::In,
                //             DataTransferType::Standard,
                //             Recipient::Device,
                //         ),
                //         request: bRequest::Generic(StandardbRequest::GetInterface),
                //         index: 1,
                //         value: 0,
                //         data: Some(buffer.addr_len_tuple()),
                //     },
                // );

                // debug!("current interface:{:?}", buffer.to_vec())
            }
        }

        Ok(UCB::new(CompleteCode::Debug))
    }
}
