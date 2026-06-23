//! A fake capture source for tests and headless CI (hard rule 5 / capture
//! standard C4): the rest of the app builds and the capture spine runs without
//! macOS, objc2, or any permission.

use std::sync::mpsc::Sender;

use super::{CaptureSource, FocusEvent};

/// Emits a fixed, pre-seeded list of events, then drops the sender.
#[derive(Default)]
pub struct FakeCapture {
    events: Vec<FocusEvent>,
}

impl FakeCapture {
    pub fn new(events: Vec<FocusEvent>) -> Self {
        Self { events }
    }
}

impl CaptureSource for FakeCapture {
    fn start(self: Box<Self>, tx: Sender<FocusEvent>) {
        for ev in self.events {
            if tx.send(ev).is_err() {
                break;
            }
        }
    }
}
