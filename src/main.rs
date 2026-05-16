mod app;
mod grid;
mod input;
mod platform;
mod renderer;
mod terminal;
mod theme;
mod window;

use tracing::info;
use tracing_subscriber::{fmt::format::FmtSpan, EnvFilter};
use winit::event_loop::EventLoop;

use app::{App, UserEvent};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .init();

    info!("Myco starting");

    let event_loop = EventLoop::<UserEvent>::with_user_event().build().unwrap();
    let proxy = event_loop.create_proxy();
    let mut app = App::new(proxy);
    event_loop.run_app(&mut app).unwrap();
}
