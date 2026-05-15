pub mod gpu_state;

use std::sync::Arc;
use tracing::warn;
use winit::window::Window;

use gpu_state::GpuState;

/// Represents the outcome of a render attempt.
#[derive(Debug)]
pub enum RenderResult {
    /// Frame rendered successfully.
    Ok,
    /// Surface lost -- needs reconfiguration.
    SurfaceLost,
    /// Timeout or occluded -- skip this frame.
    SkipFrame,
}

/// Renderer orchestrates the GPU render pipeline.
///
/// For Plan 01-01, the renderer simply clears the screen with the theme background color.
/// QuadRenderer and TextEngine integration will be added by Plan 01-02.
pub struct Renderer {
    gpu_state: GpuState,
}

impl Renderer {
    /// Create a new renderer with wgpu initialization.
    ///
    /// Uses `pollster::block_on()` for one-time async wgpu init.
    /// This must NOT be called inside the render loop.
    pub fn new(window: Arc<Window>) -> Self {
        let gpu_state = pollster::block_on(GpuState::new(window));
        Self { gpu_state }
    }

    /// Handle window resize by reconfiguring the GPU surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu_state.resize(width, height);
    }

    /// Render a frame with the given clear color.
    ///
    /// For Plan 01-01, this only clears the screen with `clear_color`.
    /// Plan 01-02 will add quad rendering and text rendering passes.
    ///
    /// Uses wgpu 29's `CurrentSurfaceTexture` enum (not the old `Result<_, SurfaceError>`).
    pub fn render(&mut self, clear_color: [f32; 4]) -> RenderResult {
        let surface_texture = self.gpu_state.surface().get_current_texture();

        let output = match surface_texture {
            wgpu::CurrentSurfaceTexture::Success(tex) => tex,
            wgpu::CurrentSurfaceTexture::Suboptimal(tex) => {
                // Still usable, but reconfigure soon
                tex
            }
            wgpu::CurrentSurfaceTexture::Timeout | wgpu::CurrentSurfaceTexture::Occluded => {
                return RenderResult::SkipFrame;
            }
            wgpu::CurrentSurfaceTexture::Outdated => {
                // Reconfigure and try next frame
                let width = self.gpu_state.width();
                let height = self.gpu_state.height();
                self.gpu_state.resize(width, height);
                return RenderResult::SkipFrame;
            }
            wgpu::CurrentSurfaceTexture::Lost => {
                warn!("Surface lost -- reconfiguring");
                let width = self.gpu_state.width();
                let height = self.gpu_state.height();
                self.gpu_state.resize(width, height);
                return RenderResult::SurfaceLost;
            }
            wgpu::CurrentSurfaceTexture::Validation => {
                warn!("Surface validation error");
                return RenderResult::SkipFrame;
            }
        };

        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder =
            self.gpu_state
                .device()
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("Render Encoder"),
                });

        {
            let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Main Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0] as f64,
                            g: clear_color[1] as f64,
                            b: clear_color[2] as f64,
                            a: clear_color[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            // Plan 01-02: quad_renderer.render(&mut pass);
            // Plan 01-02: text_engine.render(&mut pass);
        }

        self.gpu_state
            .queue()
            .submit(std::iter::once(encoder.finish()));
        output.present();
        RenderResult::Ok
    }

    /// Access the underlying GPU state's device.
    pub fn device(&self) -> &wgpu::Device {
        self.gpu_state.device()
    }

    /// Access the underlying GPU state's queue.
    pub fn queue(&self) -> &wgpu::Queue {
        self.gpu_state.queue()
    }

    /// The texture format of the surface.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.gpu_state.format()
    }
}
