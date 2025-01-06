use core::error::Error;

use alloc::{
    boxed::Box,
    collections::{btree_map::BTreeMap, btree_set::BTreeSet},
    vec::Vec,
};

use crate::hidreport::{Field, Report, ReportDescriptor};

#[derive(Debug)]
pub struct ReportEvent {
    pub usage_page: u16,
    pub usage: u16,
    pub value: i64,
    pub relative: bool,
}

pub struct ReportHandler {
    pub descriptor: ReportDescriptor,
    pub total_byte_length: usize,
    pub absolutes: BTreeMap<(u16, u16), i64>,
    pub arrays: BTreeSet<(u16, u16)>,
}

impl ReportHandler {
    pub fn new(descriptor_bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        let descriptor = ReportDescriptor::try_from(descriptor_bytes).unwrap();
        let mut total_byte_length = 0;
        for report in descriptor.input_reports() {
            let size = report.size_in_bytes();
            if size > total_byte_length {
                total_byte_length = size;
            }
        }
        Ok(Self {
            descriptor,
            total_byte_length,
            absolutes: BTreeMap::new(),
            arrays: BTreeSet::new(),
        })
    }

    pub fn handle(&mut self, report_bytes: &[u8]) -> Result<Vec<ReportEvent>, Box<dyn Error>> {
        let mut events = Vec::new();
        let mut new_arrays = BTreeSet::new();
        if let Some(report) = self.descriptor.find_input_report(report_bytes) {
            for field in report.fields() {
                match field {
                    Field::Variable(variable) => {
                        let usage_page = variable.usage.usage_page.into();
                        let usage = variable.usage.usage_id.into();
                        let value = {
                            let value = variable.extract(report_bytes).unwrap();
                            if value.is_signed() {
                                i64::from(i32::from(value))
                            } else {
                                i64::from(u32::from(value))
                            }
                        };
                        log::trace!(
                            "{:?}:{:?} = {:?} ({})",
                            usage_page,
                            usage,
                            value,
                            if variable.is_relative {
                                "relative"
                            } else {
                                "absolute"
                            }
                        );
                        if variable.is_relative {
                            if value == 0 {
                                // Skip relative events where value has not changed
                                continue;
                            }
                        } else {
                            if let Some(last_value) = self.absolutes.get(&(usage_page, usage)) {
                                if &value == last_value {
                                    // Skip absolute events where value has not changed
                                    continue;
                                }
                            }
                            // Insert new value
                            self.absolutes.insert((usage_page, usage), value);
                        }

                        events.push(ReportEvent {
                            usage_page,
                            usage,
                            value,
                            relative: variable.is_relative,
                        });
                    }
                    Field::Array(array) => {
                        log::trace!("{:?}", array);
                        //TODO: use array.is_signed?
                        for index in array.extract(&report_bytes).unwrap() {
                            log::trace!("  = {:?}", index);
                            if let Some(usage) = array
                                .usages()
                                .get(usize::try_from(i32::from(index)).unwrap())
                            {
                                new_arrays.insert((
                                    u16::from(usage.usage_page),
                                    u16::from(usage.usage_id),
                                ));
                            }
                        }
                    }
                    Field::Constant(_constant) => {}
                }
            }
        }

        for &(usage_page, usage) in self.arrays.iter() {
            if !new_arrays.contains(&(usage_page, usage)) {
                // Release array items missing from new_arrays
                events.push(ReportEvent {
                    usage_page,
                    usage,
                    value: 0,
                    relative: false,
                });
            }
        }

        for &(usage_page, usage) in new_arrays.iter() {
            if !self.arrays.contains(&(usage_page, usage)) {
                // Press array items only in new_arrays
                events.push(ReportEvent {
                    usage_page,
                    usage,
                    value: 1,
                    relative: false,
                });
            }
        }

        //TODO: more efficient update of array items that does not involve allocation
        self.arrays = new_arrays;

        Ok(events)
    }
}
