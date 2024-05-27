use crate::err::*;
use crate::types::*;
use crate::Access;
use crate::PciAddress;
use alloc::vec::Vec;
use core::fmt;
use core::fmt::{Display, Formatter};
use core::marker::PhantomData;
use core::ops::Range;
use log::*;
use tock_registers::interfaces::{ReadWriteable, Readable};
use tock_registers::{
    register_bitfields, register_structs,
    registers::{ReadOnly, ReadWrite},
};
const MAX_BUS: usize = 256;
const MAX_DEVICES: usize = 32;
const MAX_FUNCTIONS: usize = 8;

/// The root complex of a PCI bus.
#[derive(Clone)]
pub struct PciRootComplex<A: Access> {
    mmio_base: usize,
    allocator: PciRangeAllocator,
    _marker: PhantomData<A>,
}

impl<A: Access> PciRootComplex<A> {
    pub fn new(mmio_base: usize, bar_range: Range<u64>) -> Self {
        A::setup(mmio_base);
        Self {
            mmio_base,
            allocator: PciRangeAllocator::new(bar_range),
            _marker: PhantomData::default(),
        }
    }
}

impl<A: Access> PciRootComplex<A> {
    /// Enumerates PCI devices on the given bus.
    pub fn enumerate_bus(&self) -> BusDeviceIterator<A> {
        // Safe because the BusDeviceIterator only reads read-only fields.
        let root = Self {
            mmio_base: self.mmio_base,
            allocator: self.allocator.clone(),
            _marker: PhantomData::default(),
        };
        BusDeviceIterator {
            root,
            next: PciAddress {
                bus: 0,
                device: 0,
                function: 0,
            },
            stack: Vec::new(),
        }
    }

    pub fn bar_info(&self, bdf: PciAddress, slot: u8) -> Option<Bar> {
        let cfg_addr = A::map_conf(self.mmio_base, bdf).unwrap();
        let mut ep = ConifgEndpoint::new(cfg_addr);
        ep.bar(slot)
    }

    fn read<T>(&self, bdf: PciAddress, offset: usize) -> T {
        let cfg_addr = A::map_conf(self.mmio_base, bdf).unwrap();
        unsafe {
            let addr = cfg_addr + offset;
            (addr as *const T).read_volatile()
        }
    }

    fn write<T>(&self, bdf: PciAddress, offset: usize, value: T) {
        let cfg_addr = A::map_conf(self.mmio_base, bdf).unwrap();
        unsafe {
            let addr = cfg_addr + offset;
            (addr as *mut T).write_volatile(value)
        }
    }
}
/// An iterator which enumerates PCI devices and functions on a given bus.
pub struct BusDeviceIterator<A: Access> {
    /// This must only be used to read read-only fields, and must not be exposed outside this
    /// module, because it uses the same CAM as the main `PciRoot` instance.
    root: PciRootComplex<A>,
    next: PciAddress,
    stack: Vec<PciAddress>,
}

impl<A: Access> BusDeviceIterator<A> {}

impl<A: Access> Iterator for BusDeviceIterator<A> {
    type Item = (PciAddress, DeviceFunctionInfo, ConfigSpace);

    fn next(&mut self) -> Option<Self::Item> {
        debug!("into next!");
        loop {
            if self.next.function >= MAX_FUNCTIONS {
                debug!("added");
                self.next.function = 0;
                self.next.device += 1;
            }

            if self.next.device >= MAX_DEVICES {
                if let Some(parent) = self.stack.pop() {
                    let sub = self.next.bus;
                    self.next.bus = parent.bus;
                    self.next.device = parent.device + 1;
                    self.next.function = 0;
                    let cfg_addr = A::map_conf(self.root.mmio_base, parent.clone()).unwrap();
                    let bridge = ConifgPciPciBridge::new(cfg_addr);
                    trace!("Bridge {} set subordinate: {:X}", parent, sub);
                    bridge.set_subordinate_bus_number(sub as _);
                } else {
                    debug!("none!");
                    return None;
                }
            }

            let current = self.next.clone();

            let cfg_addr = match A::map_conf(self.root.mmio_base, current.clone()) {
                Some(c) => c,
                None => {
                    debug!("no conf");
                    if current.function == 0 {
                        debug!("dev +1");
                        self.next.device += 1;
                    } else {
                        debug!("func +1");
                        self.next.function += 1;
                    }
                    continue;
                }
            };

            debug!("begin: {} @ 0x{:X}", current, cfg_addr);
            let header = PciHeader::new(cfg_addr);
            let (vid, did) = header.vendor_id_and_device_id();
            debug!("vid {:X}, did {:X}", vid, did);

            if vid == 0xffff {
                if current.function == 0 {
                    debug!("dev + 1");
                    self.next.device += 1;
                } else {
                    debug!("func + 1");
                    self.next.function += 1;
                }
                continue;
            }
            let multi = header.has_multiple_functions();

            let header_type = header.header_type();
            let (dv, bc, sc, interface) = header.revision_and_class();
            let mut info = DeviceFunctionInfo::default();
            info.vendor_id = vid;
            info.device_id = did;
            info.revision = dv;
            info.class = bc;
            info.subclass = sc;
            info.header_type = header_type;
            info.prog_if = interface;
            let config_space;
            debug!("header_type:{:?}", header_type);
            match header_type {
                HeaderType::PciPciBridge => {
                    debug!("found bridge at {:?}", self.next);
                    let bridge = ConifgPciPciBridge::new(cfg_addr);
                    self.stack.push(current.clone());
                    self.next.bus += 1;
                    self.next.device = 0;
                    self.next.function = 0;
                    bridge.set_secondary_bus_number(self.next.bus as _);
                    bridge.set_subordinate_bus_number(0xff);
                    A::probe_bridge(self.root.mmio_base, &bridge);
                    config_space = ConfigSpace {
                        address: current.clone(),
                        cfg_addr,
                        header,
                        kind: ConfigKind::PciPciBridge { inner: bridge },
                    }
                }
                HeaderType::Endpoint => {
                    if current.function == 0 && !multi {
                        self.next.device += 1;
                    } else {
                        self.next.function += 1;
                    }
                    let ep = config_ep(cfg_addr, &mut self.root.allocator);
                    config_space = ConfigSpace {
                        address: current.clone(),
                        cfg_addr,
                        header,
                        kind: ConfigKind::Endpoint { inner: ep },
                    }
                }
                _ => {
                    debug!("no_header");
                    if current.function == 0 && !multi {
                        self.next.device += 1;
                    } else {
                        self.next.function += 1;
                    }
                    continue;
                }
            }

            let out = (current.clone(), info, config_space);
            return Some(out);
        }

        debug!("isnone...");
        None
    }
}

