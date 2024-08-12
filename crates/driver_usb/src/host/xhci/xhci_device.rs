use core::{
    fmt::Error,
    mem::size_of,
    ops::DerefMut,
    ptr::{slice_from_raw_parts, slice_from_raw_parts_mut},
    time::Duration,
};

use alloc::{
    borrow::ToOwned,
    boxed::Box,
    collections::BTreeSet,
    format,
    sync::Arc,
    vec::{self, *},
};
use axhal::time::{busy_wait, busy_wait_until};
use axtask::sleep;
use log::{debug, error, info};
use num_derive::FromPrimitive;
use num_traits::{ops::mul_add, FromPrimitive, ToPrimitive};
use spinlock::SpinNoIrq;
use xhci::{
    context::{Endpoint, EndpointType, Input64Byte, InputHandler},
    ring::{
        self,
        trb::{
            command::{self, ConfigureEndpoint, EvaluateContext},
            event::Allowed,
            transfer::{self, TransferType},
        },
    },
};

use crate::{
    ax::{USBDeviceDriverOps, USBHostDriverOps},
    dma::DMA,
    err::{self, Result},
    host::{
        usb::{
            define::*,
            descriptors::{
                self, desc_configuration, desc_device, desc_endpoint, desc_hid,
                desc_interface::{self, Interface},
                Descriptor, DescriptorType,
            },
        },
        Controller, ControllerArc, PortSpeed,
    },
    OsDep,
};

const TAG: &str = "[XHCI DEVICE]";

use super::{
    event::{self, Ring},
    Xhci,
};

#[derive(Debug, Default, Clone)]
pub struct DescriptorInterface {
    pub data: desc_interface::Interface,
    pub endpoints: Vec<desc_endpoint::Endpoint>,
}

#[derive(Debug, Default, Clone)]
pub struct DescriptorConfiguration {
    pub data: desc_configuration::Configuration,
    pub interfaces: Vec<DescriptorInterface>,
}

#[derive(Clone)]
pub struct DeviceAttached<O>
where
    O: OsDep,
{
    pub hub: usize,
    pub port_id: usize,
    pub slot_id: usize,
    pub configs: Vec<DescriptorConfiguration>,
    current_config: usize,
    current_interface: usize,
    pub(crate) controller: ControllerArc<O>,
    pub device_desc: descriptors::desc_device::Device,
    os: O,
}

