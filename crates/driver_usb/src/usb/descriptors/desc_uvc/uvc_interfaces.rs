use core::ptr;

use alloc::vec::Vec;
use const_enum::ConstEnum;
use log::trace;

use super::UVCDescriptorTypes;

#[derive(ConstEnum, Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum UVCStandardVideoInterfaceClass {
    CC_Video = 0x0e,
}

#[derive(ConstEnum, Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum UVCInterfaceSubclass {
    UNDEFINED = 0x00,
    VIDEOCONTROL = 0x01,
    VIDEOSTREAMING = 0x02,
    VIDEO_INTERFACE_COLLECTION = 0x03,
}

#[derive(ConstEnum, Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum UVCControlInterfaceSubclass {
    DESCRIPTOR_UNDEFINED = 0x00,
    HEADER = 0x01,
    INPUT_TERMINAL = 0x02,
    OUTPUT_TERMINAL = 0x03,
    SELECTOR_UNIT = 0x04,
    PROCESSING_UNIT = 0x05,
    EXTENSION_UNIT = 0x06,
    ENCODING_UNIT = 0x07,
}

#[derive(ConstEnum, Copy, Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum UVCVSInterfaceSubclass {
    UNDEFINED = 0x00,
    INPUT_HEADER = 0x01,
    OUTPUT_HEADER = 0x02,
    STILL_IMAGE_FRAME = 0x03,
    FORMAT_UNCOMPRESSED = 0x04,
    FRAME_UNCOMPRESSED = 0x05,
    FORMAT_MJPEG = 0x06,
    FRAME_MJPEG = 0x07,
    FORMAT_MPEG2TS = 0x0A,
    FORMAT_DV = 0x0C,
    COLORFORMAT = 0x0D,
    FORMAT_FRAME_BASED = 0x10,
    FRAME_FRAME_BASED = 0x11,
    FORMAT_STREAM_BASED = 0x12,
    FORMAT_H264 = 0x13,
    FRAME_H264 = 0x14,
    FORMAT_H264_SIMULCAST = 0x15,
    FORMAT_VP8 = 0x16,
    FRAME_VP8 = 0x17,
    FORMAT_VP8_SIMULCAST = 0x18,
}

#[derive(ConstEnum, Clone, Debug, Copy, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u8)]
pub(crate) enum UVCStandardVideoInterfaceProtocols {
    PC_PROTOCOL_UNDEFINED = 0x00,
    PC_PROTOCOL_15 = 0x01,
}

#[derive(Debug, Clone)]
pub enum UVCInterface {
    Control(UVCControlInterface),
    Streaming(UVCStreamingInterface),
}

#[derive(Debug, Clone)]
pub enum UVCControlInterface {
    Header(UVCControlInterfaceHeader),
    OutputTerminal(UVCControlInterfaceOutputTerminal),
    InputTerminal(UVCControlInterfaceInputTerminal),
    ExtensionUnit(UVCControlInterfaceExtensionUnit),
    ProcessingUnit(UVCControlInterfaceProcessingUnit),
}

#[derive(Debug, Clone)]
pub enum UVCStreamingInterface {
    InputHeader(UVCVSInterfaceInputHeader),
    OutputHeader,
    FormatUncompressed(UVCVSInterfaceFormatUncompressed),
    FormatMjpeg(UVCVSInterfaceFormatMJPEG),
    FormatMpeg2ts,
    FormatDv,
    FormatFrameBased,
    FormatStreamBased,
    FormatH264,
    FormatH264Simulcast,
    FormatVp8,
    FormatVp8Simulcast,
    COLORFORMAT(UVCVSInterfaceColorFormat),

    StillImageFrame(UVCVSInterfaceStillImageFrame),
    FrameUncompressed(UVCVSInterfaceFrameUncompressed),
    FrameMjpeg(UVCVSInterfaceFrameMJPEG),
    FrameFrameBased,
    FrameH264,
    FrameVp8,
}

