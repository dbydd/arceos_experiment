use axalloc::{global_no_cache_allocator, GlobalNoCacheAllocator};
use axhal::mem::VirtAddr;
use conquer_once::spin::OnceCell;
use log::debug;
use page_box::PageBox;
use spinning_top::Spinlock;
use xhci::{
    context::{Device, Device64Byte, DeviceHandler},
    ring::trb::event::{CompletionCode, TransferEvent},
};

use crate::dma::DMAVec;

use super::{event_ring::TypeXhciTrb, registers};

const XHCI_CONFIG_MAX_SLOTS: usize = 64;
pub(crate) struct SlotManager {
    dcbaa: DMAVec<GlobalNoCacheAllocator, u64>,
    // device: PageBox<[Device64Byte]>,
}

impl SlotManager {
    pub fn assign_device(&mut self, port_id: u8, device: VirtAddr) {
        debug!("assign device: {:?} to dcbaa {}", device, port_id);
        assert!(device.is_aligned(64usize), "device not aligned to 64");

        // self.device[valid_slot_id as usize - 1] = device;
        self.dcbaa[port_id as usize] = device.as_usize() as u64;
        //TODO 需要考虑内存同步问题
        //TODO 内存位置可能不对
    }

    // pub fn deref_device_at(&self, slot_id: usize) -> Device64Byte {
    //     unsafe { *(self.dcbaa[slot_id].as_mut_ptr() as *mut Device64Byte) }
    // }
}

pub(crate) static SLOT_MANAGER: OnceCell<Spinlock<SlotManager>> = OnceCell::uninit();

pub(crate) fn transfer_event(
    uch_completion_code: CompletionCode,
    trb: TransferEvent,
) -> Result<TypeXhciTrb, ()> {
    assert!((1 <= trb.slot_id()) && (usize::from(trb.slot_id()) <= XHCI_CONFIG_MAX_SLOTS));
    debug!("transfer event! param: {:?},{:?}", uch_completion_code, trb);
    match uch_completion_code {
        CompletionCode::Success => {
            debug!("transfer event succeed!");
        }
        any => {
            debug!("failed, code:{:?}", any);
        }
    }
    Ok(trb.into_raw())
    // TODO: event transfer
}

pub(crate) fn new() {
    registers::handle(|r| {
        let hcsp1 = r.capability.hcsparams1.read_volatile();
        let count_device_slots = hcsp1.number_of_device_slots();

        debug!("max slot: {}", count_device_slots); // return 0, not good!

        r.operational.config.update_volatile(|cfg| {
            cfg.set_max_device_slots_enabled(count_device_slots);
        });

        let slot_manager = SlotManager {
            // dcbaa: PageBox::new_slice(
            //     VirtAddr::from(0 as usize),
            //     (count_device_slots + 1) as usize,
            // ),
            dcbaa: {
                let mut dmavec = DMAVec::new(
                    (count_device_slots + 1) as usize,
                    64,
                    global_no_cache_allocator(),
                );
                dmavec.iter_mut().for_each(|slice| *slice = 0_u64);
                dmavec
            }, // device: PageBox::new_slice(Device::new_64byte(), XHCI_CONFIG_MAX_SLOTS + 1),
        };

        r.operational.dcbaap.update_volatile(|d| {
            let addr = slot_manager.dcbaa.as_ptr().addr() as u64;
            debug!("addr of dcbaa: {:x}", addr);
            assert!(addr % 64 == 0, "dcbaa not aligned to 64!");
            d.set(addr);
        });

        SLOT_MANAGER
            .try_init_once(move || Spinlock::new(slot_manager))
            .expect("Failed to initialize `SlotManager`.");
    });
    debug!("initialized!");
}

// pub fn set_dcbaa(buffer_array: &[VirtAddr]) {
//     let mut dcbaa_box = &mut SLOT_MANAGER.get().unwrap().lock().dcbaa;
//     buffer_array
//         .iter()
//         .zip(dcbaa_box.iter_mut())
//         .for_each(|(l, r)| *r = *l);
// }
