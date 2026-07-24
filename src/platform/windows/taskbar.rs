use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use std::fmt;
use windows::core::{w, HSTRING};
use windows::Win32::Foundation::{GetLastError, SetLastError, HWND, WIN32_ERROR};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, MessageBoxW, SetWindowLongPtrW, SetWindowPos, GWL_EXSTYLE, MB_ICONERROR,
    MB_OK, SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, WS_EX_APPWINDOW,
    WS_EX_TOOLWINDOW,
};
use winit::platform::windows::WindowExtWindows;
use winit::window::Window;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskbarExclusionError(String);

impl TaskbarExclusionError {
    pub fn user_message(&self) -> String {
        format!(
            "Black Hole Trash stopped because its render window could not be hidden from the taskbar.\n\n{}",
            self.0
        )
    }
}

impl fmt::Display for TaskbarExclusionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.user_message())
    }
}

fn excluded_taskbar_style(style: u32) -> u32 {
    (style | WS_EX_TOOLWINDOW.0) & !WS_EX_APPWINDOW.0
}

fn is_taskbar_excluded(style: u32) -> bool {
    style & WS_EX_TOOLWINDOW.0 != 0 && style & WS_EX_APPWINDOW.0 == 0
}

pub fn apply_and_verify(window: &Window) -> Result<(), TaskbarExclusionError> {
    window.set_skip_taskbar(true);
    let hwnd = window_hwnd(window)?;
    let current = read_ex_style(hwnd)?;
    write_ex_style(hwnd, excluded_taskbar_style(current))?;
    unsafe {
        SetWindowPos(
            hwnd,
            None,
            0,
            0,
            0,
            0,
            SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
        )
    }
    .map_err(|error| TaskbarExclusionError(format!("SetWindowPos failed: {error}")))?;
    verify(window)
}

pub fn set_visible_and_verify(window: &Window, visible: bool) -> Result<(), TaskbarExclusionError> {
    // winit rebuilds GWL_EXSTYLE during visibility changes without carrying
    // its skip-taskbar state into that rebuild. Apply before showing to avoid
    // a transient taskbar tab, then apply again after winit rewrites the style.
    apply_and_verify(window)?;
    window.set_visible(visible);
    apply_and_verify(window)
}

fn verify(window: &Window) -> Result<(), TaskbarExclusionError> {
    let style = read_ex_style(window_hwnd(window)?)?;
    if is_taskbar_excluded(style) {
        Ok(())
    } else {
        Err(TaskbarExclusionError(format!(
            "unexpected extended style 0x{style:08X}; WS_EX_TOOLWINDOW must be set and WS_EX_APPWINDOW must be clear"
        )))
    }
}

pub fn show_failure(error: &TaskbarExclusionError) {
    let message = HSTRING::from(error.user_message());
    unsafe {
        MessageBoxW(
            None,
            &message,
            w!("Black Hole Trash - taskbar exclusion failed"),
            MB_OK | MB_ICONERROR,
        );
    }
}

fn window_hwnd(window: &Window) -> Result<HWND, TaskbarExclusionError> {
    let handle = window
        .window_handle()
        .map_err(|error| TaskbarExclusionError(format!("window handle unavailable: {error}")))?;
    match handle.as_raw() {
        RawWindowHandle::Win32(handle) => Ok(HWND(handle.hwnd.get() as *mut std::ffi::c_void)),
        other => Err(TaskbarExclusionError(format!(
            "expected a Win32 window handle, got {other:?}"
        ))),
    }
}

fn read_ex_style(hwnd: HWND) -> Result<u32, TaskbarExclusionError> {
    unsafe {
        SetLastError(WIN32_ERROR(0));
    }
    let value = unsafe { GetWindowLongPtrW(hwnd, GWL_EXSTYLE) };
    let error = unsafe { GetLastError() };
    if value == 0 && error.0 != 0 {
        Err(TaskbarExclusionError(format!(
            "GetWindowLongPtrW failed with Win32 error {}",
            error.0
        )))
    } else {
        Ok(value as u32)
    }
}

fn write_ex_style(hwnd: HWND, style: u32) -> Result<(), TaskbarExclusionError> {
    unsafe {
        SetLastError(WIN32_ERROR(0));
    }
    let previous = unsafe { SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style as isize) };
    let error = unsafe { GetLastError() };
    if previous == 0 && error.0 != 0 {
        Err(TaskbarExclusionError(format!(
            "SetWindowLongPtrW failed with Win32 error {}",
            error.0
        )))
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::UI::WindowsAndMessaging::{
        WS_EX_APPWINDOW, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
    };

    #[test]
    fn taskbar_policy_adds_toolwindow_and_removes_appwindow() {
        let unrelated = WS_EX_NOACTIVATE.0;
        let style = excluded_taskbar_style(WS_EX_APPWINDOW.0 | unrelated);

        assert_ne!(style & WS_EX_TOOLWINDOW.0, 0);
        assert_eq!(style & WS_EX_APPWINDOW.0, 0);
        assert_ne!(style & unrelated, 0);
    }

    #[test]
    fn compliant_style_is_recognized() {
        assert!(is_taskbar_excluded(WS_EX_TOOLWINDOW.0));
        assert!(!is_taskbar_excluded(WS_EX_APPWINDOW.0));
        assert!(!is_taskbar_excluded(WS_EX_TOOLWINDOW.0 | WS_EX_APPWINDOW.0));
    }
}
