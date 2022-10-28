use strale::renderer::{backend::Backend, renderer::Renderer};
use vulkano_win::VkSurfaceBuild;
use winit::{
    dpi::{LogicalSize, PhysicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

mod runtime;

fn main() {
    println!("Running Strale");

    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();

    window.set_inner_size(PhysicalSize::new(1920.0, 1080.0));

    let mut backend = Backend::new(window).unwrap();

    let mut renderer = Renderer::new(&backend).unwrap();

    let mut recreate_swapchain = false;

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent {
            event: WindowEvent::CloseRequested,
            ..
        } => {
            *control_flow = ControlFlow::Exit;
        }
        Event::WindowEvent {
            event: WindowEvent::Resized(_),
            ..
        } => {
            recreate_swapchain = true;
        }
        Event::RedrawEventsCleared => {
            println!("rendering");
            renderer.render(&backend).unwrap();
            println!("rendered");
        }
        _ => (),
    });
}
