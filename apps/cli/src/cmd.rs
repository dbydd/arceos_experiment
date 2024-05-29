use std::io::{self};

use driver_usb::host::xhci::MemoryMapper;

#[cfg(all(not(feature = "axstd"), unix))]

macro_rules! print_err {
    ($cmd: literal, $msg: expr) => {
        println!("{}: {}", $cmd, $msg);
    };
    ($cmd: literal, $arg: expr, $err: expr) => {
        println!("{}: {}: {}", $cmd, $arg, $err);
    };
}

type CmdHandler = fn(&str);

const CMD_TABLE: &[(&str, CmdHandler)] = &[
    ("exit", do_exit),
    ("help", do_help),
    ("uname", do_uname),
    ("ldr", do_ldr),
    ("str", do_str),
    ("test_xhci", test_xhci),
    ("enum_device", enum_device),
];

fn test_xhci(_args: &str) {
    driver_usb::try_init(0x31a08000 as usize)
    // unsafe { xhci::Registers::new(0xffff_0000_31a0_8000 as usize, MemoryMapper {}) };
}

fn enum_device(_args: &str) {
    driver_usb::enum_device();
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
        println!("try: ldr ffff0000400fe000 4");
    }

    fn ldr_one(addr: &str, offset: &str) -> io::Result<()> {
        // println!("addr = {}", addr);

        if let (Ok(parsed_addr), Ok(parsed_offset)) = (
            u64::from_str_radix(addr, 16),
            u64::from_str_radix(offset, 10),
        ) {
            for i in 0..parsed_offset {
                let address: *const u64 = (parsed_addr + i * 8) as *const u64; // 强制转换为合适的指针类型
                if address.is_aligned() {
                    let value: u64;
                    // println!("Parsed address: {:p}", address); // 打印地址时使用 %p 格式化符号

                    unsafe {
                        value = *address;
                    }

                    let le_bytes = value.to_le_bytes();

                    // println!("Value at address {}: 0x{:X}", addr, value); // 使用输入的地址打印值
                    // println!("value at address{} = 0x{:X}: ", addr, value);
                    for (j, chunk) in le_bytes.chunks(4).enumerate() {
                        let mut chunk_value: u32 = 0;
                        for (i, byte) in chunk.iter().enumerate() {
                            chunk_value |= (*byte as u32) << (i * 8);
                        }
                        println!(
                            "offset: 0x{:02x}: {:032b}",
                            i as usize * 8 + j * 4,
                            chunk_value
                        );
                    }
                } else {
                    println!("addr not aligned!");
                }
            }
        } else {
            println!("Failed to parse address.");
        }
        return Ok(());
    }

    // for addr in args.split_whitespace() {
    //     if let Err(e) = ldr_one(addr) {
    //         println!("ldr {} {}", addr, e);
    //     }
    // }
    let mut split_ascii_whitespace = args.split_ascii_whitespace();
    let base_addr = split_ascii_whitespace.next();
    let byte_counts = split_ascii_whitespace.next().unwrap_or("1");
    ldr_one(base_addr.unwrap(), byte_counts);
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

fn test_net(str: &str) {}
