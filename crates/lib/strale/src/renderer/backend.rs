use std::sync::Arc;

use vulkano::{
    device::{physical::PhysicalDeviceType, DeviceExtensions},
    instance::{Instance, InstanceCreateInfo},
    swapchain::Surface,
    VulkanLibrary,
};
use vulkano_win::create_surface_from_winit;
use winit::window::Window;

use super::{
    device::Device,
    swapchain::{self, Swapchain},
};

pub struct Backend {
    pub device: Arc<Device>,
    pub surface: Arc<Surface<Window>>,
    pub swapchain: Swapchain,
}

impl Backend {
    pub fn new(window: Window) -> anyhow::Result<Self> {
        let library = VulkanLibrary::new().unwrap();
        let required_extensions = vulkano_win::required_extensions(&library);
        let device_extensions = DeviceExtensions {
            khr_swapchain: true,
            ..DeviceExtensions::empty()
        };

        // Now creating the instance.
        let instance = Instance::new(
            library,
            InstanceCreateInfo {
                enabled_extensions: required_extensions,
                // Enable enumerating devices that use non-conformant vulkan implementations. (ex. MoltenVK)
                enumerate_portability: true,
                ..Default::default()
            },
        )
        .unwrap();

        let surface = create_surface_from_winit(window, instance.clone())?;

        let (physical_device, queue_family_index) = instance
            .enumerate_physical_devices()
            .unwrap()
            .filter(|p| {
                // Some devices may not support the extensions or features that your application, or
                // report properties and limits that are not sufficient for your application. These
                // should be filtered out here.
                p.supported_extensions().contains(&device_extensions)
            })
            .filter_map(|p| {
                p.queue_family_properties()
                    .iter()
                    .enumerate()
                    .position(|(i, q)| {
                        // We select a queue family that supports graphics operations. When drawing to
                        // a window surface, as we do in this example, we also need to check that queues
                        // in this queue family are capable of presenting images to the surface.
                        q.queue_flags.graphics
                            && p.surface_support(i as u32, &surface).unwrap_or(false)
                    })
                    // The code here searches for the first queue family that is suitable. If none is
                    // found, `None` is returned to `filter_map`, which disqualifies this physical
                    // device.
                    .map(|i| (p, i as u32))
            })
            .min_by_key(|(p, _)| {
                // We assign a lower score to device types that are likely to be faster/better.
                match p.properties().device_type {
                    PhysicalDeviceType::DiscreteGpu => 0,
                    PhysicalDeviceType::IntegratedGpu => 1,
                    PhysicalDeviceType::VirtualGpu => 2,
                    PhysicalDeviceType::Cpu => 3,
                    PhysicalDeviceType::Other => 4,
                    _ => 5,
                }
            })
            .expect("No suitable physical device found");

        let device = Device::create(physical_device, queue_family_index)?;

        let swapchain = super::swapchain::Swapchain::new(device.clone(), surface.clone())?;

        Ok(Self {
            device,
            surface,
            swapchain,
        })
    }
}
