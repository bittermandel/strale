use std::{
    collections::HashSet,
    os::raw::c_char,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use anyhow::Result;
use ash::{
    extensions::khr,
    vk::{self},
};
use gpu_allocator::{
    vulkan::{Allocator, AllocatorCreateDesc},
    AllocatorDebugSettings,
};
use log::debug;

use super::{
    instance::Instance,
    physical_device::{PhysicalDevice, QueueFamily},
};

pub struct Queue {
    pub raw: vk::Queue,
    pub family: QueueFamily,
}

pub struct DeviceFrame {
    pub swapchain_acquired_semaphore: Option<vk::Semaphore>,
    pub rendering_complete_semaphore: Option<vk::Semaphore>,
    pub main_command_buffer: CommandBuffer,
}

impl DeviceFrame {
    pub fn new(
        pdevice: &PhysicalDevice,
        device: &ash::Device,
        global_allocator: &mut Allocator,
        queue_family: &QueueFamily,
    ) -> Self {
        Self {
            swapchain_acquired_semaphore: None,
            rendering_complete_semaphore: None,
            main_command_buffer: CommandBuffer::new(device, queue_family).unwrap(),
        }
    }
}

pub struct CommandBuffer {
    pub raw: vk::CommandBuffer,
    pub submit_done_fence: vk::Fence,
    //pool: vk::CommandPool,
}

impl CommandBuffer {
    fn new(device: &ash::Device, queue_family: &QueueFamily) -> Result<Self> {
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(queue_family.index);

        let pool = unsafe { device.create_command_pool(&pool_create_info, None).unwrap() };

        let command_buffer_allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(1)
            .command_pool(pool)
            .level(vk::CommandBufferLevel::PRIMARY);

        let cb = unsafe {
            device
                .allocate_command_buffers(&command_buffer_allocate_info)
                .unwrap()
        }[0];

        let submit_done_fence = unsafe {
            device.create_fence(
                &vk::FenceCreateInfo::builder()
                    .flags(vk::FenceCreateFlags::SIGNALED)
                    .build(),
                None,
            )
        }?;

        Ok(CommandBuffer {
            raw: cb,
            //pool,
            submit_done_fence,
        })
    }
}

pub struct Device {
    pub raw: ash::Device,
    pub physical_device: Arc<PhysicalDevice>,
    pub instance: Arc<Instance>,
    pub universal_queue: Queue,
    pub global_allocator: Arc<Mutex<Allocator>>,
    pub setup_cb: Mutex<CommandBuffer>,
    pub acceleration_structure_ext: khr::AccelerationStructure,
    pub ray_tracing_pipeline_ext: khr::RayTracingPipeline,
    pub ray_tracing_pipeline_properties: vk::PhysicalDeviceRayTracingPipelinePropertiesKHR,
    frames: [Mutex<Arc<DeviceFrame>>; 2],
    pub first_frame: Instant,
}

impl Device {
    pub fn create(physical_device: Arc<PhysicalDevice>) -> anyhow::Result<Arc<Self>> {
        let supported_extensions: HashSet<String> = unsafe {
            let extension_properties = physical_device
                .instance
                .raw
                .enumerate_device_extension_properties(physical_device.raw)?;
            debug!("Extension properties:\n{:#?}", &extension_properties);

            extension_properties
                .iter()
                .map(|ext| {
                    std::ffi::CStr::from_ptr(ext.extension_name.as_ptr() as *const c_char)
                        .to_string_lossy()
                        .as_ref()
                        .to_owned()
                })
                .collect()
        };

        let mut device_extension_names = vec![
            khr::Swapchain::name().as_ptr(),
            khr::DynamicRendering::name().as_ptr(),
        ];

        let ray_tracing_extensions = [
            vk::KhrVulkanMemoryModelFn::name().as_ptr(), // used in ray tracing shaders
            vk::KhrPipelineLibraryFn::name().as_ptr(),   // rt dep
            vk::KhrDeferredHostOperationsFn::name().as_ptr(), // rt dep
            vk::KhrBufferDeviceAddressFn::name().as_ptr(), // rt dep
            vk::KhrAccelerationStructureFn::name().as_ptr(),
            vk::KhrRayTracingPipelineFn::name().as_ptr(),
        ];

        let ray_tracing_enabled = unsafe {
            ray_tracing_extensions.iter().all(|ext| {
                let ext = std::ffi::CStr::from_ptr(*ext).to_string_lossy();

                let supported = supported_extensions.contains(ext.as_ref());

                if !supported {
                    log::info!("Ray tracing extension not supported: {}", ext);
                }

                supported
            })
        };

        if ray_tracing_enabled {
            log::info!("All ray tracing extensions are supported");

            device_extension_names.extend(ray_tracing_extensions.iter());
        }

        unsafe {
            for &ext in &device_extension_names {
                let ext = std::ffi::CStr::from_ptr(ext).to_string_lossy();
                if !supported_extensions.contains(ext.as_ref()) {
                    panic!("Device extension not supported: {}", ext);
                }
            }
        }

        let universal_queue = physical_device
            .queue_families
            .iter()
            .filter(|qf| qf.properties.queue_flags.contains(vk::QueueFlags::GRAPHICS))
            .copied()
            .next();

        let universal_queue = if let Some(universal_queue) = universal_queue {
            universal_queue
        } else {
            anyhow::bail!("No suitable render queue found");
        };

        let universal_queue_info = [vk::DeviceQueueCreateInfo::builder()
            .queue_family_index(universal_queue.index)
            .queue_priorities(&[1.0])
            .build()];

        let mut scalar_block = vk::PhysicalDeviceScalarBlockLayoutFeaturesEXT::default();
        let mut descriptor_indexing = vk::PhysicalDeviceDescriptorIndexingFeaturesEXT::default();
        let mut imageless_framebuffer =
            vk::PhysicalDeviceImagelessFramebufferFeaturesKHR::default();
        let mut shader_float16_int8 = vk::PhysicalDeviceShaderFloat16Int8Features::default();
        let mut vulkan_memory_model = vk::PhysicalDeviceVulkanMemoryModelFeaturesKHR::default();
        let mut get_buffer_device_address_features =
            ash::vk::PhysicalDeviceBufferDeviceAddressFeatures::default();
        let mut acceleration_structure_features =
            ash::vk::PhysicalDeviceAccelerationStructureFeaturesKHR::default();

        let mut ray_tracing_pipeline_features =
            ash::vk::PhysicalDeviceRayTracingPipelineFeaturesKHR::default();

        let mut features13 = vk::PhysicalDeviceVulkan13Features::builder().dynamic_rendering(true);
        let mut features2 = vk::PhysicalDeviceFeatures2::builder()
            .push_next(&mut scalar_block)
            .push_next(&mut descriptor_indexing)
            .push_next(&mut imageless_framebuffer)
            .push_next(&mut shader_float16_int8)
            .push_next(&mut vulkan_memory_model)
            .push_next(&mut get_buffer_device_address_features);

        let device_create_info = vk::DeviceCreateInfo::builder()
            .queue_create_infos(&universal_queue_info)
            .enabled_extension_names(&device_extension_names)
            .push_next(&mut features2)
            .push_next(&mut features13)
            .build();

        unsafe {
            let instance = &physical_device.instance.raw;
            let device = physical_device
                .instance
                .raw
                .create_device(physical_device.raw, &device_create_info, None)
                .unwrap();
            log::info!("Created a Vulkan device");

            let mut global_allocator = Allocator::new(&AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device: physical_device.raw,
                debug_settings: AllocatorDebugSettings {
                    log_leaks_on_shutdown: false,
                    log_memory_information: true,
                    log_allocations: true,
                    ..Default::default()
                },
                buffer_device_address: true,
            })?;

            let universal_queue = Queue {
                raw: device.get_device_queue(universal_queue.index, 0),
                family: universal_queue,
            };

            let frame0 = DeviceFrame::new(
                &physical_device.clone(),
                &device,
                &mut global_allocator,
                &universal_queue.family,
            );
            let frame1 = DeviceFrame::new(
                &physical_device.clone(),
                &device,
                &mut global_allocator,
                &universal_queue.family,
            );

            let setup_cb = CommandBuffer::new(&device, &universal_queue.family).unwrap();

            let acceleration_structure_ext =
                khr::AccelerationStructure::new(&physical_device.instance.raw, &device);

            let ray_tracing_pipeline_ext =
                khr::RayTracingPipeline::new(&physical_device.instance.raw, &device);

            let ray_tracing_pipeline_properties = khr::RayTracingPipeline::get_properties(
                &physical_device.instance.raw,
                physical_device.raw,
            );
            Ok(Arc::new(Device {
                physical_device: physical_device.clone(),
                instance: physical_device.instance.clone(),
                raw: device.clone(),
                universal_queue,
                global_allocator: Arc::new(Mutex::new(global_allocator)),
                setup_cb: Mutex::new(setup_cb),
                acceleration_structure_ext,
                ray_tracing_pipeline_ext,
                // ray_query_ext,
                ray_tracing_pipeline_properties,
                frames: [Mutex::new(Arc::new(frame0)), Mutex::new(Arc::new(frame1))],
                first_frame: Instant::now(),
            }))
        }
    }

    pub fn begin_frame(&self) -> Arc<DeviceFrame> {
        let mut frame0 = self.frames[0].lock().unwrap();
        {
            let frame0: &mut DeviceFrame = Arc::get_mut(&mut frame0).unwrap_or_else(|| {
                panic!("Unable to begin frame: frame data is being held by user code")
            });

            unsafe {
                self.raw
                    .wait_for_fences(
                        &[frame0.main_command_buffer.submit_done_fence],
                        true,
                        std::u64::MAX,
                    )
                    .unwrap();
            }
        }

        frame0.clone()
    }

    pub fn finish_frame(&self, frame: Arc<DeviceFrame>) {
        drop(frame);

        let mut frame0 = self.frames[0].lock().unwrap();
        let frame0: &mut DeviceFrame = Arc::get_mut(&mut frame0).unwrap();

        {
            let mut frame1 = self.frames[1].lock().unwrap();
            let frame1: &mut DeviceFrame = Arc::get_mut(&mut frame1).unwrap();

            std::mem::swap(frame0, frame1);
        }
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        unsafe {
            log::trace!("device_wait_idle");
            let _ = self.raw.device_wait_idle();
        }
    }
}
