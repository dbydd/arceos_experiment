pub mod driver_usb_storage_scsi;
//this mod should not exist in new world;
//and furture more, maybe we should consider reconstruct axdrivers

pub trait USBDriverBasicOps {
    type Driver;
    fn filter(&self, desc: &super::host::structures::descriptor::Interface)
        -> Option<Self::Driver>;
}
