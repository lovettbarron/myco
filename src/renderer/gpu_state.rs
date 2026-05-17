use std::sync::Arc;
use tracing::info;
use winit::window::Window;

/// Manages the wgpu device, surface, queue, and configuration.
///
/// Created once during `App::resumed()` via `pollster::block_on(GpuState::new(window))`.
/// Owns the GPU surface lifetime through `Arc<Window>`.
pub struct GpuState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    format: wgpu::TextureFormat,
    #[allow(dead_code)]
    window: Arc<Window>,
}

impl GpuState {
    /// Initialize the wgpu rendering backend.
    ///
    /// Creates an instance with PRIMARY backends (Metal on macOS),
    /// requests an adapter compatible with the window surface,
    /// and configures the surface with sRGB format preference and AutoVsync.
    pub async fn new(window: Arc<Window>) -> Self {
        let mut desc = wgpu::InstanceDescriptor::new_without_display_handle();
        desc.backends = wgpu::Backends::PRIMARY;
        let instance = wgpu::Instance::new(desc);

        let surface = instance.create_surface(window.clone()).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        info!(
            adapter = adapter.get_info().name,
            backend = ?adapter.get_info().backend,
            "GPU adapter selected"
        );

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .unwrap();

        let caps = surface.get_capabilities(&adapter);
        let format = caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(caps.formats[0]);

        let size = window.inner_size();
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: wgpu::CompositeAlphaMode::Auto,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        info!(
            width = config.width,
            height = config.height,
            format = ?format,
            "GPU surface configured"
        );

        Self {
            surface,
            device,
            queue,
            config,
            format,
            window,
        }
    }

    /// Reconfigure the surface after a window resize.
    ///
    /// Guards against zero dimensions (Pitfall 2) which would cause a wgpu panic.
    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.config.width = width;
            self.config.height = height;
            self.surface.configure(&self.device, &self.config);
        }
    }

    /// Access the wgpu device.
    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    /// Access the wgpu queue.
    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    /// The texture format used for the surface.
    pub fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Access the surface for rendering.
    pub fn surface(&self) -> &wgpu::Surface<'static> {
        &self.surface
    }

    /// Current surface width.
    pub fn width(&self) -> u32 {
        self.config.width
    }

    /// Current surface height.
    pub fn height(&self) -> u32 {
        self.config.height
    }
}
