mod capture_exclusion;
mod ctrl_double_tap;
mod cursor_gravity;
mod drop_window;
mod ole_drop_target;
mod recycle_bin;
mod taskbar;
mod window_occlusion;

pub use capture_exclusion::{
    apply_and_verify as apply_capture_exclusion, show_failure as show_capture_exclusion_failure,
    show_failure_message as show_capture_exclusion_failure_message,
    verify as verify_capture_exclusion,
};
pub use ctrl_double_tap::CtrlDoubleTapController;
pub use cursor_gravity::{CursorGravityController, CursorSnapshot, TRAIL_SAMPLES};
pub use drop_window::DropWindow;
pub use recycle_bin::{recycle_async, show_recycle_failure, RecycleResult};
pub use taskbar::{
    apply_and_verify as apply_taskbar_exclusion,
    set_visible_and_verify as set_taskbar_excluded_visibility,
    show_failure as show_taskbar_exclusion_failure,
};
pub use window_occlusion::is_black_hole_occluded;

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
