mod bindless_descriptor_set;
mod renderers;
pub mod utils;
mod vertex;
pub mod vulkan;

use std::sync::Arc;

use ash::vk::{self, Rect2D};

use self::{
    bindless_descriptor_set::create_bindless_descriptor_set,
    renderers::triangles::TrianglesPipeline,
    vertex::{Sphere, Vertex},
    vulkan::{
        backend::Backend,
        buffer::{Buffer, BufferDesc},
        device::Device,
        swapchain::Swapchain,
    },
};

pub struct Renderer {
    device: Arc<Device>,
    triangles_pipeline: TrianglesPipeline,
}

impl Renderer {
    pub fn new(backend: &Backend) -> anyhow::Result<Renderer> {
        let bindless_descriptor_set = create_bindless_descriptor_set(backend.device.as_ref());

        let vertices = [
            Vertex {
                position: [-1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [1.0, 1.0, 0.0, 1.0],
            },
            Vertex {
                position: [0.0, -1.0, 0.0, 1.0],
            },
        ];

        let vertex_buffer_size = vertices.len() * std::mem::size_of::<Vertex>();

        let vertex_buffer = unsafe {
            backend.device.create_buffer(
                BufferDesc {
                    size: vertex_buffer_size,
                    usage: vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS
                        | vk::BufferUsageFlags::INDEX_BUFFER
                        | vk::BufferUsageFlags::TRANSFER_DST,
                    memory_location: gpu_allocator::MemoryLocation::GpuOnly,
                },
                "vertex buffer",
                Some(std::slice::from_raw_parts(
                    vertices.as_ptr() as *const u8,
                    vertex_buffer_size,
                )),
            )
        };

        let spheres = [
            Sphere {
                position: [0.0, -1000.0, 0.0],
                radius: 1000.0,
                material: 0 as f32,
                albedo: [0.5, 0.5, 0.5],
            },
            Sphere {
                position: [2.0, 0.2, 0.0],
                radius: 0.2,
                material: 0 as f32,
                albedo: [0.5, 0.5, 0.5],
            },
            Sphere {
                position: [0.0, 1.2, 1.0],
                radius: 1.0,
                material: 0 as f32,
                albedo: [0.5, 0.5, 0.5],
            },
            Sphere {
                position: [1.0, 0.2, 1.0],
                radius: 0.2,
                material: 0 as f32,
                albedo: [0.5, 0.5, 0.5],
            },
        ];

        let sphere_buffer = unsafe {
            backend.device.create_buffer(
                BufferDesc {
                    size: 1024 * 1024 * 1024,
                    usage: vk::BufferUsageFlags::TRANSFER_DST
                        | vk::BufferUsageFlags::STORAGE_BUFFER
                        | vk::BufferUsageFlags::INDEX_BUFFER
                        | vk::BufferUsageFlags::SHADER_DEVICE_ADDRESS,
                    memory_location: gpu_allocator::MemoryLocation::GpuOnly,
                },
                "sphere buffer",
                Some(std::slice::from_raw_parts(
                    spheres.as_ptr() as *const u8,
                    spheres.len() * std::mem::size_of::<Sphere>(),
                )),
            )
        };

        Self::write_descriptor_set_buffer(
            &backend.device.clone(),
            bindless_descriptor_set,
            0,
            &vertex_buffer,
        );
        Self::write_descriptor_set_buffer(
            &backend.device.clone(),
            bindless_descriptor_set,
            1,
            &sphere_buffer,
        );

        let mut triangles_pipeline = TrianglesPipeline::create_pipeline(
            &backend.device,
            backend.swapchain.desc,
            spheres.len(),
        );
        triangles_pipeline
            .inner
            .add_descriptor_set(0, bindless_descriptor_set);

        Ok(Renderer {
            device: backend.device.clone(),
            triangles_pipeline,
        })
    }

    fn write_descriptor_set_buffer(
        device: &Device,
        set: vk::DescriptorSet,
        dst_binding: u32,
        buffer: &Buffer,
    ) {
        let buffer_info = vk::DescriptorBufferInfo::builder()
            .buffer(buffer.raw)
            .range(vk::WHOLE_SIZE)
            .build();

        let write_descriptor_set = vk::WriteDescriptorSet::builder()
            .dst_set(set)
            .descriptor_type(vk::DescriptorType::STORAGE_BUFFER)
            .dst_binding(dst_binding)
            .buffer_info(std::slice::from_ref(&buffer_info))
            .build();

        unsafe {
            device
                .raw
                .update_descriptor_sets(std::slice::from_ref(&write_descriptor_set), &[])
        }
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
                        .extent(swapchain.desc.dims)
                        .offset(vk::Offset2D { x: 0, y: 0 })
                        .build(),
                );

            let viewports = &[vk::Viewport {
                width: swapchain.desc.dims.width as f32,
                height: -(swapchain.desc.dims.height as f32),
                y: swapchain.desc.dims.height as f32,
                ..Default::default()
            }];

            let scissors = &[Rect2D::builder().extent(swapchain.desc.dims).build()];

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

                self.device.raw.cmd_set_viewport(main_cb.raw, 0, viewports);
                self.device.raw.cmd_set_scissor(main_cb.raw, 0, scissors);

                self.triangles_pipeline
                    .inner
                    .bind_pipeline(&self.device, main_cb.raw);

                self.triangles_pipeline
                    .render(&self.device.clone(), main_cb);

                self.device.raw.cmd_end_rendering(main_cb.raw);

                self.device.raw.end_command_buffer(main_cb.raw).unwrap();

                let submit_info = [vk::SubmitInfo::builder()
                    .command_buffers(std::slice::from_ref(&main_cb.raw))
                    .wait_semaphores(std::slice::from_ref(&swapchain_image.acquire_semaphore))
                    .signal_semaphores(std::slice::from_ref(
                        &swapchain_image.rendering_finished_semaphore,
                    ))
                    .wait_dst_stage_mask(&[vk::PipelineStageFlags::FRAGMENT_SHADER])
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
