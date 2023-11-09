use core::f32::consts::E;
use std::io::{self};

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
    ("uart", do_UART),
    ("test", do_test),
    ("move", do_m0ve),
    ("test_pci", pci_test),
];

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
        println!("try: ldr ffff0000400fe000 / ldr ffff000040080000 ffff000040080008");
    }

    fn ldr_one(addr_offset: (u64, u64)) -> io::Result<()> {
        println!("addr = {:x},offset = {:x}", addr_offset.0, addr_offset.1);

        let address: *const u64 = addr_offset.0 as *const u64; // 强制转换为合适的指针类型

        let value: u64;
        let conv_val: u64;
        println!("Parsed address: {:p}", address); // 打印地址时使用 %p 格式化符号

        unsafe {
            value = *address.clone();
            conv_val = (value >> (8 * (4 - addr_offset.1))) & 0x00ff;
        }

        println!(
            "prime: Value at address {:x}, value: 0x{:x} | splitted: Value at address {:x}: ,value: 0x{:x}",
            addr_offset.0,
            value,
            addr_offset.0 + addr_offset.1,
            conv_val,
        ); // 使用输入的地址打印值
        return Ok(());
    }

    let mut splitted = args.split_ascii_whitespace();
    match splitted.next() {
        Some(base_addr) => {
            if let Some(count) = splitted.next() {
                let from_str_radix =
                    u64::from_str_radix(count, 16).expect("panic while convert offset");
                for offset in 0..from_str_radix {
                    ldr_one(conv_addr_and_offset_with_4offset(base_addr, offset * 4))
                        .expect("panic while load value");
                }
            } else {
                ldr_one(conv_addr_with_4offset(base_addr)).expect("panic while load value");
            }
        }
        None => {
            panic!("error at first arg: ldr $1");
        }
    }
}

// use crate::mem::phys_to_virt;
use core::ptr::{read_volatile, write_volatile};

