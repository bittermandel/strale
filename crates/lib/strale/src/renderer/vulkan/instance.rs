use std::{
    ffi::{CStr, CString},
    os::raw::{c_char, c_void},
    sync::Arc,
};

use anyhow::Result;
use ash::{extensions::ext, vk};

#[derive(Default)]
pub struct DeviceBuilder {
    pub required_extensions: Vec<*const i8>,
    pub graphics_debugging: bool,
}

impl DeviceBuilder {
    pub fn build(self) -> Result<Arc<Instance>> {
        Ok(Arc::new(Instance::create(self)?))
    }

    pub fn required_extensions(mut self, required_extensions: Vec<*const i8>) -> Self {
        self.required_extensions = required_extensions;

        self.required_extensions
            .push(vk::KhrGetPhysicalDeviceProperties2Fn::name().as_ptr());
        #[allow(deprecated)]
        self.required_extensions
            .push(ext::DebugReport::name().as_ptr());
        self.required_extensions
            .push(vk::ExtDebugUtilsFn::name().as_ptr());

        self
    }
}

pub struct Instance {
    pub entry: ash::Entry,
    pub raw: ash::Instance,
}

impl Instance {
    pub fn builder() -> DeviceBuilder {
        DeviceBuilder::default()
    }

    fn create(builder: DeviceBuilder) -> Result<Self> {
        let entry = unsafe { ash::Entry::load()? };

        let required_layer_names: Vec<CString> =
            vec![CString::new("VK_LAYER_KHRONOS_validation").unwrap()];

        let layer_names: Vec<*const i8> = required_layer_names
            .iter()
            .map(|layer_name| layer_name.as_ptr())
            .collect();

        let app_desc = vk::ApplicationInfo::builder().api_version(vk::make_api_version(0, 1, 3, 0));

        let instance_desc = vk::InstanceCreateInfo::builder()
            .application_info(&app_desc)
            .enabled_extension_names(&builder.required_extensions)
            .enabled_layer_names(&layer_names);

        let instance = unsafe { entry.create_instance(&instance_desc, None)? };
        log::info!("Created a Vulkan instance");

        let debug_info = ash::vk::DebugReportCallbackCreateInfoEXT {
            flags: ash::vk::DebugReportFlagsEXT::ERROR
                | ash::vk::DebugReportFlagsEXT::WARNING
                | ash::vk::DebugReportFlagsEXT::PERFORMANCE_WARNING,
            pfn_callback: Some(vulkan_debug_callback),
            ..Default::default()
        };

        #[allow(deprecated)]
        let debug_loader = ext::DebugReport::new(&entry, &instance);

        let _debug_callback = unsafe {
            #[allow(deprecated)]
            debug_loader
                .create_debug_report_callback(&debug_info, None)
                .unwrap()
        };

        let _debug_utils = ash::extensions::ext::DebugUtils::new(&entry, &instance);

        Ok(Self {
            entry,
            raw: instance,
        })
    }
}

unsafe extern "system" fn vulkan_debug_callback(
    _flags: vk::DebugReportFlagsEXT,
    _obj_type: vk::DebugReportObjectTypeEXT,
    _src_obj: u64,
    _location: usize,
    _msg_code: i32,
    _layer_prefix: *const c_char,
    message: *const c_char,
    _user_data: *mut c_void,
) -> u32 {
    let message = CStr::from_ptr(message).to_str().unwrap();

    #[allow(clippy::if_same_then_else)]
    if message.starts_with("Validation Error: [ VUID-VkWriteDescriptorSet-descriptorType-00322")
        || message.starts_with("Validation Error: [ VUID-VkWriteDescriptorSet-descriptorType-02752")
    {
        // Validation layers incorrectly report an error in pushing immutable sampler descriptors.
        //
        // https://www.khronos.org/registry/vulkan/specs/1.2-extensions/man/html/vkCmdPushDescriptorSetKHR.html
        // This documentation claims that it's necessary to push immutable samplers.
    } else if message.starts_with("Validation Performance Warning") {
    } else if message.starts_with("Validation Warning: [ VUID_Undefined ]") {
        log::warn!("{}\n", message);
    } else {
        log::error!("{}\n", message);
    }

    ash::vk::FALSE
}
