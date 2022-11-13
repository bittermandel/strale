use std::{ffi::CStr, io::Cursor, mem::align_of, sync::Arc};

use ash::{
    util::{read_spv, Align},
    vk::{self, Extent2D, GraphicsPipelineCreateInfo, PipelineViewportStateCreateInfo, Rect2D},
};
use bytemuck::offset_of;

use crate::renderer::{
    utils::get_memory_type_index,
    vulkan::device::{CommandBuffer, Device},
};

#[derive(Clone, Debug, Copy, Default)]
struct Vertex {
    pos: [f32; 4],
    color: [f32; 4],
}

pub unsafe fn vertex_buffer(device: &Device) -> vk::Buffer {
    let vertex_input_buffer_info = vk::BufferCreateInfo {
        size: 3 * std::mem::size_of::<Vertex>() as u64,
        usage: vk::BufferUsageFlags::VERTEX_BUFFER,
        sharing_mode: vk::SharingMode::EXCLUSIVE,
        ..Default::default()
    };

    let vertex_input_buffer = device
        .raw
        .create_buffer(&vertex_input_buffer_info, None)
        .unwrap();

    let vertex_input_buffer_memory_req = device
        .raw
        .get_buffer_memory_requirements(vertex_input_buffer);

    let vertex_input_buffer_memory_index = get_memory_type_index(
        &device.physical_device.memory_properties,
        0xffff_ffff,
        vk::MemoryPropertyFlags::HOST_VISIBLE | vk::MemoryPropertyFlags::HOST_COHERENT,
    )
    .expect("Unable to find suitable memorytype for the vertex buffer.");

    let vertex_buffer_allocate_info = vk::MemoryAllocateInfo {
        allocation_size: vertex_input_buffer_memory_req.size,
        memory_type_index: vertex_input_buffer_memory_index,
        ..Default::default()
    };

    let vertex_input_buffer_memory = device
        .raw
        .allocate_memory(&vertex_buffer_allocate_info, None)
        .unwrap();

    let vertices = [
        Vertex {
            pos: [-1.0, 1.0, 0.0, 1.0],
            color: [0.0, 1.0, 0.0, 1.0],
        },
        Vertex {
            pos: [1.0, 1.0, 0.0, 1.0],
            color: [0.0, 0.0, 1.0, 1.0],
        },
        Vertex {
            pos: [0.0, -1.0, 0.0, 1.0],
            color: [1.0, 0.0, 0.0, 1.0],
        },
    ];

    let vert_ptr = device
        .raw
        .map_memory(
            vertex_input_buffer_memory,
            0,
            vertex_input_buffer_memory_req.size,
            vk::MemoryMapFlags::empty(),
        )
        .unwrap();

    let mut vert_align = Align::new(
        vert_ptr,
        align_of::<Vertex>() as u64,
        vertex_input_buffer_memory_req.size,
    );
    vert_align.copy_from_slice(&vertices);
    device.raw.unmap_memory(vertex_input_buffer_memory);
    device
        .raw
        .bind_buffer_memory(vertex_input_buffer, vertex_input_buffer_memory, 0)
        .unwrap();

    return vertex_input_buffer;
}

pub fn render_triangle(device: &Arc<Device>, cb: &CommandBuffer) {
    let mut vertex_spv_file =
        Cursor::new(&include_bytes!("../../../../../../assets/shaders/triangle.vert.spv")[..]);
    let mut frag_spv_file =
        Cursor::new(&include_bytes!("../../../../../../assets/shaders/triangle.frag.spv")[..]);

    let vertex_code =
        read_spv(&mut vertex_spv_file).expect("Failed to read vertex shader spv file");
    let vertex_shader_info = vk::ShaderModuleCreateInfo::builder().code(&vertex_code);

    let frag_code = read_spv(&mut frag_spv_file).expect("Failed to read fragment shader spv file");
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

    let layout_create_info = vk::PipelineLayoutCreateInfo::default();

    let pipeline_layout = unsafe {
        device
            .raw
            .create_pipeline_layout(&layout_create_info, None)
            .unwrap()
    };

    let dynamic_state_info = vk::PipelineDynamicStateCreateInfo::builder()
        .dynamic_states(&[vk::DynamicState::SCISSOR, vk::DynamicState::VIEWPORT])
        .build();

    let vertex_input_binding_descriptions = [vk::VertexInputBindingDescription {
        binding: 0,
        stride: std::mem::size_of::<Vertex>() as u32,
        input_rate: vk::VertexInputRate::VERTEX,
    }];
    let vertex_input_attribute_descriptions = [
        vk::VertexInputAttributeDescription {
            location: 0,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Vertex, pos) as u32,
        },
        vk::VertexInputAttributeDescription {
            location: 1,
            binding: 0,
            format: vk::Format::R32G32B32A32_SFLOAT,
            offset: offset_of!(Vertex, color) as u32,
        },
    ];

    let vertex_input_state_info = vk::PipelineVertexInputStateCreateInfo::builder()
        .vertex_attribute_descriptions(&vertex_input_attribute_descriptions)
        .vertex_binding_descriptions(&vertex_input_binding_descriptions);
    let vertex_input_assembly_state_info = vk::PipelineInputAssemblyStateCreateInfo {
        topology: vk::PrimitiveTopology::TRIANGLE_LIST,
        ..Default::default()
    };

    let viewports = &[vk::Viewport {
        width: 1920.0,
        height: 1080.0,
        ..Default::default()
    }];
    let scissors = &[Rect2D::builder()
        .extent(Extent2D {
            height: 1080,
            width: 1920,
        })
        .build()];

    let graphics = GraphicsPipelineCreateInfo::builder()
        .push_next(&mut pipeline)
        // We describe the formats of attachment images where the colors, depth and/or stencil
        // information will be written. The pipeline will only be usable with this particular
        // configuration of the attachment images.
        // We need to indicate the layout of the vertices.
        .vertex_input_state(&vertex_input_state_info)
        .input_assembly_state(&vertex_input_assembly_state_info)
        // Use a resizable viewport set to draw over the entire window
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
        // Now that our builder is filled, we call `build()` to obtain an actual pipeline.
        .rasterization_state(
            &vk::PipelineRasterizationStateCreateInfo::builder()
                .cull_mode(vk::CullModeFlags::FRONT)
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

    let vertex_input_buffer = unsafe { vertex_buffer(device) };

    unsafe {
        device
            .raw
            .cmd_bind_pipeline(cb.raw, vk::PipelineBindPoint::GRAPHICS, pipeline);
        device
            .raw
            .cmd_bind_vertex_buffers(cb.raw, 0, &[vertex_input_buffer], &[0]);
        device.raw.cmd_set_viewport(cb.raw, 0, viewports);
        device.raw.cmd_set_scissor(cb.raw, 0, scissors);
        device.raw.cmd_draw(cb.raw, 3, 1, 0, 0);
    };
}