fn do_str(args: &str) {
    println!("str");
    if args.is_empty() {
        println!("try: str ffff0000400fe000 12345678");
    }

    fn str_one(addr: &str, val: &str) -> io::Result<()> {
        println!("addr = {}", addr);
        println!("val = {}", val);

        let addr_offset = conv_addr_with_4offset(addr);

        let address: *mut u64 = addr_offset.0 as *mut u64; // 强制转换为合适的指针类型
        println!("Parsed address: {:p}", address); // 打印地址时使用 %p 格式化符号

        if let Ok(parsed_val) = u32::from_str_radix(val, 16) {
            let value_pre: u64 = unsafe { *address };

            let mut value: u64 = parsed_val as u64; // 不需要将值转换为指针类型
            let mask = addr_offset.1 * 8;
            println!(
                "Parsed value: 0x{:X},address:{:x},offset:{:x}",
                value, addr_offset.0, addr_offset.1
            ); // 直接打印解析的值

            value = (value << mask) | (value_pre & mask);

            unsafe {
                //  *address = value
                write_volatile(address, value)
            }

            println!("Write value at address {}: 0x{:X}", addr, value); // 使用输入的地址打印值
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

fn conv_addr_with_4offset(str: &str) -> (u64, u64) {
    let from_str_radix = u64::from_str_radix(str, 16).expect("error while convert address");
    let offset = (from_str_radix << 32 >> 32) % 8;
    (from_str_radix - offset, offset)
}

fn conv_addr_and_offset_with_4offset(str: &str, offset: u64) -> (u64, u64) {
    let from_str_radix =
        u64::from_str_radix(str, 16).expect("error while convert address") + offset;
    let offset = (from_str_radix << 32 >> 32) % 8;

    (from_str_radix - offset, offset)
}

fn do_UART(args: &str) {
    //issue: current str will remove previous value
    // shoult turn to or_str
    match u8::from_str_radix(args, 10) {
        Ok(5) => {
            // let str_addr0 = "ffff0000fe200000 1B";
            let str_addr1 = "ffff0000fe200004 246c0";
            let str_addr2 = "ffff0000fe2000e4 55000000";
            let str_addr3 = "ffff0000fe201a24 1A";
            let str_addr4 = "ffff0000fe201a28 3";
            let str_addr5 = "ffff0000fe201a2c 70";
            let str_addr6 = "ffff0000fe201a30 301";
            //调用str写入函数
            // do_str(str_addr0);
            do_str(str_addr1);
            do_str(str_addr2);
            do_str(str_addr3);
            do_str(str_addr4);
            do_str(str_addr5);
            do_str(str_addr6);
        }
        Ok(2) => {
            //UART2 ON GPIO 0-3(actualally, operate 0,1)
            //WHICH MEANS SHOULD USE gpiosel0,offset = 0, addr =0xfe200000
            //todo solve issue: no interact

            //this line just for keep UART5
            let str_addr0 = "ffff0000fe200000 1B";
            //use bit 0b0010 0100 0000 0001 1011 = 2401B
            let str_addr1 = "ffff0000fe200000 2401B";

            //bit = 0b01_01_00_00_00_00_00_00_00_00_00_00_00_00_01_01 = 50000005
            //        15 14 13 12 11 10 09 08 07 06 05 04 03 02 01 00
            let str_addr2 = "ffff0000fe2000e4 50000005";

            //also to keep BIRD = 115200
            let str_addr3 = "ffff0000fe201a24 1A";
            let str_addr4 = "ffff0000fe201a28 3";
            //no need to modify
            let str_addr5 = "ffff0000fe201a2c 70";
            let str_addr6 = "ffff0000fe201a30 301";
            //调用str写入函数
            do_str(str_addr0);
            do_str(str_addr1);
            do_str(str_addr2);
            do_str(str_addr3);
            do_str(str_addr4);
            do_str(str_addr5);
            do_str(str_addr6);
        }
        Err(err) => println!("error while convert uart num:{err}"),
        _ => {}
    }
}

fn do_test(args: &str) {
    fn delay(seconds: u64) {
        for i in 1..seconds + 1 {
            fn fibonacci_recursive(n: u64) -> u64 {
                if n == 0 {
                    return 0;
                }
                if n == 1 {
                    return 1;
                }
                return fibonacci_recursive(n - 1) + fibonacci_recursive(n - 2);
            }
            fibonacci_recursive(36 + (i % 2));
        }
    }
    if args == "run" {
        loop {
            let arges = "ffff0000fe201a00 41";
            do_str(arges);
            delay(4);
        }
    }
}

//brokkkkkken key board
use arm_pl011::pl011::Pl011Uart;
fn do_m0ve(args: &str) {
    let uart_base = 0xffff_0000_fe20_1a00 as *mut u8;
    let mut uart = Pl011Uart::new(uart_base);

    let mut args = args.split_ascii_whitespace();

    fn delay_s_or_never(delay_s: u64) {
        if delay_s == 0 {
            return;
        };
        for i in 1..delay_s + 1 {
            println!("{} ", i);

            fn fibonacci_recursive(n: u64) -> u64 {
                if n == 0 {
                    return 0;
                }
                if n == 1 {
                    return 1;
                }
                return fibonacci_recursive(n - 1) + fibonacci_recursive(n - 2);
            }

            fibonacci_recursive(34 + (i % 2));
        }
        do_m0ve("s");
    }

    while let Some(arg) = args.next() {
        let chars = arg
            .matches(char::is_alphabetic)
            .next()
            .expect("err at arg format");
        //default: stop after 1s
        //if 0: never stop until stop arg
        let delay_s =
            u64::from_str_radix(arg.matches(char::is_numeric).next().unwrap_or("1"), 10).unwrap();
        match chars {
            "f" => {
                println!("forward, stop after{}s", delay_s);
                {
                    // 前进：0xff_fc_07_11_01_01_64_00_7e
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x07);
                    uart.putchar(0x11);
                    uart.putchar(0x01);
                    uart.putchar(0x01);
                    uart.putchar(0x64);
                    uart.putchar(0x00);
                    uart.putchar(0x7e);
                }
                delay_s_or_never(delay_s);
            }
            "r" => {
                println!("turn right, stop after{}s", delay_s);
                {
                    // 右转：0xff_fc_07_11_01_06_64_00_83
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x07);
                    uart.putchar(0x11);
                    uart.putchar(0x01);
                    uart.putchar(0x06);
                    uart.putchar(0x64);
                    uart.putchar(0x00);
                    uart.putchar(0x83);
                }
                delay_s_or_never(delay_s);
            }
            "b" => {
                println!("turn back, stop after{}s", delay_s);
                {
                    // 后退：0xff_fc_07_11_01_02_64_00_7f
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x07);
                    uart.putchar(0x11);
                    uart.putchar(0x01);
                    uart.putchar(0x02);
                    uart.putchar(0x64);
                    uart.putchar(0x00);
                    uart.putchar(0x7f);
                }
                delay_s_or_never(delay_s);
            }
            "l" => {
                println!("turn left, stop after{}s", delay_s);
                {
                    // 右转：0xff_fc_07_11_01_05_64_00_82
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x07);
                    uart.putchar(0x11);
                    uart.putchar(0x01);
                    uart.putchar(0x05);
                    uart.putchar(0x64);
                    uart.putchar(0x00);
                    uart.putchar(0x82);
                }
                delay_s_or_never(delay_s);
            }
            "a" => {
                println!("shift left, stop after{}s", delay_s);
                {
                    // 向左平移：0xff_fc_07_11_01_03_64_00_80
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x07);
                    uart.putchar(0x11);
                    uart.putchar(0x01);
                    uart.putchar(0x03);
                    uart.putchar(0x64);
                    uart.putchar(0x00);
                    uart.putchar(0x80);
                }
                delay_s_or_never(delay_s);
            }
            "d" => {
                println!("shift right, stop after{}s", delay_s);
                {
                    // 向右平移：0xff_fc_07_11_01_04_64_00_81
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x07);
                    uart.putchar(0x11);
                    uart.putchar(0x01);
                    uart.putchar(0x04);
                    uart.putchar(0x64);
                    uart.putchar(0x00);
                    uart.putchar(0x81);
                }
                delay_s_or_never(delay_s);
            }
            "s" => {
                println!("stop");
                {
                    // 停止：0xff_fc_07_11_01_00_00_00_19
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x07);
                    uart.putchar(0x11);
                    uart.putchar(0x01);
                    uart.putchar(0x00);
                    uart.putchar(0x00);
                    uart.putchar(0x00);
                    uart.putchar(0x19);
                }
            }
            "n" => {
                println!("beep!");
                {
                    // 鸣笛：0xFF_FC_05_02_60_00_67
                    uart.putchar(0xff);
                    uart.putchar(0xfc);
                    uart.putchar(0x05);
                    uart.putchar(0x02);
                    uart.putchar(0x60);
                    uart.putchar(0x00);
                    uart.putchar(0x67);
                }
            }
            _ => println!("move: argument err"),
        }
    }
}

use axdriver;
fn pci_test(arg: &str) {
    let mut devices = axdriver::init_drivers();
    devices.probe()
}
