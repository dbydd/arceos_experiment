//

use crate::usb::trasnfer::control::bRequest;

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
