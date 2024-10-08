#![no_std]
#![feature(allocator_api)]
#![feature(iter_collect_into)]


extern crate alloc;
extern  crate dtb_walker;

use core::{alloc::Allocator, ops::Range, slice};
use dtb_walker::{utils::indent, Dtb, DtbObj, HeaderError as E, PHandle, Reg, StrList, WalkOperation as Op};


use alloc::{collections::btree_map::{BTreeMap}, string::{String, ToString}, vec::{self, Vec}};

const INDENT_WIDTH: usize = 4;

pub static mut DTB:lazy_init::LazyInit<Option<dtb_walker::Dtb<'static>>> = lazy_init::LazyInit::new();

pub fn init(dtb:usize){
    unsafe {
        let dtb_ptr = dtb as *const u8;
        if *(dtb_ptr as *const u32) != 0xd00dfeed {
            DTB.init_by(None);
            return;
        }
        let size = (dtb_ptr.offset(4) as *const u32).read();
        if !(size > 0 && size <= 0x7FFF_FFFF) {
            DTB.init_by(None);
            return;
        }
        
        let dtb_slice = slice::from_raw_parts(dtb_ptr, size as usize).to_vec();

        DTB.init_by(
            Dtb::from_raw_parts_filtered(dtb_slice.as_ptr() as _, |e| {
            matches!(e, E::Misaligned(4) | E::LastCompVersion(16))
        }).ok()
        );
    }

}

#[derive(Debug,Default)]
pub struct DTBNode{
    compatible:Vec<String>,
    model:Option<String>,
    phandle: Option<PHandle>,
    status:Option<String>,
    reg: Vec<Range< usize>>,
    virtual_reg:Option<u32>,
    dma_coherent:bool,
    generals:BTreeMap<String,Vec<u8>>,
}


pub fn find_dtb_node(compatible_name:&str,)->Option<DTBNode>{
   unsafe{
       match DTB.as_mut() {
           Some(dtb) => {
               let mut target_path = [0u8;512];

               dtb.walk(|path, obj| match obj {
                   DtbObj::Property(dtb_walker::Property::Compatible(mut list))=>{
                     if list.any(|a|a.as_str_unchecked() == compatible_name){
                    target_path.copy_from_slice(path.last());
                    return Op::Terminate;
                    }
                    Op::StepOver
                    }
                    _=>Op::StepInto
                });


                let mut ret = DTBNode::default();

                dtb.walk(|path, obj| match obj {
                    DtbObj::SubNode { name: _ } if path.last() == target_path => {
                        Op::StepInto
                    }
                    DtbObj::Property(prop) => {
                        match prop {
                            dtb_walker::Property::Compatible(str_list) => ret.compatible.push(str_list.to_string()),
                            dtb_walker::Property::Model(model) => ret.model = Some(model.as_str_unchecked().to_string()),
                            dtb_walker::Property::PHandle(phandle) => ret.phandle = Some(phandle),
                            dtb_walker::Property::Status(status) => ret.status = Some(status.as_str_unchecked().to_string()),
                            dtb_walker::Property::Reg(reg) => {
                                reg.collect_into(&mut ret.reg);
                            },
                            dtb_walker::Property::VirtualReg(vreg) => ret.virtual_reg = Some(vreg),
                            dtb_walker::Property::DmaCoherent => ret.dma_coherent = true,
                            dtb_walker::Property::General { name, value } => {
                                ret.generals.insert(name.to_string(), value.to_vec());
                            },
                        }
                        Op::StepOver
                    }
                    _=>Op::StepOver,
                });

                Some(ret)
            },
            None => None,
        }
    }

}
