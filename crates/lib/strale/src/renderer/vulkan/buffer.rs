use ash::vk;
use gpu_allocator::{
    vulkan::{AllocationCreateDesc, Allocator},
    MemoryLocation,
};

use super::device::Device;

#[derive(Copy, Clone, Debug)]
pub struct BufferDesc {
    pub size: usize,
    pub usage: vk::BufferUsageFlags,
    pub memory_location: MemoryLocation,
}

pub struct Buffer {
    pub raw: vk::Buffer,
    pub desc: BufferDesc,
    pub allocation: gpu_allocator::vulkan::Allocation,
}

impl Device {
    pub fn internal_create_buffer(
        device: &Device,
        desc: BufferDesc,
        allocator: &mut Allocator,
        name: impl Into<String>,
    ) -> Buffer {
        let buffer_info = vk::BufferCreateInfo {
            size: desc.size as u64,
            usage: desc.usage,
            sharing_mode: vk::SharingMode::EXCLUSIVE,
            ..Default::default()
        };

        let buffer = unsafe {
            device
                .raw
                .create_buffer(&buffer_info, None)
                .expect("Failed to create buffer")
        };

        let requirements = unsafe { device.raw.get_buffer_memory_requirements(buffer) };

        let allocation = allocator
            .allocate(&AllocationCreateDesc {
                name: &name.into(),
                linear: true,
                location: desc.memory_location,
                requirements,
            })
            .expect("Failed to allocate buffer");

        unsafe {
            device
                .raw
                .bind_buffer_memory(buffer, allocation.memory(), allocation.offset())
                .expect("couldnt bind buffer memory");
        }

        Buffer {
            raw: buffer,
            desc,
            allocation,
        }
    }

    pub fn create_buffer(
        &self,
        mut desc: BufferDesc,
        name: impl Into<String>,
        initial_data: Option<&[u8]>,
    ) -> Buffer {
        if initial_data.is_some() {
            desc.usage |= vk::BufferUsageFlags::TRANSFER_DST;
        }

        let buffer = Self::internal_create_buffer(
            &self,
            desc,
            &mut self.global_allocator.lock().unwrap(),
            name,
        );

        if let Some(initial_data) = initial_data {
            let empty_desc = BufferDesc {
                size: desc.size,
                usage: vk::BufferUsageFlags::TRANSFER_SRC,
                memory_location: MemoryLocation::CpuToGpu,
            };

            let mut empty_buffer = Self::internal_create_buffer(
                &self,
                empty_desc,
                &mut self.global_allocator.lock().unwrap(),
                "empty buffer",
            );

            empty_buffer
                .allocation
                .mapped_slice_mut()
                .expect("memory not host visible")[0..initial_data.len()]
                .copy_from_slice(initial_data);

            let cb = self.setup_cb.lock().unwrap();

            unsafe {
                self.raw
                    .begin_command_buffer(
                        cb.raw,
                        &vk::CommandBufferBeginInfo::builder()
                            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT),
                    )
                    .unwrap();

                self.raw.cmd_copy_buffer(
                    cb.raw,
                    empty_buffer.raw,
                    buffer.raw,
                    &[vk::BufferCopy {
                        src_offset: 0,
                        dst_offset: 0,
                        size: desc.size as u64,
                    }],
                );

                self.raw.end_command_buffer(cb.raw).unwrap();

                let submit_info =
                    vk::SubmitInfo::builder().command_buffers(std::slice::from_ref(&cb.raw));

                self.raw
                    .queue_submit(
                        self.universal_queue.raw,
                        &[submit_info.build()],
                        vk::Fence::null(),
                    )
                    .expect("queue submit failed.");

                self.raw.device_wait_idle();
            };
        }

        buffer
    }
}
