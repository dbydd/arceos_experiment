use super::USBDriverBasicOps;

pub struct USBSCSIDriver;

impl USBDriverBasicOps for USBSCSIDriver {
    type Driver = USBSCSIDriver;

    fn filter(
        &self,
        desc: &crate::host::structures::descriptor::Interface,
    ) -> Option<Self::Driver> {
        match desc.ty() {
            (8, 6, _) => Some(USBSCSIDriver),
            _ => None,
        }
    }
}
