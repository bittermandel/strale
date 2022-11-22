use std::{ffi::CStr, io::Cursor, sync::Arc};

use ash::{
    util::read_spv,
    vk::{self, GraphicsPipelineCreateInfo, PipelineViewportStateCreateInfo, Rect2D},
};
use bytemuck::{Pod, Zeroable};

use crate::renderer::{
    bindless_descriptor_set::create_bindless_descriptor_set_layout,
    vulkan::{
        device::{CommandBuffer, Device},
        swapchain::SwapchainDesc,
    },
};

use super::pipeline::Pipeline;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Zeroable, Pod)]
pub struct TrianglesPushConstant {
    time: f32,
    num_spheres: u32,
}

pub struct TrianglesPipeline {
    pub inner: Pipeline,
    pub num_spheres: u32,
}

impl TrianglesPipeline {
    pub fn create_pipeline(
        device: &Device,
        desc: SwapchainDesc,
        num_spheres: usize,
    ) -> TrianglesPipeline {
        let mut vertex_spv_file =
            Cursor::new(&include_bytes!("../../../../../../assets/shaders/triangle.vert.spv")[..]);
        let mut frag_spv_file =
            Cursor::new(&include_bytes!("../../../../../../assets/shaders/triangle.frag.spv")[..]);

        let vertex_code =
            read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
        let vertex_shader_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_code);

        let frag_code =
            read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
        let frag_shader_info = vk::ShaderModuleCreateInfo::builder().code(&frag_code);

        let vertex_shader_module = unsafe {
            device
                .raw
                .create_shader_module(&vertex_shader_info, None)
                .expect("Vertex shader module error")
        };

        let fragment_shader_module = unsafe {
            device
                .raw
                .create_shader_module(&frag_shader_info, None)
                .expect("Fragment shader module error")
        };

        let descriptor_set_layouts = &[create_bindless_descriptor_set_layout(device)];

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

        let layout_create_info = vk::PipelineLayoutCreateInfo::builder()
            .set_layouts(descriptor_set_layouts)
            .push_constant_ranges(&[vk::PushConstantRange::builder()
                .stage_flags(vk::ShaderStageFlags::FRAGMENT)
                .offset(0)
                .size(std::mem::size_of::<TrianglesPushConstant>() as u32)
                .build()])
            .build();

        let pipeline_layout = unsafe {
            device
                .raw
                .create_pipeline_layout(&layout_create_info, None)
                .unwrap()
        };

        let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder()
            .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT])
            .build();

        let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo {
            vertex_attribute_description_count: 0,
            p_vertex_attribute_descriptions: std::ptr::null(),
            vertex_binding_description_count: 0,
            p_vertex_binding_descriptions: std::ptr::null(),
            ..Default::default()
        };

        let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
            topology: vk::PrimitiveTopology::TRIANGLE_LIST,
            ..Default::default()
        };

        let viewports = &[vk::Viewport {
            width: desc.dims.width as f32,
            height: -(desc.dims.height as f32),
            y: desc.dims.height as f32,
            ..Default::default()
        }];

        let scissors = &[Rect2D::builder().extent(desc.dims).build()];

        let graphics = GraphicsPipelineCreateInfo::builder()
            .push_next(&mut pipeline)
            .vertex_input_state(&vertex_input_state_info)
            .input_assembly_state(&vertex_input_assembly_state_info)
            .viewport_state(
                &PipelineViewportStateCreateInfo::builder()
                    .viewports(viewports)
                    .scissors(scissors)
                    .build(),
            )
            .color_blend_state(&color_blend_state)
            .layout(pipeline_layout)
            .stages(&[
                vk::PipelineShaderStageCreateInfo::builder()
                    .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
                    .stage(vk::ShaderStageFlags::VERTEX)
                    .module(vertex_shader_module)
                    .build(),
                vk::PipelineShaderStageCreateInfo::builder()
                    .name(unsafe { CStr::from_bytes_with_nul_unchecked(b"main\0") })
                    .stage(vk::ShaderStageFlags::FRAGMENT)
                    .module(fragment_shader_module)
                    .build(),
            ])
            .rasterization_state(
                &vk::PipelineRasterizationStateCreateInfo::builder()
                    .cull_mode(vk::CullModeFlags::BACK)
                    .front_face(vk::FrontFace::COUNTER_CLOCKWISE)
                    .line_width(1.0),
            )
            .dynamic_state(&dynamic_state_info)
            .build();

        let pipeline = unsafe {
            device
                .raw
                .create_graphics_pipelines(vk::PipelineCache::null(), &[graphics], None)
                .unwrap()[0]
        };

        TrianglesPipeline {
            inner: Pipeline {
                layout: pipeline_layout,
                raw: pipeline,
                descriptor_set_layouts: descriptor_set_layouts.to_vec(),
                ..Default::default()
            },
            num_spheres: num_spheres as u32,
        }
    }

    pub fn render(&self, device: &Arc<Device>, cb: &CommandBuffer) {
        unsafe {
            device
                .raw
                .cmd_bind_pipeline(cb.raw, vk::PipelineBindPoint::GRAPHICS, self.inner.raw);

            device.raw.cmd_push_constants(
                cb.raw,
                self.inner.layout,
                vk::ShaderStageFlags::FRAGMENT,
                0,
                std::slice::from_raw_parts(
                    [TrianglesPushConstant {
                        time: device.first_frame.elapsed().as_secs_f32(),
                        num_spheres: self.num_spheres,
                    }]
                    .as_ptr() as *const u8,
                    std::mem::size_of::<TrianglesPushConstant>(),
                ),
            );
            device.raw.cmd_draw(cb.raw, 3, 1, 0, 0);
        };
    }
}
