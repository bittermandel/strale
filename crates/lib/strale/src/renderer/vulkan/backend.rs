use std::{sync::Arc};

use ash::vk;
use raw_window_handle::{HasRawDisplayHandle};
use winit::window::Window;

use crate::renderer::vulkan::swapchain::SwapchainDesc;

use super::{
    device::Device,
    instance::Instance,
    physical_device::{enumerate_physical_devices, PhysicalDeviceList},
    surface::Surface,
    swapchain::{Swapchain},
};

pub struct Backend {
    pub device: Arc<Device>,
    pub surface: Arc<Surface>,
    pub swapchain: Swapchain,
}

impl Backend {
    pub fn new(window: &Window) -> anyhow::Result<Self> {
        // Now creating the instance.
        let instance = Instance::builder()
            .required_extensions(
                ash_window::enumerate_required_extensions(window.raw_display_handle())
                    .unwrap()
                    .to_vec(),
            )
            .build()?;

        log::info!("instance created");

        let surface = Surface::create(&instance, window)?;

        let physical_devices =
            enumerate_physical_devices(&instance)?.with_presentation_support(&surface);

        let physical_device = physical_devices
            .into_iter()
            // If there are multiple devices with the same score, `max_by_key` would choose the last,
            // and we want to preserve the order of devices from `enumerate_physical_devices`.
            .rev()
            .max_by_key(|device| match device.properties.device_type {
                //vk::PhysicalDeviceType::INTEGRATED_GPU => 200,
                vk::PhysicalDeviceType::DISCRETE_GPU => 1000,
                //vk::PhysicalDeviceType::VIRTUAL_GPU => 1,
                _ => 0,
            })
            .unwrap();

        let device = Device::create(Arc::new(physical_device))?;

        let swapchain = super::swapchain::Swapchain::new(
            &device,
            &surface,
            SwapchainDesc {
                dims: vk::Extent2D {
                    height: window.inner_size().height,
                    width: window.inner_size().width,
                },
            },
        )?;

        Ok(Self {
            device,
            surface,
            swapchain,
        })
    }
}
