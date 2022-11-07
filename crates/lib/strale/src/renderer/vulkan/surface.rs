use std::sync::Arc;

use anyhow::Result;
use ash::{extensions::khr, vk};
use raw_window_handle::{HasRawDisplayHandle, HasRawWindowHandle};
use winit::window::Window;

use super::instance::Instance;

pub struct Surface {
    pub raw: vk::SurfaceKHR,
    pub fns: khr::Surface,
}

impl Surface {
    pub fn create(instance: &Instance, window: &Window) -> Result<Arc<Self>> {
        let surface = unsafe {
            ash_window::create_surface(
                &instance.entry,
                &instance.raw,
                window.raw_display_handle(),
                window.raw_window_handle(),
                None,
            )?
        };
        let surface_loader = khr::Surface::new(&instance.entry, &instance.raw);

        Ok(Arc::new(Self {
            raw: surface,
            fns: surface_loader,
        }))
    }
}
