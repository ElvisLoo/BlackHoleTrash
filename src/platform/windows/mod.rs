mod capture_exclusion;
mod cursor_gravity;
mod drop_window;
mod ole_drop_target;
mod recycle_bin;

pub use cursor_gravity::{CursorGravityController, CursorSnapshot, TRAIL_SAMPLES};
pub use drop_window::DropWindow;
pub use recycle_bin::{recycle_async, show_recycle_failure, RecycleResult};

use crossbeam_channel::{Receiver, Sender};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub enum PlatformEvent {
    DragEntered {
        generation: u64,
        paths: Vec<PathBuf>,
        point: [f64; 2],
    },
    DragMoved {
        generation: u64,
        point: [f64; 2],
    },
    DragLeft {
        generation: u64,
    },
    Dropped {
        generation: u64,
        paths: Vec<PathBuf>,
        point: [f64; 2],
    },
    MoveRequested {
        point: [f64; 2],
    },
    RecycleFinished(RecycleResult),
}

pub type EventSender = Sender<PlatformEvent>;
pub type EventReceiver = Receiver<PlatformEvent>;

pub fn event_channel() -> (EventSender, EventReceiver) {
    crossbeam_channel::bounded(128)
}
