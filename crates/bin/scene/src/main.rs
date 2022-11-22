use std::process::exit;

use strale::renderer::{renderer::Renderer, vulkan::backend::Backend};
use winit::{
    dpi::PhysicalSize,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn,
    window::WindowBuilder,
};

mod runtime;

fn main() {
    env_logger::init();
    log::info!("Running Strale");

    let mut event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_resizable(false)
        .with_title("hello-kajiya")
        .with_inner_size(winit::dpi::LogicalSize::new(1920, 1080))
        //.with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)))
        .build(&event_loop)
        .unwrap();

    let mut backend = Backend::new(&window).unwrap();

    let mut renderer = Renderer::new(&backend).unwrap();

    //let mut events = Vec::new();

    let mut running = true;

    while running {
        event_loop.run_return(|event, _, control_flow| {
            *control_flow = ControlFlow::Poll;

            match &event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::CloseRequested => {
                        *control_flow = ControlFlow::Exit;
                        running = false;
                    }
                    _ => {}
                },
                Event::MainEventsCleared => {
                    *control_flow = ControlFlow::Exit;
                }
                _ => (),
            }
        });

        renderer.draw(&mut backend.swapchain);
    }
}
