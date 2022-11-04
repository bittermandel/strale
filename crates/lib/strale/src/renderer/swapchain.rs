use std::sync::Arc;

use vulkano::{
    image::{ImageUsage, SwapchainImage},
    swapchain::{Surface, SwapchainCreateInfo, SwapchainCreationError},
};
use winit::window::Window;

use super::device::Device;

pub struct Swapchain {
    pub raw: Arc<vulkano::swapchain::Swapchain<Window>>,
    pub images: Vec<Arc<SwapchainImage<Window>>>,
}

impl Swapchain {
    pub fn new(device: Arc<Device>, surface: Arc<Surface<Window>>) -> anyhow::Result<Self> {
        let surface_capabilities = device
            .raw
            .physical_device()
            .surface_capabilities(&surface, Default::default())
            .unwrap();

        let image_format = Some(
            device
                .raw
                .physical_device()
                .surface_formats(&surface, Default::default())
                .unwrap()[0]
                .0,
        );

        let window = surface.window();

        let (swapchain, images) = vulkano::swapchain::Swapchain::new(
            device.raw.clone(),
            surface.clone(),
            SwapchainCreateInfo {
                min_image_count: surface_capabilities.min_image_count,

                image_format,
                // The dimensions of the window, only used to initially setup the swapchain.
                // NOTE:
                // On some drivers the swapchain dimensions are specified by
                // `surface_capabilities.current_extent` and the swapchain size must use these
                // dimensions.
                // These dimensions are always the same as the window dimensions.
                //
                // However, other drivers don't specify a value, i.e.
                // `surface_capabilities.current_extent` is `None`. These drivers will allow
                // anything, but the only sensible value is the window
                // dimensions.
                //
                // Both of these cases need the swapchain to use the window dimensions, so we just
                // use that.
                image_extent: window.inner_size().into(),

                image_usage: ImageUsage {
                    color_attachment: true,
                    ..ImageUsage::empty()
                },

                // The alpha mode indicates how the alpha value of the final image will behave. For
                // example, you can choose whether the window will be opaque or transparent.
                composite_alpha: surface_capabilities
                    .supported_composite_alpha
                    .iter()
                    .next()
                    .unwrap(),

                ..Default::default()
            },
        )?;

        Ok(Self {
            raw: swapchain,
            images,
        })
    }

    pub fn recreate(&mut self, window: &Window) {
        let (new_swapchain, new_images) = match self.raw.recreate(SwapchainCreateInfo {
            image_extent: window.inner_size().into(),
            ..self.raw.create_info()
        }) {
            Ok(r) => r,
            // This error tends to happen when the user is manually resizing the window.
            // Simply restarting the loop is the easiest way to fix this issue.
            Err(SwapchainCreationError::ImageExtentNotSupported { .. }) => return,
            Err(e) => panic!("Failed to recreate swapchain: {:?}", e),
        };

        self.raw = new_swapchain;
        self.images = new_images;
    }
}
