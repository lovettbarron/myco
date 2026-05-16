use alacritty_terminal::event::{Event, EventListener};
use std::sync::mpsc;

/// Bridge from alacritty_terminal's background event loop thread to the main UI thread.
///
/// Events are sent via an mpsc channel and drained in the main thread's
/// about_to_wait handler. The main thread polls at ~60fps via WaitUntil.
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
            tracing::debug!("EventListener: channel closed, dropping event: {:?}", e.0);
        }
    }
}
