use alacritty_terminal::event::{Event, EventListener};
use std::sync::mpsc;

/// Bridge from alacritty_terminal's background event loop thread to the main UI thread.
///
/// Implements the EventListener trait required by Term and EventLoop.
/// Events are sent via an mpsc channel and drained in the main thread's
/// about_to_wait handler.
#[derive(Clone)]
pub struct MycoEventListener {
    sender: mpsc::Sender<Event>,
}

impl MycoEventListener {
    pub fn new(sender: mpsc::Sender<Event>) -> Self {
        Self { sender }
    }
}

impl EventListener for MycoEventListener {
    fn send_event(&self, event: Event) {
        if let Err(e) = self.sender.send(event) {
            // Receiver dropped (panel closing) -- expected during teardown
            tracing::debug!("EventListener: channel closed, dropping event: {:?}", e.0);
        }
    }
}