fn config_ep(cfg_addr: usize, allocator: &mut PciRangeAllocator) -> ConifgEndpoint {
    let mut ep = ConifgEndpoint::new(cfg_addr);
    let mut slot = 0;
    while slot < ConifgEndpoint::MAX_BARS {
        let bar = ep.bar(slot);
        match bar {
            Some(bar) => match bar {
                Bar::Io { port } => {
                    debug!("  BAR {}: IO  port: {:X}", slot, port);
                }
                Bar::Memory64 {
                    address,
                    size,
                    prefetchable,
                } => {
                    let addr = allocator.alloc(size).unwrap();
                    unsafe {
                        ep.write_bar64(slot, addr);
                    }
                    debug!(
                        "  BAR {}: MEM [{:#x}, {:#x}){}{}",
                        slot,
                        addr,
                        addr + size,
                        " 64bit",
                        if prefetchable { " pref" } else { "" },
                    );

                    slot += 1;
                }
                Bar::Memory32 {
                    address,
                    size,
                    prefetchable,
                } => {
                    let addr = allocator.alloc(size as u64).unwrap() as u32;
                    unsafe {
                        ep.write_bar32(slot, addr);
                    }
                    debug!(
                        "  BAR {}: MEM [{:#x}, {:#x}){}{}",
                        slot,
                        addr,
                        addr + size,
                        " 32bit",
                        if prefetchable { " pref" } else { "" },
                    );
                }
            },
            None => {}
        }

        slot += 1;
    }

    ep
}

/// Information about a PCI device function.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeviceFunctionInfo {
    /// The PCI vendor ID.
    pub vendor_id: u16,
    /// The PCI device ID.
    pub device_id: u16,
    /// The PCI class.
    pub class: u8,
    /// The PCI subclass.
    pub subclass: u8,
    /// The PCI programming interface byte.
    pub prog_if: u8,
    /// The PCI revision ID.
    pub revision: u8,
    /// The type of PCI device.
    pub header_type: HeaderType,
}

impl Default for DeviceFunctionInfo {
    fn default() -> Self {
        Self {
            header_type: HeaderType::PciPciBridge,
            vendor_id: 0,
            device_id: 0,
            class: 0,
            subclass: 0,
            prog_if: 0,
            revision: 0,
        }
    }
}

impl Display for DeviceFunctionInfo {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "{:04X}:{:04X} (class {:02x}.{:02x}, rev {:02x}) {:?}",
            self.vendor_id,
            self.device_id,
            self.class,
            self.subclass,
            self.revision,
            self.header_type,
        )
    }
}

/// Used to allocate MMIO regions for PCI BARs.
#[derive(Clone)]
struct PciRangeAllocator {
    range: Range<u64>,
    current: u64,
}

impl PciRangeAllocator {
    /// Creates a new allocator from a memory range.
    pub fn new(range: Range<u64>) -> Self {
        Self {
            range: range.clone(),
            current: range.start,
        }
    }

    /// Allocates a memory region with the given size.
    ///
    /// The `size` should be a power of 2, and the returned value is also a
    /// multiple of `size`.
    pub fn alloc(&mut self, size: u64) -> Option<u64> {
        if !size.is_power_of_two() {
            return None;
        }
        let ret = align_up(self.current, size);
        if ret + size > self.range.end {
            return None;
        }

        self.current = ret + size;
        Some(ret)
    }
}

const fn align_up(addr: u64, align: u64) -> u64 {
    (addr + align - 1) & !(align - 1)
}
