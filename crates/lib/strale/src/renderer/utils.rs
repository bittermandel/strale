use ash::vk;

pub fn get_memory_type_index(
    physical_device_memory_properties: &vk::PhysicalDeviceMemoryProperties,
    type_filter: u32,
    property_flags: vk::MemoryPropertyFlags,
) -> Option<u32> {
    for i in 0..physical_device_memory_properties.memory_type_count {
        let mt = &physical_device_memory_properties.memory_types[i as usize];
        if (type_filter & (1 << i)) != 0 && mt.property_flags.contains(property_flags) {
            return Some(i);
        }
    }
    None
}
