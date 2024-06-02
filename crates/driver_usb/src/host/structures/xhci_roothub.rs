use core::mem::MaybeUninit;
use core::time::Duration;
use core::{num, option, panic, result};

use aarch64_cpu::asm::barrier::{self, SY};
use alloc::string::String;
use alloc::sync::Arc;
use axalloc::GlobalNoCacheAllocator;
use axtask::sleep;
use conquer_once::spin::OnceCell;
use log::{debug, info};
use num_traits::ToPrimitive;
use page_box::PageBox;
use spinning_top::{lock_api::Mutex, Spinlock};
use xhci::context::{Device64Byte, DeviceHandler};
use xhci::ring::trb::command::Allowed;
use xhci::ring::trb::event::PortStatusChange;
use xhci::{context::Device, registers::PortRegisterSet};

use crate::host::structures::xhci_command_manager::{CommandResult, COMMAND_MANAGER};
use crate::host::structures::xhci_slot_manager::{SlotManager, SLOT_MANAGER};
use crate::host::structures::{dump_port_status, root_port, PortLinkState};
use crate::{dma::DMAVec, host::structures::XHCI_CONFIG_MAX_PORTS};

use super::registers;
use super::root_port::RootPort;
use super::xhci_usb_device::XHCIUSBDevice;

// 定义静态变量ROOT_HUB，用于存储根集线器的实例
pub(crate) static ROOT_HUB: OnceCell<Spinlock<Roothub>> = OnceCell::uninit();

pub struct Roothub {
    ports: usize,
    root_ports: PageBox<[Arc<MaybeUninit<Spinlock<RootPort>>>]>, //pagebox分配 无 cache的空间
}

impl Roothub {
    pub fn initialize(&mut self) {
        //todo delay?
        debug!("initializing root ports");
        self.root_ports
            .iter_mut()
            .map(|a| unsafe { a.clone().assume_init() })
            .for_each(|arc| {
                debug!("initializing port {}", arc.lock().root_port_id);
                arc.lock().initialize();
            });
    }
}

// 当接收到根端口状态变化的通知时调用
// 这里似乎产生了无限循环
pub(crate) fn status_changed(port_status_changed: PortStatusChange) {
    // 将UCH端口ID转换为索引，并确保索引在有效范围内
    let port_id = port_status_changed.port_id() - 1;
    // let n_port = uch_port_id as usize - 1; //TODO 真的是-1吗？
    // let mut root_hub = ROOT_HUB
    //     .try_get()
    //     .expect("ROOT_HUB is not initialized")
    //     .data_ptr();
    // assert!(
    //     n_port < unsafe { (*root_hub).ports },
    //     "Port index out of bounds"
    // );
    debug!("port {port_id} changed!");
    dump_port_status(port_id as usize);

    // 如果端口存在，则更新其状态
    //丑陋，临时解决策略
    // unsafe { (*(*(*root_hub).root_ports[n_port].as_ptr()).data_ptr()).status_changed() }
    // if let arc_root_port =
    //     unsafe { (*(*(*root_hub).root_ports[n_port].as_mut_ptr()).data_ptr()).status_changed() }
    // {
    //     //check: does clone affect value?
    //     let mut root_port = unsafe { arc_root_port.status_changed() };
    // } else {
    //     panic!("Root port doesn't exist");
    // }
}

pub(crate) fn new() {
    let number_of_ports = registers::handle(|r| r.port_register_set.len() as usize);
    // 通过MMIO读取根集线器支持的端口数量
    let mut root_ports = PageBox::new_slice(Arc::new_zeroed(), number_of_ports); //DEBUG: using nocache allocator
                                                                                 //TODO 这里全都是同一个ARC,共享了内存，导致反复重复复制，需要修改
    debug!("number of ports:{}", number_of_ports);
    root_ports
        .iter_mut()
        .enumerate()
        .for_each(|(i, port_uninit)| {
            debug!("allocating port {i}");
            dump_port_status(i);
            *port_uninit = Arc::new(MaybeUninit::new(Spinlock::new(RootPort {
                root_port_id: i,
                device: MaybeUninit::zeroed(),
                device_inited: false,
            })));
            debug!("assert:{} == {i}", unsafe {
                port_uninit.clone().assume_init().lock().root_port_id
            })
        });
    debug!("ended");
    // 初始化ROOT_HUB静态变量
    ROOT_HUB.init_once(move || {
        let mut roothub = Roothub {
            ports: number_of_ports as usize,
            root_ports,
        };
        roothub.into()
    });

    debug!("initialized!");

    //wait 300ms

    //ininialize root port
}
