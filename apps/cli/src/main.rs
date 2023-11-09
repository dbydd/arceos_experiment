#![cfg_attr(feature = "axstd", no_std)]
#![cfg_attr(feature = "axstd", no_main)]

#[macro_use]
#[cfg(feature = "axstd")]
extern crate axstd as std;

mod cmd;

// #[cfg(feature = "use-ramfs")]
// mod ramfs;

use brcm_pcie::*;
use std::{io::prelude::*, thread::sleep};

const LF: u8 = b'\n';
const CR: u8 = b'\r';
const DL: u8 = b'\x7f';
const BS: u8 = b'\x08';
const SPACE: u8 = b' ';

const MAX_CMD_LEN: usize = 256;

fn print_prompt() {
    // print!(
    //     "arceos:{}$ ",
    //     path_to_str!(std::env::current_dir().unwrap())
    // );
    print!("arceos# ");
    std::io::stdout().flush().unwrap();
}

struct BCM2711HalImpl;

impl BCM2711Hal for BCM2711HalImpl {
    fn sleep(ms: core::time::Duration) {
        sleep(ms);
    }
}

#[cfg_attr(feature = "axstd", no_mangle)]
fn main() {
    let mut stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    let mut buf = [0; MAX_CMD_LEN];
    let mut cursor = 0;
    cmd::run_cmd("help".as_bytes());
    print_prompt();
    let pcie_base = 0xffff_0000_fd50_0000 as usize;

    let pcie_host_bridge = BCM2711PCIeHostBridge::<BCM2711HalImpl>::new(pcie_base);
    pcie_host_bridge.setup();

    loop {
        if stdin.read(&mut buf[cursor..cursor + 1]).ok() != Some(1) {
            continue;
        }
        if buf[cursor] == b'\x1b' {
            buf[cursor] = b'^';
        }
        match buf[cursor] {
            CR | LF => {
                println!();
                if cursor > 0 {
                    cmd::run_cmd(&buf[..cursor]);
                    cursor = 0;
                }
                print_prompt();
            }
            BS | DL => {
                if cursor > 0 {
                    stdout.write_all(&[BS, SPACE, BS]).unwrap();
                    cursor -= 1;
                }
            }
            0..=31 => {}
            c => {
                if cursor < MAX_CMD_LEN - 1 {
                    stdout.write_all(&[c]).unwrap();
                    cursor += 1;
                }
            }
        }
    }
}
