use std::fmt;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::GetLastError,
        UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK},
    },
};
use winit::window::Window;

pub const REQUIRED_AFFINITY: u32 = 0x11;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AffinityObservation {
    SetFailed(u32),
    ReadFailed(u32),
    Reported(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CaptureExclusionError {
    stage: &'static str,
    failure: CaptureExclusionFailure,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum CaptureExclusionFailure {
    WindowHandle(String),
    SetFailed(u32),
    ReadFailed(u32),
    Mismatch(u32),
}

impl CaptureExclusionError {
    fn from_observation(stage: &'static str, observation: AffinityObservation) -> Self {
        let failure = match observation {
            AffinityObservation::SetFailed(code) => CaptureExclusionFailure::SetFailed(code),
            AffinityObservation::ReadFailed(code) => CaptureExclusionFailure::ReadFailed(code),
            AffinityObservation::Reported(actual) => CaptureExclusionFailure::Mismatch(actual),
        };
        Self { stage, failure }
    }

    pub fn user_message(&self) -> String {
        let detail = match &self.failure {
            CaptureExclusionFailure::WindowHandle(detail) => {
                format!("Windows window handle unavailable: {detail}")
            }
            CaptureExclusionFailure::SetFailed(code) => {
                format!("SetWindowDisplayAffinity failed.\nWin32 error: {code}")
            }
            CaptureExclusionFailure::ReadFailed(code) => {
                format!("GetWindowDisplayAffinity failed.\nWin32 error: {code}")
            }
            CaptureExclusionFailure::Mismatch(actual) => format!(
                "Capture exclusion did not match.\nExpected affinity: \
                 0x{REQUIRED_AFFINITY:08X}\nReported affinity: 0x{actual:08X}"
            ),
        };
        format!(
            "Black Hole Trash stopped before recursive screen feedback could start.\n\n\
             Stage: {}\n{}\n\nRequired environment: Windows 10 version 2004 or newer \
             with Desktop Window Manager capture exclusion support. Remote sessions, virtual \
             machines, or graphics drivers may not support this mode.",
            self.stage, detail
        )
    }
}

impl fmt::Display for CaptureExclusionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.user_message())
    }
}

fn validate_affinity(
    stage: &'static str,
    observation: AffinityObservation,
) -> Result<(), CaptureExclusionError> {
    match observation {
        AffinityObservation::Reported(REQUIRED_AFFINITY) => Ok(()),
        observation => Err(CaptureExclusionError::from_observation(stage, observation)),
    }
}

fn window_hwnd(window: &Window, stage: &'static str) -> Result<isize, CaptureExclusionError> {
    let handle = window
        .window_handle()
        .map_err(|error| CaptureExclusionError {
            stage,
            failure: CaptureExclusionFailure::WindowHandle(error.to_string()),
        })?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(handle.hwnd.get()),
        other => Err(CaptureExclusionError {
            stage,
            failure: CaptureExclusionFailure::WindowHandle(format!(
                "expected Win32 handle, got {other:?}"
            )),
        }),
    }
}

fn read_affinity(hwnd: isize, stage: &'static str) -> Result<(), CaptureExclusionError> {
    #[link(name = "user32")]
    extern "system" {
        fn GetWindowDisplayAffinity(hwnd: isize, affinity: *mut u32) -> i32;
    }

    let mut actual = 0;
    if unsafe { GetWindowDisplayAffinity(hwnd, &mut actual) } == 0 {
        return validate_affinity(
            stage,
            AffinityObservation::ReadFailed(unsafe { GetLastError() }.0),
        );
    }
    validate_affinity(stage, AffinityObservation::Reported(actual))
}

pub fn apply_and_verify(window: &Window, stage: &'static str) -> Result<(), CaptureExclusionError> {
    #[link(name = "user32")]
    extern "system" {
        fn SetWindowDisplayAffinity(hwnd: isize, affinity: u32) -> i32;
    }

    let hwnd = window_hwnd(window, stage)?;
    if unsafe { SetWindowDisplayAffinity(hwnd, REQUIRED_AFFINITY) } == 0 {
        return validate_affinity(
            stage,
            AffinityObservation::SetFailed(unsafe { GetLastError() }.0),
        );
    }
    read_affinity(hwnd, stage)
}

pub fn verify(window: &Window, stage: &'static str) -> Result<(), CaptureExclusionError> {
    read_affinity(window_hwnd(window, stage)?, stage)
}

pub fn show_failure_message(message: &str) {
    eprintln!("capture exclusion: {message}");
    let message = HSTRING::from(message);
    unsafe {
        MessageBoxW(
            None,
            &message,
            w!("Black Hole Trash - incompatible capture environment"),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn show_failure(error: &CaptureExclusionError) {
    show_failure_message(&error.user_message());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_exact_exclude_from_capture_value() {
        assert_eq!(
            validate_affinity("initial setup", AffinityObservation::Reported(0x11)),
            Ok(())
        );
    }

    #[test]
    fn rejects_set_failure_with_error_code() {
        let error =
            validate_affinity("initial setup", AffinityObservation::SetFailed(87)).unwrap_err();
        assert_eq!(error.stage, "initial setup");
        assert_eq!(error.failure, CaptureExclusionFailure::SetFailed(87));
    }

    #[test]
    fn rejects_read_failure_with_error_code() {
        let error = validate_affinity("post-show verification", AffinityObservation::ReadFailed(5))
            .unwrap_err();
        assert_eq!(error.failure, CaptureExclusionFailure::ReadFailed(5));
    }

    #[test]
    fn rejects_mismatched_reported_value() {
        let error =
            validate_affinity("monitor rebuild", AffinityObservation::Reported(0x01)).unwrap_err();
        assert_eq!(error.failure, CaptureExclusionFailure::Mismatch(0x01));
    }

    #[test]
    fn mismatch_message_contains_stage_and_values() {
        let error = validate_affinity(
            "post-show verification",
            AffinityObservation::Reported(0x01),
        )
        .unwrap_err();
        let message = error.user_message();
        assert!(message.contains("post-show verification"));
        assert!(message.contains("Expected affinity: 0x00000011"));
        assert!(message.contains("Reported affinity: 0x00000001"));
    }

    #[test]
    fn win32_failure_message_contains_operation_and_code() {
        let error =
            validate_affinity("initial setup", AffinityObservation::SetFailed(87)).unwrap_err();
        let message = error.user_message();
        assert!(message.contains("SetWindowDisplayAffinity failed"));
        assert!(message.contains("Win32 error: 87"));
    }

    #[test]
    fn read_failure_message_contains_operation_and_code() {
        let error = validate_affinity("post-show verification", AffinityObservation::ReadFailed(5))
            .unwrap_err();
        let message = error.user_message();
        assert!(message.contains("post-show verification"));
        assert!(message.contains("GetWindowDisplayAffinity failed"));
        assert!(message.contains("Win32 error: 5"));
    }

    #[test]
    fn unavailable_window_handle_message_contains_stage_and_detail() {
        let error = CaptureExclusionError {
            stage: "monitor rebuild",
            failure: CaptureExclusionFailure::WindowHandle("handle not ready".to_owned()),
        };
        let message = error.user_message();
        assert!(message.contains("monitor rebuild"));
        assert!(message.contains("Windows window handle unavailable"));
        assert!(message.contains("handle not ready"));
    }
}
