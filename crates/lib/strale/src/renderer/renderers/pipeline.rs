use ash::vk;

use crate::renderer::vulkan::device::{CommandBuffer, Device};

#[derive(Default)]
pub struct Pipeline {
    pub bindings: Vec<(u32, vk::DescriptorSet)>,
    pub layout: vk::PipelineLayout,
    pub raw: vk::Pipeline,
    pub descriptor_set_layouts: Vec<vk::DescriptorSetLayout>,
}

impl Pipeline {
    pub fn add_descriptor_set(&mut self, set_idx: u32, descriptor_set: vk::DescriptorSet) {
        self.bindings.push((set_idx, descriptor_set));
    }

    pub fn bind_pipeline(&self, device: &Device, cb: vk::CommandBuffer) {
        unsafe {
            device
                .raw
                .cmd_bind_pipeline(cb, vk::PipelineBindPoint::GRAPHICS, self.raw);
        }

        for (set_idx, descriptor_set) in &self.bindings {
            unsafe {
                device.raw.cmd_bind_descriptor_sets(
                    cb,
                    vk::PipelineBindPoint::GRAPHICS,
                    self.layout,
                    *set_idx,
                    &[*descriptor_set],
                    &[],
                );
            }
        }
    }
}
