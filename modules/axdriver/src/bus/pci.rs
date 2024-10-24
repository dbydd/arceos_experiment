use crate::{bus::pci, prelude::*, AllDevices};
use axhal::mem::phys_to_virt;
use driver_pci::*;

impl AllDevices {
    pub(crate) fn probe_bus_devices(&mut self) {
        // let base_vaddr = phys_to_virt(axconfig::PCI_ECAM_BASE.into());
        let base_vaddr = phys_to_virt(axdtb::find_dtb_node("pci-host-ecam-generic").unwrap().reg.first().map(|range|range.start).unwrap().into());
        let pci_range = axconfig::PCI_RANGES.get(1).unwrap();
        let mut root = driver_pci::new_root_complex(
            base_vaddr.as_usize(),
            pci_range.0 as u64..pci_range.1 as u64,
        );

        debug!("probing in pci.rs!");
        debug!("PCI RANGE {:x} {:x}", pci_range.0, pci_range.1);

        for (bdf, dev_info, cfg) in root.enumerate_bus() {
            debug!("PCI {} {}", bdf, dev_info);
            for_each_drivers!(type Driver,{
                            if let Some(dev) = Driver::probe_pci(&mut root, bdf.clone(), &dev_info, &cfg) {
                                info!(
                                    "registered a new {:?} device at {}: {:?}",
                                    dev.device_type(),
                                    bdf,
                                    dev.device_name(),
                                );
                                self.add_device(dev);
                                continue; // skip to the next device
                            }
            });
        }
        debug!("new root over");
    }
}
