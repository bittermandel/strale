use std::sync::Arc;

use anyhow::Result;
use ash::{
    extensions::khr,
    vk::{self, ColorSpaceKHR, SwapchainKHR},
};

use super::{device::Device, image::Image, surface::Surface};

#[derive(Clone, Copy, Default)]
pub struct SwapchainDesc {
    pub dims: vk::Extent2D,
}

pub struct SwapchainImage {
    pub image: Arc<Image>,
    pub index: u32,
    pub rendering_finished_semaphore: vk::Semaphore,
    pub acquire_semaphore: vk::Semaphore,
}

pub enum SwapchainAcquireImageErr {
    RecreateFramebuffer,
}

pub struct Swapchain {
    pub fns: khr::Swapchain,
    pub raw: SwapchainKHR,
    pub device: Arc<Device>,
    pub acquire_semaphores: Vec<vk::Semaphore>,
    pub rendering_finished_semaphores: Vec<vk::Semaphore>,
    pub images: Vec<Arc<Image>>,
    pub next_semaphore: usize,
    pub desc: SwapchainDesc,
}

impl Swapchain {
    pub fn new(
        device: &Arc<Device>,
        surface: &Arc<Surface>,
        desc: SwapchainDesc,
    ) -> anyhow::Result<Self> {
        let surface_capabilities = unsafe {
            surface
                .fns
                .get_physical_device_surface_capabilities(device.physical_device.raw, surface.raw)
        }?;

        let mut desired_image_count = 3.max(surface_capabilities.min_image_count);
        if surface_capabilities.max_image_count != 0 {
            desired_image_count = desired_image_count.min(surface_capabilities.max_image_count);
        }

        log::info!("Swapchain image count: {}", desired_image_count);

        let surface_resolution = match surface_capabilities.current_extent.width {
            std::u32::MAX => desc.dims,
            _ => surface_capabilities.current_extent,
        };

        let present_mode = vk::PresentModeKHR::IMMEDIATE;
        log::info!("Presentation mode: {:?}", present_mode);

        let swapchain_create_info = vk::SwapchainCreateInfoKHR::builder()
            .surface(surface.raw)
            .min_image_count(desired_image_count)
            .image_color_space(ColorSpaceKHR::SRGB_NONLINEAR)
            .image_format(vk::Format::B8G8R8A8_UNORM)
            .image_extent(surface_resolution)
            .image_usage(vk::ImageUsageFlags::COLOR_ATTACHMENT)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .pre_transform(surface_capabilities.current_transform)
            .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .image_array_layers(1)
            .build();

        let fns = khr::Swapchain::new(&device.instance.raw, &device.raw);
        let swapchain = unsafe { fns.create_swapchain(&swapchain_create_info, None) }.unwrap();

        let vk_images = unsafe { fns.get_swapchain_images(swapchain) }.unwrap();

        let images: Vec<Arc<Image>> = vk_images
            .into_iter()
            .map(|vk_image| Arc::new(Image::new(device.clone(), vk_image)))
            .collect();

        let acquire_semaphores = (0..images.len())
            .map(|_| {
                unsafe {
                    device
                        .raw
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                }
                .unwrap()
            })
            .collect();

        let rendering_finished_semaphores = (0..images.len())
            .map(|_| {
                unsafe {
                    device
                        .raw
                        .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
                }
                .unwrap()
            })
            .collect();

        Ok(Self {
            fns,
            raw: swapchain,
            device: device.clone(),
            acquire_semaphores,
            rendering_finished_semaphores,
            images,
            next_semaphore: 0,
            desc,
        })
    }

    pub fn acquire_next_image(&mut self) -> Result<SwapchainImage, SwapchainAcquireImageErr> {
        let acquire_semaphore = self.acquire_semaphores[self.next_semaphore];
        let rendering_finished_semaphore = self.rendering_finished_semaphores[self.next_semaphore];

        let present_index = unsafe {
            self.fns.acquire_next_image(
                self.raw,
                std::u64::MAX,
                acquire_semaphore,
                vk::Fence::null(),
            )
        }
        .map(|(val, _)| val as usize);

        match present_index {
            Ok(present_index) => {
                assert_eq!(present_index, self.next_semaphore);

                self.next_semaphore = (self.next_semaphore + 1) % self.images.len();
                Ok(SwapchainImage {
                    image: self.images[present_index].clone(),
                    index: present_index as u32,
                    acquire_semaphore,
                    rendering_finished_semaphore,
                })
            }
            Err(err)
                if err == vk::Result::ERROR_OUT_OF_DATE_KHR
                    || err == vk::Result::SUBOPTIMAL_KHR =>
            {
                Err(SwapchainAcquireImageErr::RecreateFramebuffer)
            }
            err => {
                panic!("Could not acquire swapchain image: {:?}", err);
            }
        }
    }

    pub fn present_image(&self, image: SwapchainImage) {
        let present_info = vk::PresentInfoKHR::builder()
            .wait_semaphores(std::slice::from_ref(&image.rendering_finished_semaphore))
            .swapchains(std::slice::from_ref(&self.raw))
            .image_indices(std::slice::from_ref(&image.index));

        unsafe {
            match self
                .fns
                .queue_present(self.device.universal_queue.raw, &present_info)
            {
                Ok(_) => {}
                Err(err)
                    if err == vk::Result::ERROR_OUT_OF_DATE_KHR
                        || err == vk::Result::SUBOPTIMAL_KHR =>
                {
                    // Handled in the next frame
                }
                err => {
                    panic!("could not present image: {:?}", err);
                }
            }
        }
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        unsafe {
            self.fns.destroy_swapchain(self.raw, None);
        }
    }
}
