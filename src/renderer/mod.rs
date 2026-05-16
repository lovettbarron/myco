pub mod gpu_state;
pub mod quad_renderer;
pub mod text_renderer;

use std::sync::Arc;
use tracing::warn;
use winit::window::Window;

use gpu_state::GpuState;
use quad_renderer::{QuadInstance, QuadRenderer};
use text_renderer::{TextEngine, TextLabel};

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
/// Manages the quad renderer for instanced rectangle drawing
/// and the text engine for glyphon-based text rendering.
pub struct Renderer {
    gpu_state: GpuState,
    quad_renderer: QuadRenderer,
    text_engine: TextEngine,
}

impl Renderer {
    /// Create a new renderer with wgpu initialization.
    ///
    /// Uses `pollster::block_on()` for one-time async wgpu init.
    /// This must NOT be called inside the render loop.
    pub fn new(window: Arc<Window>) -> Self {
        let gpu_state = pollster::block_on(GpuState::new(window));
        let quad_renderer = QuadRenderer::new(gpu_state.device(), gpu_state.format());
        let text_engine =
            TextEngine::new(gpu_state.device(), gpu_state.queue(), gpu_state.format());
        Self {
            gpu_state,
            quad_renderer,
            text_engine,
        }
    }

    /// Handle window resize by reconfiguring the GPU surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.gpu_state.resize(width, height);
    }

    /// Render a frame with quads and text labels.
    ///
    /// Quads render first, then text on top, all in a single render pass.
    /// Uses wgpu 29's `CurrentSurfaceTexture` enum (not the old `Result<_, SurfaceError>`).
    pub fn render(
        &mut self,
        clear_color: [f32; 4],
        quads: &[QuadInstance],
        labels: &[TextLabel],
        viewport_width: f32,
        viewport_height: f32,
    ) -> RenderResult {
        // Prepare quad data
        self.quad_renderer.prepare(
            self.gpu_state.device(),
            self.gpu_state.queue(),
            quads,
            viewport_width,
            viewport_height,
        );

        // Prepare text data
        self.text_engine.prepare(
            self.gpu_state.device(),
            self.gpu_state.queue(),
            labels,
            viewport_width as u32,
            viewport_height as u32,
        );

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
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
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
            // Draw quads (panel backgrounds, title bar, etc.)
            self.quad_renderer.render(&mut pass);
            // Draw text ON TOP of quads (labels, breadcrumb, etc.)
            self.text_engine.render(&mut pass);
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