impl<O> DeviceAttached<O>
where
    O: OsDep,
{
    pub(crate) fn new(
        slot: usize,
        hub: usize,
        port_id: usize,
        os: O,
        controller: ControllerArc<O>,
    ) -> Self {
        Self {
            hub,
            port_id,
            slot_id: slot,
            configs: Vec::new(),
            current_config: 0,
            current_interface: 0,
            controller,
            os,
            device_desc: Default::default(),
        }
    }
    pub fn current_config(&self) -> &DescriptorConfiguration {
        &self.configs[self.current_config]
    }

    pub fn current_interface(&self) -> &DescriptorInterface {
        &self.current_config().interfaces[self.current_interface]
    }

    pub fn control_transfer_in(
        &self,
        endpoint: usize,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        len: usize,
    ) -> Result<Vec<u8>> {
        let dci;
        if endpoint == 0 {
            dci = 1;
        } else {
            if endpoint & 0x80 == 0 {
                return Err(err::Error::Param(format!("ep {endpoint:#X} not in!")));
            }
            dci = ep_num_to_dci(endpoint);
        }

        debug!("CTL ep {endpoint:#X}, dci {}", dci);

        let mut ctl = self.controller.lock();
        let transfer_type = if len > 0 {
            TransferType::In
        } else {
            TransferType::No
        };
        let buffer = if len > 0 {
            Some(DMA::new_vec(0u8, len, 64, self.os.dma_alloc()))
        } else {
            None
        };

        let data = if let Some(ref buffer) = buffer {
            let mut data = transfer::DataStage::default();
            data.set_data_buffer_pointer(buffer.addr() as u64)
                .set_trb_transfer_length(len as _)
                .set_direction(transfer::Direction::In);
            Some(data)
        } else {
            None
        };
        let mut setup = transfer::SetupStage::default();
        setup
            .set_request_type(request_type)
            .set_request(request)
            .set_value(value)
            .set_index(index)
            .set_length(len as _)
            .set_transfer_type(transfer_type);

        debug!("{:#?}", setup);

        let mut status = transfer::StatusStage::default();

        status.set_interrupt_on_completion();

        ctl.post_transfer(setup, data, status, self, dci as _)?;

        Ok(match &buffer {
            Some(b) => b.to_vec(),
            None => Vec::new(),
        })
    }

    pub fn control_transfer_out(
        &self,
        endpoint: usize,
        request_type: u8,
        request: u8,
        value: u16,
        index: u16,
        data: &[u8],
    ) -> Result {
        let len = data.len();
        if endpoint & 0x80 > 0 {
            return Err(err::Error::Param(format!("ep {endpoint:#X} not out!")));
        }
        let dci = ep_num_to_dci(endpoint);

        debug!("CTL ep {endpoint:#X}, dci {}", dci);

        let mut ctl = self.controller.lock();
        let transfer_type = if len > 0 {
            TransferType::Out
        } else {
            TransferType::No
        };
        let buffer = if len > 0 {
            let mut buffer = DMA::new_vec(0u8, len, 64, self.os.dma_alloc());
            buffer.copy_from_slice(data);
            Some(buffer)
        } else {
            None
        };

        let data = if let Some(ref buffer) = buffer {
            let mut data = transfer::DataStage::default();
            data.set_data_buffer_pointer(buffer.addr() as u64)
                .set_trb_transfer_length(len as _)
                .set_direction(transfer::Direction::Out);
            Some(data)
        } else {
            None
        };
        let mut setup = transfer::SetupStage::default();
        setup
            .set_request_type(request_type)
            .set_request(request)
            .set_value(value)
            .set_index(index)
            .set_length(len as _)
            .set_transfer_type(transfer_type);

        debug!("{:#?}", setup);

        let mut status = transfer::StatusStage::default();

        status.set_interrupt_on_completion();

        status.set_direction();
        ctl.post_transfer(setup, data, status, self, dci as _)?;

        Ok(())
    }

    pub fn interrupt_in(&self, endpoint: usize, len: usize) -> Result<Vec<u8>> {
        if endpoint & 0x80 == 0 {
            return Err(err::Error::Param(format!("ep {endpoint:#X} not in!")));
        }

        let dci = ep_num_to_dci(endpoint);

        debug!("Itr ep {endpoint:#X}, dci {}", dci);
        let mut data = Vec::with_capacity(len);
        data.resize_with(len, || 0);

        let mut ctl = self.controller.lock();

        ctl.post_transfer_normal(&mut data, self, dci as _)?;

        Ok(data)
    }
    pub fn interrupt_out(&self, endpoint: usize, data: &[u8]) -> Result {
        if endpoint & 0x80 > 0 {
            return Err(err::Error::Param(format!("ep {endpoint:#X} not out!")));
        }

        let dci = ep_num_to_dci(endpoint);

        debug!("Itr ep {endpoint:#X}, dci {}", dci);
        let data = unsafe { &mut *slice_from_raw_parts_mut(data.as_ptr() as *mut u8, data.len()) };

        let mut ctl = self.controller.lock();

        ctl.post_transfer_normal(data, self, dci as _)?;

        Ok(())
    }
    pub fn bulk_in(&self, endpoint: usize, len: usize) -> Result<Vec<u8>> {
        self.interrupt_in(endpoint, len)
    }
    pub fn bulk_out(&self, endpoint: usize, data: &[u8]) -> Result {
        self.interrupt_out(endpoint, data)
    }
    pub fn set_configuration(&self) -> Result {
        let config = self.current_config();
        let config_val = config.data.config_val() as u16;
        debug!("set configuration {}", config_val);

        self.control_transfer_out(
            0,
            ENDPOINT_OUT,
            REQUEST_SET_CONFIGURATION,
            config_val,
            0,
            &[],
        )?;

        Ok(())
    }
    pub fn set_interface(&self) -> Result {
        let interface = self.current_interface();
        let interface_num = interface.data.interface_number as u16;
        let setting = interface.data.alternate_setting as u16;

        debug!("set interface {}", interface_num);

        self.control_transfer_out(
            0,
            ENDPOINT_OUT | RECIPIENT_INTERFACE,
            REQUEST_SET_INTERFACE,
            setting,
            interface_num,
            &[],
        )?;

        Ok(())
    }
    pub fn test_mass_storage(&self) -> Result {
        debug!("test mass storage");
        let endpoint_in = 0x81;
        let endpoint_out = 0x2;

        self.set_configuration()?;
        self.set_interface()?;

        debug!("Reading Max LUN:");

        let lun = self.control_transfer_in(
            0,
            ENDPOINT_IN | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
            BOMS_GET_MAX_LUN,
            0,
            0,
            1,
        )?[0];

        debug!("   Max LUN = {}", lun);
        const INQUIRY_LENGTH: u8 = 0x24;
        let mut cdb = [0u8; 16];
        cdb[0] = 0x12;
        cdb[4] = INQUIRY_LENGTH;

        self.send_mass_storage_command(endpoint_out, lun, &cdb, ENDPOINT_IN, INQUIRY_LENGTH as _)?;

        self.get_mass_storage_status(endpoint_in)?;

        Ok(())
    }
    pub fn test_hid(&self) -> Result {
        info!("test begin");
        let endpoint_in = 0x81;

        self.set_configuration().unwrap();
        self.set_interface().unwrap();

        info!("reading HID report descriptors");

        let data = self
            .control_transfer_in(
                0,
                ENDPOINT_IN | REQUEST_TYPE_STANDARD | RECIPIENT_INTERFACE,
                REQUEST_GET_DESCRIPTOR,
                DescriptorType::HIDReport.forLowBit(0).bits(),
                0,
                256,
            )
            .unwrap();
        let descriptor_size = 256;
        info!("descriptor_size {}", descriptor_size);

        let size = get_hid_record_size(&data, HID_REPORT_TYPE_FEATURE);
        if size <= 0 {
            info!("Skipping Feature Report readout (None detected)");
        } else {
            info!("Reading Feature Report (length {})...", size);

            let report_buffer = self
                .control_transfer_in(
                    0,
                    ENDPOINT_IN | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
                    HID_GET_REPORT,
                    (HID_REPORT_TYPE_FEATURE << 8) | 0,
                    0,
                    size as _,
                )
                .unwrap();
        }
        let size = get_hid_record_size(&data, HID_REPORT_TYPE_INPUT);

        if (size <= 0) {
            info!("Skipping Input Report readout (None detected)");
        } else {
            info!("Reading Input Report (length {})...", size);
            let report_buffer = self
                .control_transfer_in(
                    0,
                    ENDPOINT_IN | REQUEST_TYPE_CLASS | RECIPIENT_INTERFACE,
                    HID_GET_REPORT,
                    ((HID_REPORT_TYPE_INPUT << 8) | 0x00),
                    0,
                    size as _,
                )
                .unwrap();

            // Attempt a bulk read from endpoint 0 (this should just return a raw input report)
            info!(
                "Testing interrupt read using endpoint {:#X}...",
                endpoint_in
            );

            // self.controller.lock().fill_transfer_normal_in(size as _, self, ep_num_to_dci(endpoint_in) as _);
            info!("===============================");
            info!("start mouse,please action");
            info!("===============================");
            let mut j = 1;
            loop {
                busy_wait(Duration::from_millis(100));
                let report_buffer = self.interrupt_in(endpoint_in, size as _);
                debug!("----------------------------");
                info!("report_buffer is: {:?}", report_buffer);
                // if report_buffer.iter().any(|b| *b != 0) {
                //     j += 1;
                //     if report_buffer[0] != 0 {
                //         match report_buffer[0] {
                //             1 => {
                //                 info!("left button press");
                //             }
                //             2 => {
                //                 info!("right button press");
                //             }
                //             3 => {
                //                 info!("both button press");
                //             }
                //             4 => {
                //                 info!("middle button press");
                //             }
                //             _ => {}
                //         }
                //     } else {
                //         for i in 1..4 {
                //             if i == 1 {
                //                 if report_buffer[i] & 0b0000_1000 == 0 {
                //                     info!("turn left move {:?} units", report_buffer[i]);
                //                 } else {
                //                     info!("turn right move {:?} units", report_buffer[i]);
                //                 }
                //             } else if i == 2 {
                //                 if report_buffer[i] & 0b0000_1000 == 0 {
                //                     info!("turn down move {:?} units", report_buffer[i]);
                //                 } else {
                //                     info!("turn up move {:?} units", report_buffer[i]);
                //                 }
                //             } else if i == 3 {
                //                 if report_buffer[i] < 128 {
                //                     info!("wheel turn front {:?} units", report_buffer[i]);
                //                 } else if report_buffer[i] > 128 {
                //                     info!("wheel turn back {:?} units", 255 - report_buffer[i]);
                //                 }
                //             }
                //         }
                //     }
                //     //info!("rcv data");
                // }
                // if j == 2 {
                //     loop {}
                // }
            }
        }

        Ok(())
    }

    fn get_mass_storage_status(&self, endpoint: usize) -> Result {
        // The device is allowed to STALL this transfer. If it does, you have to
        // clear the stall and try again.
        let r = self.bulk_in(endpoint, 13)?;

        let status = unsafe { &*(r.as_ptr() as *const CommandStatusWrapper) };

        debug!("{status:#?}");

        Ok(())
    }

    fn send_mass_storage_command(
        &self,
        endpoint: usize,
        lun: u8,
        cdb: &[u8],
        direction: u8,
        data_length: u32,
    ) -> Result {
        let cdb_len = CDB_LENGTH[cdb[0] as usize];
        let mut cbw = CommandBlockWrapper::default();
        let mut tag = 1;

        tag += 1;

        cbw.cbw_signature[0] = b'U';
        cbw.cbw_signature[1] = b'S';
        cbw.cbw_signature[2] = b'B';
        cbw.cbw_signature[3] = b'C';
        cbw.cbw_tag = tag;
        cbw.cbw_data_transfer_length = data_length;
        cbw.cbw_flags = direction;
        cbw.cbw_lun = lun;
        // Subclass is 1 or 6 => cdb_len
        cbw.cbw_cb_length = cdb_len;
        cbw.cbw_cb[..cdb_len as usize].copy_from_slice(&cdb[..cdb_len as usize]);

        let data = unsafe {
            &*slice_from_raw_parts(
                (&cbw) as *const CommandBlockWrapper as *const u8,
                size_of::<CommandBlockWrapper>(),
            )
        };

        // The device is allowed to STALL this transfer. If it does, you have to
        // clear the stall and try again.
        self.bulk_out(endpoint, data)?;
        debug!("  sent {} CDB bytes", cdb_len);

        Ok(())
    }
}

