use core::{f32::consts::E, usize};

use aarch64_cpu::asm::barrier::{self, SY};
use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use futures_util::future::ok;
use log::debug;
use spinning_top::Spinlock;
use xhci::{
    context::Slot,
    extended_capabilities::debug::Debug,
    registers::operational::PageSizeRegister,
    ring::trb::{
        command::{
            self, AddressDevice, Allowed, ConfigureEndpoint, DisableSlot, EnableSlot,
            EvaluateContext, ResetDevice, ResetEndpoint, SetTrDequeuePointer,
        },
        event::{self, CommandCompletion, CompletionCode},
    },
};

use super::{command_ring::CmdRing, registers, XHCI_CONFIG_MAX_SLOTS};
use crate::{dma::DMAVec, host::structures::xhci_event_manager};

pub(crate) struct CommandManager {
    command_ring: CmdRing,
    current_trb: VirtAddr,
}

type SlotID = u8;
#[derive(Debug)]
pub(crate) enum CommandResult {
    Success(CommandCompletion),
    OtherButSuccess(CommandCompletion),
    ERROR(u8),
    InvalidSlot,
    ImpossibleAvailable,
}

impl CommandManager {
    fn slot_id_in_valid_range(slotid: u8) -> bool {
        (1..=XHCI_CONFIG_MAX_SLOTS).contains(&(slotid as usize))
    }

    pub fn disable_slot(&mut self, slotid: SlotID) -> CommandResult {
        if Self::slot_id_in_valid_range(slotid) {
            return self.do_command(Allowed::DisableSlot({
                let mut disable_slot = DisableSlot::new();
                disable_slot.set_slot_id(slotid);
                disable_slot
            }));
        }
        return CommandResult::InvalidSlot;
    }

    pub fn evaluate_context(&mut self, slot_id: u8, input_addr: VirtAddr) -> CommandResult {
        self.do_command(Allowed::EvaluateContext(
            *EvaluateContext::default()
                .set_input_context_pointer(input_addr.as_usize() as u64)
                .set_slot_id(slot_id),
        ))
    }

    pub fn reset_device(&mut self, slot_id: u8) -> CommandResult {
        self.do_command(Allowed::ResetDevice(
            *ResetDevice::default().set_slot_id(slot_id),
        ))
    }

    pub fn enable_slot(&mut self) -> CommandResult {
        self.do_command(Allowed::EnableSlot(EnableSlot::new()))
    }

    pub fn reset_endpoint(&mut self, endpoint_id: u8, slot_id: u8) -> CommandResult {
        self.do_command(Allowed::ResetEndpoint(
            *ResetEndpoint::default()
                .set_endpoint_id(endpoint_id)
                .set_slot_id(slot_id),
        ))
    }

    pub fn config_endpoint(
        &mut self,
        slot_id: u8,
        input_ctx: VirtAddr,
        defconfig: bool,
    ) -> CommandResult {
        if (defconfig) {
            self.do_command(Allowed::ConfigureEndpoint(
                *ConfigureEndpoint::default()
                    .set_slot_id(slot_id)
                    .set_deconfigure()
                    .set_input_context_pointer(input_ctx.as_usize() as u64),
            ))
        } else {
            self.do_command(Allowed::ConfigureEndpoint(
                *ConfigureEndpoint::default()
                    .set_slot_id(slot_id)
                    .clear_deconfigure()
                    .set_input_context_pointer(input_ctx.as_usize() as u64),
            ))
        }
    }

    pub fn address_device(&mut self, addr: VirtAddr, slot_id: u8, bsr: bool) -> CommandResult {
        debug!("addressing device!");
        if bsr {
            self.do_command(Allowed::AddressDevice({
                let mut address_device = AddressDevice::default();
                address_device
                    .set_input_context_pointer(addr.as_usize() as u64)
                    .set_slot_id(slot_id)
                    .set_block_set_address_request();
                address_device
            }))
        } else {
            self.do_command(Allowed::AddressDevice({
                let mut address_device = AddressDevice::default();
                address_device
                    .set_input_context_pointer(addr.as_usize() as u64)
                    .set_slot_id(slot_id);
                address_device
            }))
        }
    }

    pub fn set_transfer_ring_deque(&mut self, endpoint_id: u8, slot_id: u8) -> CommandResult {
        self.do_command(Allowed::SetTrDequeuePointer(
            *SetTrDequeuePointer::default()
                .set_slot_id(slot_id)
                .set_endpoint_id(endpoint_id),
        ))
    }

