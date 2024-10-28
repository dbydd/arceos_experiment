use std::{
    io::{self},
    sync::Mutex,
};

use axdriver::AllDevices;
// use driver_usb::host::{xhci::MemoryMapper, USBHost, USBHostConfig, Xhci};
use axhal::mem::phys_to_virt;
use lazy_init::LazyInit;

#[cfg(all(not(feature = "axstd"), unix))]

macro_rules! print_err {
    ($cmd: literal, $msg: expr) => {
        println!("{}: {}", $cmd, $msg);
    };
    ($cmd: literal, $arg: expr, $err: expr) => {
        println!("{}: {}: {}", $cmd, $arg, $err);
    };
}

static Drivers: LazyInit<Mutex<AllDevices>> = LazyInit::new();

type CmdHandler = fn(&str);

const CMD_TABLE: &[(&str, CmdHandler)] = &[
    ("exit", do_exit),
    ("help", do_help),
    ("uname", do_uname),
    ("ldr", do_ldr),
    ("str", do_str),
    // ("test_xhci", test_xhci),
    ("dump_dtb", dump_dtb),
    ("test", test),
    ("test_pci", do_test_pci),
    ("test_usb", do_run_usb),
];

fn do_run_usb(_args: &str) {
    Drivers.lock().xhci.iter_mut().for_each(|controller| {
        controller.init().init_probe().drive_all();
    });
}

fn do_uname(_args: &str) {
    let arch = option_env!("AX_ARCH").unwrap_or("");
    let platform = option_env!("AX_PLATFORM").unwrap_or("");
    let smp = match option_env!("AX_SMP") {
        None | Some("1") => "",
        _ => " SMP",
    };
    let version = option_env!("CARGO_PKG_VERSION").unwrap_or("0.1.0");
    println!(
        "ArceOS {ver}{smp} {arch} {plat}",
        ver = version,
        smp = smp,
        arch = arch,
        plat = platform,
    );
}

fn do_test_pci(_args: &str) {
    println!("test pci");
    Drivers.init_by(Mutex::new(axdriver::init_drivers()));
}

fn do_help(_args: &str) {
    println!("Available commands:");
    for (name, _) in CMD_TABLE {
        println!("  {}", name);
    }
}

fn do_exit(_args: &str) {
    println!("Bye~");
    std::process::exit(0);
}

fn do_ldr(args: &str) {
    println!("ldr");
    if args.is_empty() {
        println!("try: ldr ffff0000400fe000 / ldr ffff000040080000 ffff000040080008");
    }

    fn ldr_one(addr: &str) -> io::Result<()> {
        println!("addr = {}", addr);

        if let Ok(parsed_addr) = u64::from_str_radix(addr, 16) {
            let address: *const u64 = parsed_addr as *const u64; // 强制转换为合适的指针类型

            let value: u64;
            println!("Parsed address: {:p}", address); // 打印地址时使用 %p 格式化符号

            unsafe {
                value = *address;
            }

            println!("Value at address {}: 0x{:X}", addr, value); // 使用输入的地址打印值
        } else {
            println!("Failed to parse address.");
        }
        return Ok(());
    }

    for addr in args.split_whitespace() {
        if let Err(e) = ldr_one(addr) {
            println!("ldr {} {}", addr, e);
        }
    }
}

// use crate::mem::phys_to_virt;
// use core::ptr::{read_volatile, write_volatile};

fn do_str(args: &str) {
    println!("str");
    if args.is_empty() {
        println!("try: str ffff0000400fe000 12345678");
    }

    fn str_one(addr: &str, val: &str) -> io::Result<()> {
        println!("addr = {}", addr);
        println!("val = {}", val);

        if let Ok(parsed_addr) = u64::from_str_radix(addr, 16) {
            let address: *mut u64 = parsed_addr as *mut u64; // 强制转换为合适的指针类型
            println!("Parsed address: {:p}", address); // 打印地址时使用 %p 格式化符号

            if let Ok(parsed_val) = u32::from_str_radix(val, 16) {
                let value: u64 = parsed_val as u64; // 不需要将值转换为指针类型
                println!("Parsed value: 0x{:X}", value); // 直接打印解析的值

                // let ptr = phys_to_virt(parsed_addr.into()).as_mut_ptr() as *mut u32;
                unsafe {
                    *address = value;
                    // write_volatile(address, value);
                    // write_volatile(ptr, value);
                }

                println!("Write value at address {}: 0x{:X}", addr, value); // 使用输入的地址打印值
            }
        } else {
            println!("Failed to parse address.");
        }

        Ok(())
    }

    let mut split_iter = args.split_whitespace();

    if let Some(addr) = split_iter.next() {
        println!("First element: {}", addr);

        if let Some(val) = split_iter.next() {
            println!("Second element: {}", val);
            str_one(addr, val).unwrap(); // 调用 str_one 函数并传递 addr 和 val
        }
    }
}

pub fn run_cmd(line: &[u8]) {
    let line_str = unsafe { core::str::from_utf8_unchecked(line) };
    let (cmd, args) = split_whitespace(line_str);
    if !cmd.is_empty() {
        for (name, func) in CMD_TABLE {
            if cmd == *name {
                func(args);
                return;
            }
        }
        println!("{}: command not found", cmd);
    }
}

fn split_whitespace(str: &str) -> (&str, &str) {
    let str = str.trim();
    str.find(char::is_whitespace)
        .map_or((str, ""), |n| (&str[..n], str[n + 1..].trim()))
}

fn dump_dtb(str: &str) {
    // axdtb::dump_dtb();
    println!("{:#?}", axdtb::find_dtb_node("pci-host-ecam-generic"));
}

fn test(_str: &str) {
    let phys_to_virt = 0x40_1000_0000usize as *const u8;
    println!("read:0x{}", unsafe { *(phys_to_virt as *const u64) })
}
