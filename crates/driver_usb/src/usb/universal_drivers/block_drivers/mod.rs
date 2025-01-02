use num_derive::{FromPrimitive, ToPrimitive};

pub mod bot_protocol;

#[derive(Copy, Clone, Debug, ToPrimitive, FromPrimitive)]
#[repr(u8)]
pub enum USBMassStorageSubclassCode {
    SCSI_CommandSetNotReported = 0x00,
    RBC = 0x01,   // Reduced Block Commands
    MMC_5 = 0x02, // MultiMediaCard
    Obsolete_QIC157 = 0x03,
    UFI = 0x04, // Uniform Floppy Interface
    Obsolete_SFF8070ti = 0x05,
    SCSI_TransparentCommandSet = 0x06,
    LSD_FS = 0x07,
    IEEE1667 = 0x08h,
    VendorSpec = 0xff,
    Reserved,
}
