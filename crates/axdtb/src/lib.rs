#![no_std]
#![feature(allocator_api)]
#![feature(iter_collect_into)]
#![feature(strict_provenance)]

extern crate alloc;
extern crate dtb_walker;

use axhal::mem::phys_to_virt;
use byteorder::{BigEndian, ByteOrder, LittleEndian};
use core::{alloc::Allocator, ops::Range, slice};
use dtb_walker::{
    utils::indent, Dtb, DtbObj, HeaderError as E, PHandle, Reg, StrList, WalkOperation as Op,
};

use alloc::{
    collections::btree_map::BTreeMap,
    string::{String, ToString},
    vec::{self, Vec},
};
use log::{debug, info, log};

const INDENT_WIDTH: usize = 4;

pub static mut DTB: lazy_init::LazyInit<Option<dtb_walker::Dtb<'static>>> =
    lazy_init::LazyInit::new();

pub fn init(dtb: usize) {
    debug!("in! addr:{dtb}");
    unsafe {
        let mut dtb_ptr = phys_to_virt(dtb.into()).as_ptr();

        debug!("aligned!:{:x}", dtb_ptr.addr());
        if *(dtb_ptr as *const u32) != 0xedfe0dd0 {
            debug!("magic number invalid! {:x}", *(dtb_ptr as *const u32));
            DTB.init_by(None);
            return;
        }
        let size = (dtb_ptr.offset(4) as *const u32).read();
        if !(size > 0 && size <= 0x7FFF_FFFF) {
            debug!("head invalid!");
            DTB.init_by(None);
            return;
        }

        // let dtb_slice = convert_dtb_to_big_endian(slice::from_raw_parts(dtb_ptr, size as usize));
        // let dtb_slice = slice::from_raw_parts(dtb_ptr, size as usize).to_vec;

        // let dtb = Dtb::from(dtb_slice.as_ptr() as _, |e| {
        let dtb = Dtb::from_raw_parts_filtered(dtb_ptr as _, |e| {
            matches!(
                e,
                E::Misaligned(4) | E::LastCompVersion(16) | E::StructContent
            )
        });

        DTB.init_by(dtb.inspect_err(|err| debug!("dtb:{:#?}", err)).ok());
    }
}

#[derive(Debug, Default)]
pub struct DTBNode {
    compatible: Vec<String>,
    model: Option<String>,
    phandle: Option<PHandle>,
    status: Option<String>,
    reg: Vec<Range<usize>>,
    virtual_reg: Option<u32>,
    dma_coherent: bool,
    generals: BTreeMap<String, Vec<u8>>,
}

pub fn dump_dtb() {
    unsafe {
        DTB.as_ref().unwrap().walk(|path, obj| match obj {
            DtbObj::SubNode { name } => {
                info!(
                    "{}{path}/{:?}",
                    indent(path.level(), INDENT_WIDTH),
                    String::from_utf8(name.to_vec()).unwrap()
                );
                Op::StepInto
            }
            DtbObj::Property(prop) => {
                let indent = indent(path.level(), INDENT_WIDTH);
                info!("{indent}{prop:?}");
                Op::StepOver
            }
        })
    };
}

pub fn find_dtb_node(compatible_name: &str) -> Option<DTBNode> {
    walk_dtb_node(compatible_name, unsafe { DTB.as_mut().unwrap() })
}

fn walk_dtb_node(compatible_name: &str, dtb: &mut dtb_walker::Dtb) -> Option<DTBNode> {
    unsafe {
        let mut target_path: String = String::new();

        dtb.walk(|path, obj| match obj {
            DtbObj::Property(dtb_walker::Property::Compatible(mut list)) => {
                if list.any(|a| a.as_str_unchecked() == compatible_name) {
                    target_path = path.to_string();
                    return Op::Terminate;
                }
                Op::StepOver
            }
            DtbObj::SubNode { name } => Op::StepInto,
            _ => Op::StepOver,
        });

        debug!("located at {target_path}");

        let mut ret = DTBNode::default();

        dtb.walk(|path, obj| match obj {
            DtbObj::SubNode { name: _ } => Op::StepInto,
            DtbObj::Property(prop) if path.to_string() == target_path => {
                match prop {
                    dtb_walker::Property::Compatible(str_list) => {
                        ret.compatible.push(str_list.to_string())
                    }
                    dtb_walker::Property::Model(model) => {
                        ret.model = Some(model.as_str_unchecked().to_string())
                    }
                    dtb_walker::Property::PHandle(phandle) => ret.phandle = Some(phandle),
                    dtb_walker::Property::Status(status) => {
                        ret.status = Some(status.as_str_unchecked().to_string())
                    }
                    dtb_walker::Property::Reg(reg) => {
                        reg.collect_into(&mut ret.reg);
                    }
                    dtb_walker::Property::VirtualReg(vreg) => ret.virtual_reg = Some(vreg),
                    dtb_walker::Property::DmaCoherent => ret.dma_coherent = true,
                    dtb_walker::Property::General { name, value } => {
                        ret.generals.insert(name.to_string(), value.to_vec());
                    }
                }
                Op::StepOver
            }
            _ => {
                debug!("passed {path}");
                Op::StepOver
            }
        });

        Some(ret)
    }
}

fn convert_dtb_to_big_endian(data: &[u8]) -> Vec<u8> {
    if data.len() % 4 != 0 {
        panic!("DTB data not aligned");
    }

    let mut result = Vec::with_capacity(data.len());

    for chunk in data.chunks_exact(4) {
        let value = LittleEndian::read_u32(chunk);
        result.extend_from_slice(&value.to_be_bytes());
    }

    result
}
