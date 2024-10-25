//

use core::alloc::Allocator;
use core::ptr;

use crate::abstractions::dma::DMA;
use crate::usb::transfer::control::bRequest;

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Debug, Clone)]
pub enum UVCSpecBRequest {
    SET_CUR = 0b00000001, //设置属性
    GET_CUR = 0b10000001, //获取当前属性
    GET_MIN = 0b10000010, //获取最小设置属性
    GET_MAX = 0b10000011, //获取最大设置属性
    GET_RES = 0b10000100, //获取分辨率属性
    GET_LEN = 0b10000101, //获取数据长度属性
    GET_INF = 0b10000110, //获取设备支持的特定类请求属性
    GET_DEF = 0b10000111, //获取默认属性
}

impl From<UVCSpecBRequest> for bRequest {
    fn from(value: UVCSpecBRequest) -> Self {
        Self::DriverSpec(value as u8)
    }
}

#[repr(C)]
pub struct UVCStreamControlBlock {
    pub hint: ReadWrite<u16, Hint::Register>,
    pub format_index: u8,
    pub frame_index: u8,
    pub frame_interval: u32,
    pub keyframe_rate: u16,
    pub p_frame_rate: u16,
    pub compress_quality: u16,
    pub compress_window_size: u16,
    pub delay: u16,
    pub max_video_frame_size: u32,
    pub max_payload_transfer_size: u32,
    pub clock_frequency: u32,
    pub framing_info: ReadWrite<u8, FramingInfo::Register>,
    pub prefered_version: u8,
    pub min_version: u8,
    pub max_version: u8,
    pub usage: u8,
    pub bit_depth_luma: u8,
    pub settings: ReadWrite<u8, Settings::Register>,
    pub max_number_of_ref_frames_plus_1: u8,
    pub rate_control_modes: ReadWrite<u16, RateControlModes::Register>,
    pub layout_per_stream: ReadWrite<u8, LayoutPerStream::Register>,
}

impl UVCStreamControlBlock {
    pub fn clear(&mut self) -> &mut Self {
        unsafe {
            ptr::write_bytes(
                self as *mut UVCStreamControlBlock,
                0u8,
                size_of::<UVCStreamControlBlock>(),
            )
        };
        self
    }
}

#[derive(ConstEnum, Debug)]
#[repr(usize)]
pub enum StreamControlBlockVersionToSizeMap {
    Ver1p0 = 26,
    Ver1p1 = 32,
    Ver1p5 = 48,
}

impl<A> DMA<UVCStreamControlBlock, A>
where
    A: Allocator,
{
    pub fn to_buffer(&mut self, map: StreamControlBlockVersionToSizeMap) -> (usize, usize) {
        (self.addr(), map as usize)
    }

    pub fn create_uvcstream_control_block(a: A) -> Self {
        DMA::reserve_with_0(4096, a)
    }
}

use const_enum::ConstEnum;
use tock_registers::register_bitfields;
use tock_registers::register_structs;
use tock_registers::registers::ReadWrite;

// Define the bitfield for the hint bitmap
register_bitfields! {u16,
    pub Hint [
        DwFrameInterval 0,
        WKeyFrameRate 1,
        WPFrameRate 2,
        WCompQuality 3,
        WCompWindowSize 4,
    ]
}

// Define the framing information bitmap
register_bitfields! {u8,
    pub FramingInfo [
        FrameID 0,
        PayloadSpecific 1,
        EndOfStream 2 ,
    ]
}

// Define the settings bitmap
register_bitfields! {u8,
    pub Settings [
        // This will be defined according to the specific payload specification
        // Placeholder for now
        Setting0 0,
        Setting1 1,
        Setting2 2,
        Setting3 3,
        Setting4 4,
        Setting5 5,
        Setting6 6,
        Setting7 7
    ]
}

// Define the rate control modes
register_bitfields! {u16,
    pub RateControlModes [
        Stream0Mode OFFSET(0)  NUMBITS(4),
        Stream1Mode OFFSET(4)  NUMBITS(4),
        Stream2Mode OFFSET(8)  NUMBITS(4),
        Stream3Mode OFFSET(12) NUMBITS(4)
    ]
}

// Define the layout per stream
register_bitfields! {u32,
     pub LayoutPerStream [
        Stream0Layout OFFSET(0)  NUMBITS(16),
        Stream1Layout OFFSET(16) NUMBITS(16),
        Stream2Layout OFFSET(32) NUMBITS(16),
        Stream3Layout OFFSET(48) NUMBITS(16)
    ]
}
