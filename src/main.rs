mod app;
mod platform;
mod renderer;
mod theme;
mod window;

use tracing::info;
use tracing_subscriber::EnvFilter;
use winit::event_loop::EventLoop;

use app::App;

fn main() {
    // Initialize structured logging with environment filter.
    // Set RUST_LOG=debug (or info, trace, etc.) to control verbosity.
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    info!("Myco starting");

    let event_loop = EventLoop::new().unwrap();
    let mut app = App::default();
    event_loop.run_app(&mut app).unwrap();
}