    pub fn do_commands(&mut self, trb: &[Allowed]) {
        trb.iter().for_each(|trb| {
            self.do_command(*trb);
        })
    }

    pub fn do_command(&mut self, trb: Allowed) -> CommandResult {
        debug!("do command {:?} !", trb);
        //todo check
        let mut trb1 = trb.into_raw();
        trb1[3] |= self.command_ring.cycle_state(); //weird
        if let Some(poped) = self.command_ring.get_enque_trb() {
            debug!(
                "enque trb, assert addr 16B aligned! {:x}",
                poped.as_ptr().addr()
            );
            assert!(poped.as_ptr().is_aligned_to(16));

            *poped = trb1;
            self.command_ring.inc_enque();

            registers::handle(|r| {
                r.doorbell.update_volatile_at(0, |d| {
                    d.set_doorbell_stream_id(0);
                    d.set_doorbell_target(0);
                });
                //TODO: suspect, view
            });
            debug!("waiting for interrupt handler complete!");
            while let handle_event = xhci_event_manager::handle_event() {
                match handle_event {
                    Ok(trb) if let Ok(complete) = CommandCompletion::try_from(trb) => {
                        registers::handle(|r| {
                            r.interrupter_register_set
                                .interrupter_mut(0)
                                .erdp
                                .update_volatile(|erdp| {
                                    erdp.clear_event_handler_busy();
                                })
                        });

                        match complete.completion_code() {
                            Ok(code) if code == CompletionCode::Success => {
                                debug!("t o t a l success!");
                                return CommandResult::Success(complete);
                            }
                            Ok(other) => {
                                debug!("ok, but: {:?}\n full trb: {:?}", other, complete);
                                return CommandResult::OtherButSuccess(complete);
                            }
                            Err(err) => {
                                debug!("command complete error {err} !");
                                return CommandResult::ERROR(err);
                            }
                        }
                    }
                    _ => {}
                }
            }

            // while let handle_event = xhci_event_manager::handle_event() {
            //     if handle_event.is_ok() {
            //         debug!("interrupt handler complete! result = {:?}", handle_event);
            //         let command_result =
            //             CommandCompletion::try_from(handle_event.unwrap()).unwrap();
            //         let slot_id = command_result.slot_id();

            //         if Self::slot_id_in_valid_range(self.uch_slot_id) {
            //             debug!("slot id valid!");
            //             return CommandResult::Success(
            //                 self.uch_complete_code,
            //                 Some(self.uch_slot_id),
            //                 command_result,
            //             );
            //         } else {
            //             return CommandResult::NoSlotsAvailableError;
            //         }
            //     }
            // }
        }
        CommandResult::ImpossibleAvailable
    }

    pub fn command_ring_ptr(&self) -> VirtAddr {
        self.command_ring.get_ring_addr()
    }
}

pub(crate) static COMMAND_MANAGER: OnceCell<Spinlock<CommandManager>> = OnceCell::uninit();

pub(crate) fn command_completed(trb: CommandCompletion) -> Result<[u32; 4], ()> {
    debug!("handleing command complete:{:?}", trb);
    Ok(trb.into_raw())

    // let slotid = uch_slot_id + 1;
    // debug!("command_complete: trying to lock!");
    // let mut command_manager = unsafe { &mut (*COMMAND_MANAGER.try_get().unwrap().data_ptr()) };
    // debug!("command_complete: locked!");
    // if command_manager.command_complete || command_manager.current_trb != trb {
    //     debug!(
    //         "equal! return ! {},0x{:x}",
    //         command_manager.command_complete,
    //         command_manager.current_trb.as_usize()
    //     );
    //     // return;
    // }
    // command_manager.current_trb = 0.into();

    // barrier::dmb(SY);

    // command_manager.command_complete = true;
}

pub(crate) fn new() {
    let cmd_manager = CommandManager {
        command_ring: CmdRing::new(),
        current_trb: VirtAddr::from(0),
    };
    registers::handle(|r| {
        r.operational.crcr.update_volatile(|c| {
            c.set_command_ring_pointer(cmd_manager.command_ring.get_ring_addr().as_usize() as u64);
            if cmd_manager.command_ring.cycle_state() != 0 {
                c.set_ring_cycle_state();
            } else {
                c.clear_ring_cycle_state();
            };
        });

        COMMAND_MANAGER
            .try_init_once(move || Spinlock::new(cmd_manager))
            .expect("Failed to initialize `CommandManager`.");
    });

    debug!("initialized!");
}
