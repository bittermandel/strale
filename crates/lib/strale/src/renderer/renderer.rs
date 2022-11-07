use std::sync::Arc;

use ash::vk;
use winit::window::Window;

use super::{
    renderers::triangle::render_triangle,
    vulkan::{backend::Backend, device::Device, swapchain::Swapchain},
};

pub struct Renderer {
    device: Arc<Device>,
}

impl Renderer {
    pub fn new(backend: &Backend) -> anyhow::Result<Self> {
        Ok(Self {
            device: backend.device.clone(),
        })
    }

    pub fn draw(&mut self, swapchain: &mut Swapchain) {
        let current_frame = self.device.begin_frame();

        unsafe {
            self.device
                .raw
                .reset_command_buffer(
                    current_frame.main_command_buffer.raw,
                    vk::CommandBufferResetFlags::default(),
                )
                .unwrap();

            self.device
                .raw
                .begin_command_buffer(
                    current_frame.main_command_buffer.raw,
                    &vk::CommandBufferBeginInfo::builder()
                        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                )
                .unwrap();
        }

        // Now we can write to GPU

        let swapchain_image = swapchain
            .acquire_next_image()
            .ok()
            .expect("swapchain image");

        // Record and submit main command buffer
        {
            let main_cb = &current_frame.main_command_buffer;

            vk_sync::cmd::pipeline_barrier(
                &self.device.raw,
                main_cb.raw,
                None,
                &[],
                &[vk_sync::ImageBarrier {
                    discard_contents: false,
                    image: swapchain_image.image.raw,
                    previous_accesses: &[vk_sync::AccessType::Nothing],
                    next_accesses: &[vk_sync::AccessType::ColorAttachmentWrite],
                    next_layout: vk_sync::ImageLayout::Optimal,
                    previous_layout: vk_sync::ImageLayout::Optimal,
                    range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: vk::REMAINING_MIP_LEVELS,
                        base_array_layer: 0,
                        layer_count: vk::REMAINING_ARRAY_LAYERS,
                    },
                    dst_queue_family_index: self.device.universal_queue.family.index,
                    src_queue_family_index: self.device.universal_queue.family.index,
                }],
            );

            // DO SCREEN RENDER STUFF

            // Do CB stuff
            let color_attachment_info = vk::RenderingAttachmentInfo::builder()
                .clear_value(vk::ClearValue {
                    color: vk::ClearColorValue {
                        float32: [0.0, 0.0, 1.0, 0.0],
                    },
                })
                .image_layout(vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL)
                .image_view(swapchain_image.image.view)
                .load_op(vk::AttachmentLoadOp::CLEAR)
                .store_op(vk::AttachmentStoreOp::STORE);

            let render_info = vk::RenderingInfoKHR::builder()
                .color_attachments(std::slice::from_ref(&color_attachment_info))
                .layer_count(1)
                .render_area(
                    vk::Rect2D::builder()
                        .extent(vk::Extent2D {
                            width: 800,
                            height: 600,
                        })
                        .offset(vk::Offset2D { x: 0, y: 0 })
                        .build(),
                );

            vk_sync::cmd::pipeline_barrier(
                &self.device.raw,
                main_cb.raw,
                None,
                &[],
                &[vk_sync::ImageBarrier {
                    discard_contents: false,
                    image: swapchain_image.image.raw,
                    previous_accesses: &[vk_sync::AccessType::ColorAttachmentWrite],
                    previous_layout: vk_sync::ImageLayout::Optimal,
                    next_accesses: &[vk_sync::AccessType::Present],
                    next_layout: vk_sync::ImageLayout::Optimal,
                    range: vk::ImageSubresourceRange {
                        aspect_mask: vk::ImageAspectFlags::COLOR,
                        base_mip_level: 0,
                        level_count: vk::REMAINING_MIP_LEVELS,
                        base_array_layer: 0,
                        layer_count: vk::REMAINING_ARRAY_LAYERS,
                    },
                    dst_queue_family_index: self.device.universal_queue.family.index,
                    src_queue_family_index: self.device.universal_queue.family.index,
                }],
            );

            unsafe {
                self.device
                    .raw
                    .cmd_begin_rendering(main_cb.raw, &render_info);

                render_triangle(&self.device.clone());

                self.device.raw.cmd_end_rendering(main_cb.raw);

                self.device.raw.end_command_buffer(main_cb.raw).unwrap();

                let submit_info = [vk::SubmitInfo::builder()
                    .command_buffers(std::slice::from_ref(&main_cb.raw))
                    .wait_semaphores(std::slice::from_ref(&swapchain_image.acquire_semaphore))
                    .signal_semaphores(std::slice::from_ref(
                        &swapchain_image.rendering_finished_semaphore,
                    ))
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                    .build()];

                self.device
                    .raw
                    .reset_fences(std::slice::from_ref(&main_cb.submit_done_fence))
                    .expect("reset fences");

                self.device
                    .raw
                    .queue_submit(
                        self.device.universal_queue.raw,
                        &submit_info,
                        main_cb.submit_done_fence,
                    )
                    .expect("queue submit failed");
            }

            swapchain.present_image(swapchain_image);
        }

        self.device.finish_frame(current_frame);
    }
}