/// HID
fn get_hid_record_size(hid_report_descriptor: &[u8], r#type: u16) -> isize {
    let mut i = hid_report_descriptor[0] as usize + 1;
    let mut j = 0;
    let mut offset = 0;
    let mut record_size = [0, 0, 0];
    let mut nb_bits = 0;
    let mut nb_items = 0;
    let mut found_record_marker = false;

    while i < hid_report_descriptor.len() {
        offset = (hid_report_descriptor[i] & 0x03) as usize + 1;
        if offset == 4 {
            offset = 5;
        }
        match hid_report_descriptor[i] & 0xFC {
            0x74 => {
                // bitsize
                nb_bits = hid_report_descriptor[i + 1] as isize;
            }
            0x94 => {
                // count
                nb_items = 0;
                for j in 1..offset {
                    nb_items = ((hid_report_descriptor[i + j] as u32) << (8 * (j - 1))) as isize;
                }
            }
            0x80 => {
                // input
                found_record_marker = true;
                j = 0;
            }
            0x90 => {
                // output
                found_record_marker = true;
                j = 1;
            }
            0xB0 => {
                // feature
                found_record_marker = true;
                j = 2;
            }
            0xC0 => {
                // end of collection
                nb_items = 0;
                nb_bits = 0;
            }
            _ => {}
        }
        if found_record_marker {
            found_record_marker = false;
            record_size[j as usize] += nb_items * nb_bits;
        }
        i += offset;
    }
    if r#type < HID_REPORT_TYPE_INPUT || r#type > HID_REPORT_TYPE_FEATURE {
        0
    } else {
        (record_size[(r#type - HID_REPORT_TYPE_INPUT) as usize] + 7) / 8
    }
}
