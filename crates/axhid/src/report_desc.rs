use num_derive::FromPrimitive;

#[derive(FromPrimitive)]
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ReportTy {
    Input = 1,
    Output,
    Feature,
}

pub const REPORT_DESC_TY: u8 = 34;
