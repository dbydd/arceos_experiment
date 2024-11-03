use alloc::{boxed::Box, sync::Arc, vec::Vec};
use drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance};
use log::trace;
use spinlock::SpinNoIrq;
use urb::URB;

use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance, USBSystemConfig,
};

use self::drivers::DriverContainers;
/// This module contains a bunch of code that uses as usb device descriptor decoder, just treat it as black box, you wouldn't want to unserstand its code...
pub mod descriptors;
/// This module contains the USB driver system, which is responsible for managing USB drivers and drived devices.
pub mod drivers;
/// Content in this module would be used in URB, which is responsible for managing USB devices.
pub mod operation;
/// this module contains usb transfers operations, just refer usb spec for more details
pub mod transfer;
///USB Request Block/USB Complete Block and related structures
pub mod urb;

/// This module contains Regular USB drivers, use them as you want, and we always welcome pr.
#[cfg(feature = "packed_drivers")]
pub(crate) mod universal_drivers;

/// The USB driver Layer.
/// contains:
/// * registered USB device drivers
/// * USB device driver instances
pub struct USBDriverSystem<'a, O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    managed_modules: DriverContainers<'a, O>,
    driver_device_instances: Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>,
}

impl<'a, O> USBDriverSystem<'a, O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(config: Arc<SpinNoIrq<USBSystemConfig<O>>>) -> Self {
        Self {
            config,
            managed_modules: DriverContainers::new(),
            driver_device_instances: Vec::new(),
        }
    }

    pub fn init(&mut self) {
        #[cfg(feature = "packed_drivers")]
        {
            self.managed_modules.load_driver(Box::new(
                universal_drivers::hid_drivers::hid_mouse::HidMouseDriverModule,
            ));

            self.managed_modules.load_driver(Box::new(
                universal_drivers::uvc_drivers::generic_uvc::GenericUVCDriverModule,
            ));
        }

        trace!("usb system driver modules load complete!")
    }

    ///this method should invoked after driver independent devices created
    ///
    ///this method should only been executed by [`crate::USBSystem::init_probe()`]
    pub fn init_probe(
        &mut self,
        devices: &mut Vec<DriverIndependentDeviceInstance<O>>,
        preparing_list: &mut Vec<Vec<URB<'a, O>>>,
    ) {
        devices
            .iter_mut()
            .flat_map(|device| {
                self.managed_modules
                    .create_for_device(device, self.config.clone(), preparing_list)
            })
            .collect_into(&mut self.driver_device_instances);
        trace!(
            "current driver managed device num: {}",
            self.driver_device_instances.len()
        )
    }

    ///tick phase: gather all usb driver requests
    pub fn tick(&mut self) -> Vec<Vec<URB<'a, O>>> {
        self.driver_device_instances
            .iter()
            .filter_map(|drv_dev| {
                drv_dev.lock().gather_urb().map(|mut vec| {
                    vec.iter_mut()
                        .for_each(|urb| urb.set_sender(drv_dev.clone()));
                    vec
                })
            })
            .collect()
    }
}
