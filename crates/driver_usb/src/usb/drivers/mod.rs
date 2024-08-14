use alloc::{boxed::Box, sync::Arc, vec::Vec};
use driverapi::USBSystemDriverModuleInstance;
use spinlock::SpinNoIrq;

use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance, USBSystemConfig,
};

use self::driverapi::USBSystemDriverModule;

use super::urb::URB;

pub mod driverapi;

pub struct DriverContainers<'a, O>
where
    O: PlatformAbstractions,
{
    drivers: Vec<Box<dyn USBSystemDriverModule<'a, O>>>,
}

impl<'a, O> DriverContainers<'a, O>
where
    O: PlatformAbstractions,
{
    pub fn new() -> Self {
        DriverContainers {
            drivers: Vec::new(),
        }
    }

    pub fn load_driver(&mut self, mut module: Box<dyn USBSystemDriverModule<'a, O>>) {
        self.drivers.push(module)
    }

    pub fn create_for_device(
        &mut self,
        device: &mut DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
        preparing_list: &mut Vec<Vec<URB<'a, O>>>,
    ) -> Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>> {
        let collect = self
            .drivers
            .iter()
            .filter_map(|module| module.should_active(device, config.clone()))
            .flat_map(|a| a)
            .inspect(|a| {
                let sender = a.clone();
                if let Some(mut prep_list) = a.lock().prepare_for_drive() {
                    prep_list
                        .iter_mut()
                        .for_each(|urb| urb.set_sender(sender.clone()));
                    preparing_list.push(prep_list)
                }
            })
            .collect();
        collect
    }
}
