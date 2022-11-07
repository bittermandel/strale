use std::sync::Arc;

use anyhow::Result;
use ash::vk::{self, PhysicalDeviceMemoryProperties, PhysicalDeviceProperties};

use super::{instance::Instance, surface::Surface};

#[derive(Copy, Clone)]
pub struct QueueFamily {
    pub index: u32,
    pub properties: vk::QueueFamilyProperties,
}

pub struct PhysicalDevice {
    pub instance: Arc<Instance>,
    pub raw: vk::PhysicalDevice,
    pub queue_families: Vec<QueueFamily>,
    pub properties: PhysicalDeviceProperties,
    pub memory_properties: PhysicalDeviceMemoryProperties,
}

pub fn enumerate_physical_devices(instance: &Arc<Instance>) -> Result<Vec<PhysicalDevice>> {
    unsafe {
        let physical_devices = instance.raw.enumerate_physical_devices()?;

        Ok(physical_devices
            .into_iter()
            .map(|pdevice| {
                let properties = instance.raw.get_physical_device_properties(pdevice);

                let queue_families = instance
                    .raw
                    .get_physical_device_queue_family_properties(pdevice)
                    .into_iter()
                    .enumerate()
                    .map(|(index, properties)| QueueFamily {
                        index: index as _,
                        properties,
                    })
                    .collect();

                let memory_properties = instance.raw.get_physical_device_memory_properties(pdevice);

                PhysicalDevice {
                    instance: instance.clone(),
                    raw: pdevice,
                    queue_families,
                    memory_properties,
                    properties,
                }
            })
            .collect())
    }
}

pub trait PhysicalDeviceList {
    fn with_presentation_support(self, surface: &Surface) -> Self;
}

impl PhysicalDeviceList for Vec<PhysicalDevice> {
    fn with_presentation_support(self, surface: &Surface) -> Self {
        self.into_iter()
            .filter_map(|mut pdevice| {
                let supports_presentation =
                    pdevice
                        .queue_families
                        .iter()
                        .enumerate()
                        .any(|(queue_index, info)| unsafe {
                            info.properties
                                .queue_flags
                                .contains(vk::QueueFlags::GRAPHICS)
                                && surface
                                    .fns
                                    .get_physical_device_surface_support(
                                        pdevice.raw,
                                        queue_index as u32,
                                        surface.raw,
                                    )
                                    .unwrap()
                        });
                if supports_presentation {
                    Some(pdevice)
                } else {
                    None
                }
            })
            .collect()
    }
}
