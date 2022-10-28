use std::sync::Arc;

use vulkano::device::{
    physical::PhysicalDevice, DeviceCreateInfo, DeviceExtensions, Features, Queue, QueueCreateInfo,
    QueueFlags,
};

pub struct Device {
    pub raw: Arc<vulkano::device::Device>,
    pub queue: Arc<Queue>,
}

impl Device {
    pub fn create(
        physical_device: Arc<PhysicalDevice>,
        queue_index: u32,
    ) -> anyhow::Result<Arc<Self>> {
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        let (device, mut queues) = vulkano::device::Device::new(
            // Which physical device to connect to.
            physical_device,
            DeviceCreateInfo {
                // A list of optional features and extensions that our program needs to work correctly.
                // Some parts of the Vulkan specs are optional and must be enabled manually at device
                // creation. In this example the only thing we are going to need is the `khr_swapchain`
                // extension that allows us to draw to a window.
                enabled_extensions: device_extensions,

                enabled_features: Features {
                    dynamic_rendering: true,
                    ..Features::empty()
                },

                // The list of queues that we are going to use. Here we only use one queue, from the
                // previously chosen queue family.
                queue_create_infos: vec![QueueCreateInfo {
                    queue_family_index: queue_index,
                    ..Default::default()
                }],

                ..Default::default()
            },
        )
        .unwrap();

        Ok(Arc::new(Self {
            raw: device,
            queue: queues.next().unwrap(),
        }))
    }
}
