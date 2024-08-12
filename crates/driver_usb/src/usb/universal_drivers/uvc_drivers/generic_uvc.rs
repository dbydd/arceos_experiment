use core::mem::MaybeUninit;

use alloc::{sync::Arc, vec, vec::Vec};
use log::trace;
use spinlock::SpinNoIrq;

use crate::{
    abstractions::PlatformAbstractions,
    glue::driver_independent_device_instance::DriverIndependentDeviceInstance,
    host::data_structures::MightBeInited,
    usb::{
        descriptors::parser::ParserMetaData,
        drivers::driverapi::{USBSystemDriverModule, USBSystemDriverModuleInstance},
    },
    USBSystemConfig,
};

pub struct GenericUVCDriverModule; //TODO: Create annotations to register
pub struct GenericUVCDriver<O>
where
    O: PlatformAbstractions,
{
    config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
}

impl<'a, O> USBSystemDriverModule<'a, O> for GenericUVCDriverModule
where
    O: PlatformAbstractions + 'static,
{
    fn should_active(
        &self,
        independent_dev: &DriverIndependentDeviceInstance<O>,
        config: Arc<SpinNoIrq<crate::USBSystemConfig<O>>>,
    ) -> Option<Vec<Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>>>> {
        if let MightBeInited::Inited(desc) = &*independent_dev.descriptors
            && let ParserMetaData::UVC(_) = desc.metadata
        {
            Some(vec![GenericUVCDriver::new(config.clone())])
        } else {
            None
        }
    }

    fn preload_module(&self) {
        trace!("loaded Generic UVC Driver Module!");
        todo!()
    }
}

impl<'a, O> GenericUVCDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    pub fn new(
        config: Arc<SpinNoIrq<USBSystemConfig<O>>>,
    ) -> Arc<SpinNoIrq<dyn USBSystemDriverModuleInstance<'a, O>>> {
        Arc::new(SpinNoIrq::new(Self { config }))
    }
}

impl<'a, O> USBSystemDriverModuleInstance<'a, O> for GenericUVCDriver<O>
where
    O: PlatformAbstractions + 'static,
{
    fn prepare_for_drive(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        todo!()
    }

    fn gather_urb(&mut self) -> Option<Vec<crate::usb::urb::URB<'a, O>>> {
        todo!()
    }

    fn receive_complete_event(&mut self, ucb: crate::glue::ucb::UCB<O>) {
        todo!()
    }
}
