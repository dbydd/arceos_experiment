#![no_std]
#![no_main]
#![allow(warnings)]
#![feature(allocator_api)]

use core::ops::RangeBounds;
use core::time::Duration;

use alloc::sync::Arc;
use ax_event_bus::events::mouse::MouseEvent;
use ax_event_bus::events::{EventData, EventHandler, Events};
use axalloc::GlobalNoCacheAllocator;
use axhal::paging::PageSize;
use axhal::{mem::VirtAddr, time::busy_wait};
use driver_pca9685::{car_run_task, Quest};
use driver_usb::abstractions::event::USBSystemEvent;
use driver_usb::{USBSystem, USBSystemConfig};

extern crate alloc;
#[macro_use]
extern crate axstd as std;

#[derive(Clone)]
struct PlatformAbstraction;

impl driver_usb::abstractions::OSAbstractions for PlatformAbstraction {
    type VirtAddr = VirtAddr;
    type DMA = GlobalNoCacheAllocator;

    const PAGE_SIZE: usize = PageSize::Size4K as usize;

    fn dma_alloc(&self) -> Self::DMA {
        axalloc::global_no_cache_allocator()
    }

    fn send_event(&self, event: USBSystemEvent) {
        match event {
            USBSystemEvent::MouseEvent(driver_usb::abstractions::event::MouseEvent {
                dx,
                dy,
                left,
                right,
                middle,
                wheel,
            }) => {
                ax_event_bus::post_event(
                    Events::MouseEvent,
                    EventData::MouseEvent(MouseEvent {
                        dx,
                        dy,
                        left,
                        right,
                        middle,
                        wheel,
                    }),
                );
            }
        };
    }
}

impl driver_usb::abstractions::HALAbstractions for PlatformAbstraction {
    fn force_sync_cache() {}
}

struct MouseEventHandler;

impl EventHandler for MouseEventHandler {
    fn handle(&self, event: &mut ax_event_bus::events::EventData) -> bool {
        if let EventData::MouseEvent(data) = event {
            let mut flag = false;
            println!("{:?}", data);
            match (&data.dx, &data.dy, &data.left) {
                (x, y, _) if (-10..=10).contains(x) && (-10..=10).contains(y) => {
                    car_run_task(Quest::Stop)
                }
                (x, y, _) if y.abs() > x.abs() => {
                    // car_run_task(if *y < 0 { Quest::Advance } else { Quest::Back });
                    if *y < 0 {
                        car_run_task(Quest::Advance)
                    } else {
                        car_run_task(Quest::Back)
                    };
                }
                (x, y, false) if x.abs() > y.abs() => {
                    // car_run_task(
                    if *x > 0 {
                        car_run_task(Quest::RotateLeft)
                    } else {
                        // Quest::RotateLeft
                        car_run_task(Quest::RotateRight)
                    }
                    // );
                }
                (x, y, true) if x.abs() > 10 && y.abs() > 10 => {
                    if *x > 0 {
                        if *y > 0 {
                            car_run_task(Quest::BackRight)
                        } else {
                            car_run_task(Quest::AdvanceRight)
                        }
                    } else {
                        if *y > 0 {
                            car_run_task(Quest::BackLeft)
                        } else {
                            car_run_task(Quest::AdvanceLeft)
                        }
                    }
                }
                _ => {}
            }
            return true;
        }
        false
    }
}

#[no_mangle]
fn main() {
    let mut usbsystem = driver_usb::USBSystem::new({
        USBSystemConfig::new(0xffff_0000_31a0_8000, 48, 0, PlatformAbstraction)
    })
    .init()
    .init_probe();
    println!("usb initialized");

    driver_pca9685::pca_init(2500, 2500, 2500, 2500);
    println!("i2c init completed");

    let handler: Arc<dyn EventHandler> = Arc::new(MouseEventHandler);

    ax_event_bus::register_handler(Events::MouseEvent, &handler);
    println!("handler registered");

    usbsystem.drive_all();
}
