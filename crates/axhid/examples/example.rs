// Adapted from hidapi-rs/examples/dump_descriptors.rs, license MIT

use axhid::report_handler::ReportHandler;
use hidapi::HidApi;
use std::io;

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    match HidApi::new() {
        Ok(api) => {
            let infos: Vec<_> = api.device_list().collect();
            for (i, info) in infos.iter().enumerate() {
                println!(
                    "{}: {} ({:04X}:{:04X} interface {} path {:?}):",
                    i,
                    match info.product_string() {
                        Some(s) => s,
                        _ => "<COULD NOT FETCH>",
                    },
                    info.vendor_id(),
                    info.product_id(),
                    info.interface_number(),
                    info.path()
                );
            }

            println!("Choose a number:");
            let mut line = String::new();
            io::stdin().read_line(&mut line).unwrap();
            let number: usize = line.trim().parse().unwrap();
            let info = infos.get(number).unwrap();

            println!(
                "{} ({:04X}:{:04X} interface {} path {:?}):",
                match info.product_string() {
                    Some(s) => s,
                    _ => "<COULD NOT FETCH>",
                },
                info.vendor_id(),
                info.product_id(),
                info.interface_number(),
                info.path()
            );

            let mut descriptor = vec![0u8; 2048];
            match info.open_device(&api) {
                Ok(dev) => match dev.get_report_descriptor(&mut descriptor) {
                    Ok(length) => {
                        log::debug!("    Descriptor {:X?}", &mut descriptor[..length]);
                        let mut handler = ReportHandler::new(&descriptor[..length]).unwrap();
                        loop {
                            let mut report = vec![0; handler.total_byte_length];
                            match dev.read(&mut report) {
                                Ok(length) => {
                                    log::debug!("    Report {:X?}", &mut report[..length]);
                                    for event in handler.handle(&report[..length]).unwrap() {
                                        log::info!("{:?}", event);
                                    }
                                }
                                Err(err) => {
                                    log::error!("    Failed to read report ({:?})", err);
                                    break;
                                }
                            }
                        }
                    }
                    Err(err) => log::error!("    Failed to retrieve descriptor ({:?})", err),
                },
                Err(err) => log::error!("    Failed to open device ({:?})", err),
            }
        }
        Err(e) => {
            log::error!("Error: {}", e);
        }
    }
}
