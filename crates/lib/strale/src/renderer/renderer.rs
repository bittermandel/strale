use std::sync::Arc;

use anyhow::Result;
use vulkano::{
    buffer::{BufferUsage, CpuAccessibleBuffer, TypedBufferAccess},
    command_buffer::{
        pool::{standard::StandardCommandPoolBuilder, StandardCommandPool},
        AutoCommandBufferBuilder, CommandBufferUsage, RenderingAttachmentInfo, RenderingInfo,
    },
    image::{view::ImageView, ImageAccess, SwapchainImage},
    memory::pool::StandardMemoryPool,
    pipeline::{graphics::viewport::Viewport, Pipeline},
    render_pass::{LoadOp, StoreOp},
    swapchain::{acquire_next_image, AcquireError, SwapchainCreateInfo, SwapchainCreationError},
    sync::{self, FlushError, GpuFuture},
};
use winit::window::Window;

use super::{backend::Backend, device::Device, renderers::triangle, vertex::Vertex};

pub struct Renderer {
    device: Arc<Device>,
    vertex_buffer: Arc<CpuAccessibleBuffer<[Vertex]>>,
    previous_frame_end: Option<Box<dyn GpuFuture>>,
}

impl Renderer {
    pub fn new(backend: &Backend) -> anyhow::Result<Self> {
        let empty_buffer = CpuAccessibleBuffer::from_iter(
            backend.device.raw.clone(),
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            vec![
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, -1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, -1.0],
                },
                Vertex {
                    position: [1.0, 1.0],
                },
                Vertex {
                    position: [-1.0, 1.0],
                },
            ],
        )?;

        Ok(Self {
            device: backend.device.clone(),
            vertex_buffer: empty_buffer,
            previous_frame_end: None,
        })
    }

    pub fn set_vertices(&mut self, vertices: Vec<Vertex>) -> Result<()> {
        let new_buffer = CpuAccessibleBuffer::from_iter(
            self.device.raw.clone(),
            BufferUsage {
                vertex_buffer: true,
                ..BufferUsage::empty()
            },
            false,
            vertices,
        )?;
        self.vertex_buffer = new_buffer;

        Ok(())
    }

    pub fn render(&mut self, backend: &Backend) -> anyhow::Result<()> {
        let mut builder = AutoCommandBufferBuilder::primary(
            self.device.raw.clone(),
            self.device.queue.queue_family_index(),
            CommandBufferUsage::OneTimeSubmit,
        )
        .unwrap();

        let mut viewport = Viewport {
            origin: [0.0, 0.0],
            dimensions: [0.0, 0.0],
            depth_range: 0.0..1.0,
        };

        let attachment_image_views =
            window_size_dependent_setup(&backend.swapchain.images, &mut viewport);

        let (image_index, _, acquire_future) =
            match acquire_next_image(backend.swapchain.raw.clone(), None) {
                Ok(r) => r,
                Err(e) => panic!("Failed to acquire next image: {:?}", e),
            };

        let (triangle_pipeline, pc) = triangle::render_triangle(backend);

        builder
            .begin_rendering(RenderingInfo {
                color_attachments: vec![Some(RenderingAttachmentInfo {
                    // `Clear` means that we ask the GPU to clear the content of this
                    // attachment at the start of rendering.
                    load_op: LoadOp::Clear,
                    // `Store` means that we ask the GPU to store the rendered output
                    // in the attachment image. We could also ask it to discard the result.
                    store_op: StoreOp::Store,
                    // The value to clear the attachment with. Here we clear it with a
                    // blue color.
                    //
                    // Only attachments that have `LoadOp::Clear` are provided with
                    // clear values, any others should use `None` as the clear value.
                    clear_value: Some([0.0, 0.0, 0.0, 1.0].into()),
                    ..RenderingAttachmentInfo::image_view(
                        // We specify image view corresponding to the currently acquired
                        // swapchain image, to use for this attachment.
                        attachment_image_views[image_index as usize].clone(),
                    )
                })],
                ..Default::default()
            })
            .unwrap()
            .set_viewport(0, [viewport.clone()])
            .bind_pipeline_graphics(triangle_pipeline.clone())
            .bind_vertex_buffers(0, self.vertex_buffer.clone())
            .push_constants(triangle_pipeline.layout().clone(), 0, pc)
            .draw(self.vertex_buffer.len() as u32, 1, 0, 0)
            .unwrap()
            // We leave the render pass.
            .end_rendering()
            .unwrap();

        // Finish building the command buffer by calling `build`.
        let command_buffer = builder.build().unwrap();

        self.previous_frame_end = Some(sync::now(self.device.raw.clone()).boxed());

        let future = self
            .previous_frame_end
            .take()
            .unwrap()
            .join(acquire_future)
            .then_execute(backend.device.queue.clone(), command_buffer)
            .unwrap()
            // The color output is now expected to contain our triangle. But in order to show it on
            // the screen, we have to *present* the image by calling `present`.
            //
            // This function does not actually present the image immediately. Instead it submits a
            // present command at the end of the queue. This means that it will only be presented once
            // the GPU has finished executing the command buffer that draws the triangle.
            .then_swapchain_present(
                backend.device.queue.clone(),
                vulkano::swapchain::PresentInfo::swapchain(backend.swapchain.raw.clone()),
            )
            .then_signal_fence_and_flush();

        println!("waiting for future");

        match future {
            Ok(future) => {
                self.previous_frame_end = Some(future.boxed());
            }
            Err(FlushError::OutOfDate) => {
                self.previous_frame_end = Some(sync::now(self.device.raw.clone()).boxed());
            }
            Err(e) => {
                println!("Failed to flush future: {:?}", e);
                self.previous_frame_end = Some(sync::now(self.device.raw.clone()).boxed());
            }
        }

        Ok(())
    }
}

fn window_size_dependent_setup(
    images: &[Arc<SwapchainImage<Window>>],
    viewport: &mut Viewport,
) -> Vec<Arc<ImageView<SwapchainImage<Window>>>> {
    let dimensions = images[0].dimensions().width_height();
    viewport.dimensions = [dimensions[0] as f32, dimensions[1] as f32];

    images
        .iter()
        .map(|image| ImageView::new_default(image.clone()).unwrap())
        .collect::<Vec<_>>()
}
