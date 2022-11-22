use std::sync::Arc;


use ash::vk::{self};

use super::device::Device;

pub struct Image {
    pub raw: vk::Image,
    pub view: vk::ImageView,
}

impl Image {
    pub fn new(device: Arc<Device>, image: vk::Image) -> Self {
        let create_info = vk::ImageViewCreateInfo::builder()
            .image(image)
            .components(vk::ComponentMapping {
                r: vk::ComponentSwizzle::R,
                g: vk::ComponentSwizzle::G,
                b: vk::ComponentSwizzle::B,
                a: vk::ComponentSwizzle::A,
            })
            .view_type(vk::ImageViewType::TYPE_2D)
            .format(vk::Format::B8G8R8A8_UNORM)
            .subresource_range(vk::ImageSubresourceRange {
                aspect_mask: vk::ImageAspectFlags::COLOR,
                level_count: 1,
                layer_count: 1,
                ..Default::default()
            });

        let view = unsafe { device.raw.create_image_view(&create_info, None).unwrap() };

        Self { raw: image, view }
    }
}