#[derive(Clone, Debug, Copy)]
#[allow(non_camel_case_types)]
pub enum UVCStreamingFormartInterface {
    FormatUncompressed(UVCVSInterfaceFormatUncompressed),
    FormatMjpeg(UVCVSInterfaceFormatMJPEG),
    COLORFORMAT(UVCVSInterfaceColorFormat),
    FormatMpeg2ts,
    FormatDv,
    FormatFrameBased,
    FormatStreamBased,
    FormatH264,
    FormatH264Simulcast,
    FormatVp8,
    FormatVp8Simulcast,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum UVCStreamingFrameInterface {
    StillImageFrame(UVCVSInterfaceStillImageFrame),
    FrameUncompressed(UVCVSInterfaceFrameUncompressed),
    FrameMjpeg(UVCVSInterfaceFrameMJPEG),
    FrameFrameBased,
    FrameH264,
    FrameVp8,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCControlInterfaceHeader {
    length: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    bcd_uvc: u16,
    total_length: u16,
    clock_frequency: u32,
    in_collection: u8,
    interface_nr: Vec<u8>,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCControlInterfaceInputTerminal {
    length: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    terminal_id: u8,
    terminal_type: u16,
    associated_terminal: u8,
    string_index_terminal: u8,
    reserved: Vec<u8>,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCControlInterfaceOutputTerminal {
    length: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    terminal_id: u8,
    terminal_type: u16,
    associated_terminal: u8,
    source_id: u8,
    string_index_terminal: u8,
    reserved: Vec<u8>,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCControlInterfaceExtensionUnit {
    length: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    unit_id: u8,
    guid_extension_code: [u8; 16],
    num_controls: u8,
    nr_in_pins: u8,
    source_ids: Vec<u8>,
    control_size: u8,
    controls: Vec<u8>,
    extension: u8,
}

#[derive(Clone, Debug, Copy)]
#[allow(non_camel_case_types)]
pub struct UVCControlInterfaceProcessingUnit {
    length: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    unit_id: u8,
    source_id: u8,
    max_multiplier: u16,
    control_size: u8,
    controls: [u8; 3],
    processing: u8,
    video_standards: u8,
}

#[derive(ConstEnum, Clone, Debug, Copy, PartialEq)]
#[allow(non_camel_case_types)]
#[repr(u16)]
pub(crate) enum UVCCONTROLOutputTerminalType {
    OTT_VendorSpec = 0x300,
    OTT_Display = 0x301,
    OTT_MEDIA_TRANSPORT_OUTPUT = 0x302,
    TT_VendorSpec = 0x0100,
    TT_Streaming = 0x0101,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCVSInterfaceInputHeader {
    length: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    num_formats: u8,
    total_length: u16,
    endpoint_address: u8,
    info: u8,
    terminal_link: u8,
    still_capture_method: u8,
    trigger_support: u8,
    trigger_useage: u8,
    control_size: u8,
    interface_nr: Vec<u8>,
}

#[derive(Clone, Debug, Copy)]
#[allow(non_camel_case_types)]
pub struct UVCVSInterfaceFormatMJPEG {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_sub_type: u8,
    pub format_index: u8,
    pub num_frame_descriptors: u8,
    pub flags: u8,
    pub default_frame_index: u8,
    pub aspect_ratio_x: u8,
    pub aspect_ratio_y: u8,
    pub interlace_flags: u8,
    pub is_copy_protect: u8,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCVSInterfaceFrameMJPEG {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_sub_type: u8,
    pub frame_index: u8,
    pub capabilities: u8,
    pub width: u16,
    pub height: u16,
    pub min_bit_rate: u32,
    pub max_bit_rate: u32,
    pub max_video_frame_buffer_size: u32,
    pub default_frame_interval: u32,
    pub frame_interval_type: u8,
    pub frame_interval: FrameInterval,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCVSInterfaceStillImageFrame {
    length: u8,
    descriptor_type: u8,
    descriptor_sub_type: u8,
    endpoint_address: u8,
    num_image_size_paterns: u8,
    width_heights: Vec<(u16, u16)>,
    num_compression_pattern: u8,
    compressions: Vec<u8>,
}

#[derive(Clone, Copy, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCVSInterfaceFormatUncompressed {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_sub_type: u8,
    pub format_index: u8,
    pub number_frame_descriptor: u8,
    pub guid_format: [u8; 16],
    pub bits_per_pixel: u8,
    pub default_frame_index: u8,
    pub aspect_ratio_x: u8,
    pub aspect_ratio_y: u8,
    pub m_interlace_flags: u8,
    pub is_copy_protect: u8,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub struct UVCVSInterfaceFrameUncompressed {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_sub_type: u8,
    pub frame_index: u8,
    pub capabilities: u8,
    pub width: u16,
    pub height: u16,
    pub min_bit_rate: u32,
    pub max_bit_rate: u32,
    pub max_video_frame_buffer_size: u32,
    pub default_frame_interval: u32,
    pub frame_interval_type: u8,
    pub frame_interval: FrameInterval,
}

#[derive(Clone, Debug)]
#[allow(non_camel_case_types)]
pub enum FrameInterval {
    Continuous((u32, u32, u32)),
    Discrete(Vec<u32>),
}

#[derive(Clone, Debug, Copy)]
#[allow(non_camel_case_types)]
pub struct UVCVSInterfaceColorFormat {
    pub length: u8,
    pub descriptor_type: u8,
    pub descriptor_sub_type: u8,
    pub color_primaries: u8,
    pub transfer_characteristics: u8,
    pub matrix_coefficients: u8,
}

impl UVCControlInterface {
    pub fn from_u8_array(raw: &[u8]) -> Self {
        trace!("buffer:{:?}", raw);
        let len = raw[0];
        let descriptor_type = raw[1];
        let descriptor_sub_type = raw[2];
        trace!(
            "subtype{:?}",
            UVCControlInterfaceSubclass::from(descriptor_sub_type)
        );

        match UVCControlInterfaceSubclass::from(descriptor_sub_type) {
            UVCControlInterfaceSubclass::DESCRIPTOR_UNDEFINED => panic!("impossible"),
            UVCControlInterfaceSubclass::HEADER => Self::Header({
                trace!("header!");
                let len_array_nr = len - 12;
                UVCControlInterfaceHeader {
                    length: len.clone(),
                    descriptor_type,
                    descriptor_sub_type,
                    bcd_uvc: u16::from_ne_bytes(raw[3..=4].try_into().unwrap()),
                    total_length: u16::from_ne_bytes(raw[5..=6].try_into().unwrap()),
                    clock_frequency: u32::from_ne_bytes(raw[7..=10].try_into().unwrap()),
                    in_collection: raw[11],
                    interface_nr: {
                        let mut vec = Vec::new();
                        for i in 12..len {
                            vec.push(raw[i as usize]);
                        }
                        vec
                    },
                }
            }),
            UVCControlInterfaceSubclass::INPUT_TERMINAL => {
                Self::InputTerminal(UVCControlInterfaceInputTerminal {
                    length: len,
                    descriptor_type,
                    descriptor_sub_type,
                    terminal_id: raw[3],
                    terminal_type: u16::from_ne_bytes(unsafe { raw[4..=5].try_into().unwrap() }),
                    associated_terminal: raw[6],
                    string_index_terminal: raw[7],
                    reserved: raw[8..(len as usize)].to_vec(),
                })
            }
            UVCControlInterfaceSubclass::OUTPUT_TERMINAL => {
                Self::OutputTerminal(UVCControlInterfaceOutputTerminal {
                    length: len,
                    descriptor_type,
                    descriptor_sub_type,
                    terminal_id: raw[3],
                    terminal_type: u16::from_ne_bytes(unsafe { raw[4..=5].try_into().unwrap() }),
                    associated_terminal: raw[6],
                    source_id: raw[7],
                    string_index_terminal: raw[8],
                    reserved: raw[9..(len as usize)].to_vec(),
                })
            }
            UVCControlInterfaceSubclass::SELECTOR_UNIT => todo!(),
            UVCControlInterfaceSubclass::PROCESSING_UNIT => {
                Self::ProcessingUnit({
                    //this descriptor may change with different uvc version
                    unsafe { ptr::read((raw as *const [u8]).cast()) }
                })
            }
            UVCControlInterfaceSubclass::EXTENSION_UNIT => Self::ExtensionUnit({
                let nr_in_pins = raw[21];
                let last_in_pin = 21 + nr_in_pins as usize;
                let in_pins = raw[21..last_in_pin].to_vec();

                let control_size = raw[last_in_pin];
                let last_control = last_in_pin + control_size as usize;
                let controls = raw[last_in_pin..last_control].to_vec();

                UVCControlInterfaceExtensionUnit {
                    length: len,
                    descriptor_type,
                    descriptor_sub_type,
                    unit_id: raw[3],
                    guid_extension_code: {
                        let mut codes = [0u8; 16];
                        codes.copy_from_slice(&raw[4..20]);
                        codes
                    },
                    num_controls: raw[20],
                    nr_in_pins,
                    source_ids: in_pins,
                    control_size: control_size,
                    controls,
                    extension: raw[last_control],
                }
            }),
            UVCControlInterfaceSubclass::ENCODING_UNIT => todo!(),
        }
    }
}

impl UVCStreamingInterface {
    pub fn from_u8_array(raw: &[u8]) -> Self {
        trace!("buffer:{:?}", raw);
        let len = raw[0];
        let descriptor_type = raw[1];
        let descriptor_sub_type = UVCVSInterfaceSubclass::from(raw[2]);
        trace!("subtype{:?}", descriptor_sub_type);
        match descriptor_sub_type {
            UVCVSInterfaceSubclass::INPUT_HEADER => Self::InputHeader({
                let control_size = raw[12];
                UVCVSInterfaceInputHeader {
                    length: len,
                    descriptor_type,
                    descriptor_sub_type: descriptor_sub_type.into(),
                    num_formats: raw[3],
                    total_length: u16::from_ne_bytes(raw[4..=5].try_into().unwrap()),
                    endpoint_address: raw[6],
                    info: raw[7],
                    terminal_link: raw[8],
                    still_capture_method: raw[9],
                    trigger_support: raw[10],
                    trigger_useage: raw[11],
                    control_size,
                    interface_nr: raw[13..(len as usize)].to_vec(),
                }
            }),
            UVCVSInterfaceSubclass::FORMAT_MJPEG => {
                Self::FormatMjpeg(unsafe { ptr::read((raw as *const [u8]).cast()) })
            }
            UVCVSInterfaceSubclass::FRAME_MJPEG => {
                let frame_interval_type = raw[25];

                let frame_interval = match frame_interval_type {
                    0 => FrameInterval::Continuous((
                        u32::from_ne_bytes(raw[26..30].try_into().unwrap()),
                        u32::from_ne_bytes(raw[30..34].try_into().unwrap()),
                        u32::from_ne_bytes(raw[34..(len as usize)].try_into().unwrap()),
                    )),
                    other => FrameInterval::Discrete(
                        raw[26..((26 + other * 4) as usize)]
                            .chunks(4)
                            .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
                            .collect(),
                    ),
                };

                Self::FrameMjpeg(UVCVSInterfaceFrameMJPEG {
                    length: len,
                    descriptor_type,
                    descriptor_sub_type: descriptor_sub_type.into(),
                    frame_index: raw[3],
                    capabilities: raw[4],
                    width: u16::from_ne_bytes(raw[5..=6].try_into().unwrap()),
                    height: u16::from_ne_bytes(raw[7..=8].try_into().unwrap()),
                    min_bit_rate: u32::from_ne_bytes(raw[9..13].try_into().unwrap()),
                    max_bit_rate: u32::from_ne_bytes(raw[13..17].try_into().unwrap()),
                    max_video_frame_buffer_size: u32::from_ne_bytes(
                        raw[17..21].try_into().unwrap(),
                    ),
                    default_frame_interval: u32::from_ne_bytes(raw[21..25].try_into().unwrap()),
                    frame_interval_type,
                    frame_interval,
                })
            }
            UVCVSInterfaceSubclass::STILL_IMAGE_FRAME => {
                let num_image_size_paterns = raw[4];
                let loc_num_compression_pattern = 5 + 4 * num_image_size_paterns as usize;
                let width_heights = raw[5..loc_num_compression_pattern]
                    .chunks(4)
                    .map(|t| {
                        (
                            u16::from_ne_bytes(t[0..=1].try_into().unwrap()),
                            u16::from_ne_bytes(t[2..=3].try_into().unwrap()),
                        )
                    })
                    .collect();

                Self::StillImageFrame(UVCVSInterfaceStillImageFrame {
                    length: len,
                    descriptor_type,
                    descriptor_sub_type: descriptor_sub_type.into(),
                    endpoint_address: raw[3],
                    num_image_size_paterns,
                    width_heights,
                    num_compression_pattern: raw[loc_num_compression_pattern],
                    compressions: raw[loc_num_compression_pattern + 1..len as usize].to_vec(),
                })
            }
            UVCVSInterfaceSubclass::FORMAT_UNCOMPRESSED => {
                Self::FormatUncompressed(unsafe { ptr::read((raw as *const [u8]).cast()) })
            }
            UVCVSInterfaceSubclass::FRAME_UNCOMPRESSED => {
                let frame_interval_type = raw[25];

                let frame_interval = match frame_interval_type {
                    0 => FrameInterval::Continuous((
                        u32::from_ne_bytes(raw[26..30].try_into().unwrap()),
                        u32::from_ne_bytes(raw[30..34].try_into().unwrap()),
                        u32::from_ne_bytes(raw[34..(len as usize)].try_into().unwrap()),
                    )),
                    other => FrameInterval::Discrete(
                        raw[26..((26 + other * 4) as usize)]
                            .chunks(4)
                            .map(|c| u32::from_ne_bytes(c.try_into().unwrap()))
                            .collect(),
                    ),
                };

                Self::FrameUncompressed(UVCVSInterfaceFrameUncompressed {
                    length: len,
                    descriptor_type,
                    descriptor_sub_type: descriptor_sub_type.into(),
                    frame_index: raw[3],
                    capabilities: raw[4],
                    width: u16::from_ne_bytes(raw[5..=6].try_into().unwrap()),
                    height: u16::from_ne_bytes(raw[7..=8].try_into().unwrap()),
                    min_bit_rate: u32::from_ne_bytes(raw[9..13].try_into().unwrap()),
                    max_bit_rate: u32::from_ne_bytes(raw[13..17].try_into().unwrap()),
                    max_video_frame_buffer_size: u32::from_ne_bytes(
                        raw[17..21].try_into().unwrap(),
                    ),
                    default_frame_interval: u32::from_ne_bytes(raw[21..25].try_into().unwrap()),
                    frame_interval_type,
                    frame_interval,
                })
            }
            UVCVSInterfaceSubclass::COLORFORMAT => {
                Self::COLORFORMAT(unsafe { ptr::read((raw as *const [u8]).cast()) })
            }

            todo => todo!("impl:{:?}", todo),
        }
    }
}

impl UVCStreamingFormartInterface {
    pub fn ismatch(&self, another: &UVCStreamingFrameInterface) -> bool {
        match self {
            UVCStreamingFormartInterface::FormatUncompressed(_)
                if let UVCStreamingFrameInterface::FrameUncompressed(_) = another =>
            {
                true
            }
            UVCStreamingFormartInterface::FormatMjpeg(_)
                if let UVCStreamingFrameInterface::FrameMjpeg(_) = another =>
            {
                true
            }
            UVCStreamingFormartInterface::FormatMpeg2ts => todo!(),
            UVCStreamingFormartInterface::FormatDv => todo!(),
            UVCStreamingFormartInterface::FormatFrameBased => todo!(),
            UVCStreamingFormartInterface::FormatStreamBased => todo!(),
            UVCStreamingFormartInterface::FormatH264 => todo!(),
            UVCStreamingFormartInterface::FormatH264Simulcast => todo!(),
            UVCStreamingFormartInterface::FormatVp8 => todo!(),
            UVCStreamingFormartInterface::FormatVp8Simulcast => todo!(),
            _ => false,
        }
    }

    pub fn filter_out_self(input: &UVCStreamingInterface) -> Option<Self> {
        match input {
            UVCStreamingInterface::FormatUncompressed(any) => {
                Some(Self::FormatUncompressed(any.clone()))
            }
            UVCStreamingInterface::FormatMjpeg(any) => Some(Self::FormatMjpeg(any.clone())),
            UVCStreamingInterface::FormatMpeg2ts => todo!(),
            UVCStreamingInterface::FormatDv => todo!(),
            UVCStreamingInterface::FormatFrameBased => todo!(),
            UVCStreamingInterface::FormatStreamBased => todo!(),
            UVCStreamingInterface::FormatH264 => todo!(),
            UVCStreamingInterface::FormatH264Simulcast => todo!(),
            UVCStreamingInterface::FormatVp8 => todo!(),
            UVCStreamingInterface::FormatVp8Simulcast => todo!(),
            _ => None,
        }
    }

    pub fn filter_out_color_formart(input: &UVCStreamingInterface) -> Option<Self> {
        match input {
            UVCStreamingInterface::COLORFORMAT(color) => Some(Self::COLORFORMAT(color.clone())),
            _ => None,
        }
    }
}

impl UVCStreamingFrameInterface {
    pub fn filter_out_self(input: &UVCStreamingInterface) -> Option<Self> {
        match input {
            UVCStreamingInterface::FrameUncompressed(any) => {
                Some(Self::FrameUncompressed(any.clone()))
            }
            UVCStreamingInterface::FrameMjpeg(any) => Some(Self::FrameMjpeg(any.clone())),
            UVCStreamingInterface::FrameFrameBased => todo!(),
            UVCStreamingInterface::FrameH264 => todo!(),
            UVCStreamingInterface::FrameVp8 => todo!(),
            _ => None,
        }
    }

    pub fn filter_out_still(input: &UVCStreamingInterface) -> Option<Self> {
        match input {
            UVCStreamingInterface::StillImageFrame(any) => Some(Self::StillImageFrame(any.clone())),
            _ => None,
        }
    }
}
