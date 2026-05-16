use std::sync::Arc;
use tracing::{info, warn};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowId};

use crate::renderer::Renderer;
use crate::theme::Theme;
use crate::window::create_window;

/// Main application state.
///
/// Owns the window, renderer, and theme. Grid and panel fields will be
/// added by Plan 01-02.
///
/// For Plan 01-01, the app renders a solid background color from the theme.
pub struct App {
    window: Option<Arc<Window>>,
    renderer: Option<Renderer>,
    theme: Theme,
}

impl Default for App {
    fn default() -> Self {
        Self {
            window: None,
            renderer: None,
            theme: Theme::default(),
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // Only initialize once
        if self.window.is_some() {
            return;
        }

        info!("Application resumed -- creating window and GPU state");

        let window = create_window(event_loop);

        // Set up custom title bar with traffic lights (D-14)
        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::setup_custom_title_bar(&window);
        }

        let renderer = Renderer::new(window.clone());

        // Re-apply traffic light positioning after renderer init to handle layout settling
        // (Open Question 4 resolution from RESEARCH.md)
        #[cfg(target_os = "macos")]
        {
            crate::platform::macos::setup_custom_title_bar(&window);
        }

        self.window = Some(window);
        self.renderer = Some(renderer);

        info!("Application initialization complete");
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("Close requested -- exiting");
                event_loop.exit();
            }

            WindowEvent::Resized(size) => {
                // Guard against zero dimensions (Pitfall 2 / T-01-01 mitigation)
                if size.width > 0 && size.height > 0 {
                    if let Some(renderer) = &mut self.renderer {
                        renderer.resize(size.width, size.height);
                    }

                    // Re-apply traffic light positioning after resize (Assumption A4)
                    #[cfg(target_os = "macos")]
                    if let Some(window) = &self.window {
                        crate::platform::macos::setup_custom_title_bar(window);
                    }
                }
            }

            WindowEvent::RedrawRequested => {
                if let (Some(renderer), Some(window)) =
                    (&mut self.renderer, &self.window)
                {
                    let size = window.inner_size();
                    let vw = size.width as f32;
                    let vh = size.height as f32;
                    match renderer.render(self.theme.background, &[], vw, vh) {
                        crate::renderer::RenderResult::Ok => {}
                        crate::renderer::RenderResult::SkipFrame => {}
                        crate::renderer::RenderResult::SurfaceLost => {
                            warn!("Surface lost -- will attempt recovery next frame");
                        }
                    }
                }
            }

            _ => {}
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}
