use std::sync::Arc;

use ash::vk::{
    self, GraphicsPipelineCreateInfo, PipelineRenderingCreateInfo, PipelineViewportStateCreateInfo,
    RenderPass,
};

use crate::renderer::vulkan::{backend::Backend, device::Device};

pub fn render_triangle(device: &Arc<Device>) -> vk::Pipeline {
    let mut pipeline = vk::PipelineRenderingCreateInfo::builder()
        .color_attachment_formats(&[vk::Format::B8G8R8A8_UNORM]);

    let color_blend_attachment_states = [vk::PipelineColorBlendAttachmentState {
        blend_enable: 0,
        src_color_blend_factor: vk::BlendFactor::SRC_COLOR,
        dst_color_blend_factor: vk::BlendFactor::ONE_MINUS_DST_COLOR,
        color_blend_op: vk::BlendOp::ADD,
        src_alpha_blend_factor: vk::BlendFactor::ZERO,
        dst_alpha_blend_factor: vk::BlendFactor::ZERO,
        alpha_blend_op: vk::BlendOp::ADD,
        color_write_mask: vk::ColorComponentFlags::RGBA,
    }];
    let color_blend_state = vk::PipelineColorBlendStateCreateInfo::builder()
        .logic_op(vk::LogicOp::CLEAR)
        .attachments(&color_blend_attachment_states);

    let graphics = GraphicsPipelineCreateInfo::builder()
        .push_next(&mut pipeline)
        // We describe the formats of attachment images where the colors, depth and/or stencil
        // information will be written. The pipeline will only be usable with this particular
        // configuration of the attachment images.
        // We need to indicate the layout of the vertices.
        .vertex_input_state(&vk::PipelineVertexInputStateCreateInfo::default())
        // The content of the vertex buffer describes a list of triangles.
        .input_assembly_state(&vk::PipelineInputAssemblyStateCreateInfo::default())
        // Use a resizable viewport set to draw over the entire window
        .viewport_state(
            &PipelineViewportStateCreateInfo::builder().viewports(&[vk::Viewport {
                width: 1920.0,
                height: 1080.0,
                ..Default::default()
            }]),
        )
        .color_blend_state(&color_blend_state)
        // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
        .rasterization_state(
            &vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::FRONT)
                .front_face(vk::FrontFace::COUNTER_CLOCKWISE),
        )
        .build();

    unsafe {
        device
            .raw
            .create_graphics_pipelines(vk::PipelineCache::null(), &[graphics], None)
            .unwrap()[0]
    }
}
