use super::ole_drop_target::OleDropTarget;
use super::{EventSender, PlatformEvent};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Mutex, OnceLock};
use windows::core::{w, ComObject};
use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{
    IDropTarget, OleInitialize, OleUninitialize, RegisterDragDrop, RevokeDragDrop,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{GetCapture, ReleaseCapture, SetCapture};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, GetCursorPos, GetWindowRect, RegisterClassW,
    SetLayeredWindowAttributes, SetWindowPos, ShowWindow, CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST,
    LWA_ALPHA, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WM_CANCELMODE,
    WM_CAPTURECHANGED, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WNDCLASSW, WS_EX_LAYERED,
    WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

const WINDOW_SIZE: i32 = 88;
static EVENT_SENDER: OnceLock<EventSender> = OnceLock::new();
static DRAG_OFFSET: Mutex<Option<[i32; 2]>> = Mutex::new(None);
static WIDGET_DRAG_ACTIVE: AtomicBool = AtomicBool::new(false);

pub struct DropWindow {
    hwnd: HWND,
    _target: IDropTarget,
    ole_initialized: bool,
    enabled: bool,
}

impl DropWindow {
    pub fn new(sender: EventSender) -> windows::core::Result<Self> {
        EVENT_SENDER.get_or_init(|| sender.clone());
        let ole_initialized = unsafe { OleInitialize(None) }.is_ok();
        register_class()?;
        let instance = unsafe { GetModuleHandleW(None)? };
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE | WS_EX_LAYERED,
                w!("BlackHoleTrashDropTarget"),
                w!("Black Hole Trash Drop Target"),
                WS_POPUP,
                0,
                0,
                WINDOW_SIZE,
                WINDOW_SIZE,
                None,
                None,
                Some(HINSTANCE(instance.0)),
                None,
            )?
        };
        unsafe {
            SetLayeredWindowAttributes(hwnd, Default::default(), 1, LWA_ALPHA)?;
        }
        let target: IDropTarget = ComObject::new(OleDropTarget::new(hwnd, sender)).into_interface();
        unsafe {
            RegisterDragDrop(hwnd, &target)?;
        }
        Ok(Self {
            hwnd,
            _target: target,
            ole_initialized,
            enabled: false,
        })
    }

    pub fn set_center(&self, center: [f64; 2]) {
        let x = center[0].round() as i32 - WINDOW_SIZE / 2;
        let y = center[1].round() as i32 - WINDOW_SIZE / 2;
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                x,
                y,
                WINDOW_SIZE,
                WINDOW_SIZE,
                SWP_NOACTIVATE
                    | if self.enabled {
                        SWP_SHOWWINDOW
                    } else {
                        Default::default()
                    },
            );
        }
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        unsafe {
            let _ = ShowWindow(self.hwnd, if enabled { SW_SHOWNOACTIVATE } else { SW_HIDE });
        }
    }

    pub fn is_dragging() -> bool {
        WIDGET_DRAG_ACTIVE.load(Ordering::Acquire)
    }
}

impl Drop for DropWindow {
    fn drop(&mut self) {
        WIDGET_DRAG_ACTIVE.store(false, Ordering::Release);
        unsafe {
            let _ = RevokeDragDrop(self.hwnd);
            let _ = DestroyWindow(self.hwnd);
            if self.ole_initialized {
                OleUninitialize();
            }
        }
    }
}

fn register_class() -> windows::core::Result<()> {
    static REGISTERED: OnceLock<()> = OnceLock::new();
    if REGISTERED.get().is_some() {
        return Ok(());
    }
    let module = unsafe { GetModuleHandleW(None)? };
    let class = WNDCLASSW {
        style: CS_HREDRAW | CS_VREDRAW,
        lpfnWndProc: Some(window_proc),
        hInstance: HINSTANCE(module.0),
        lpszClassName: w!("BlackHoleTrashDropTarget"),
        ..Default::default()
    };
    let atom = unsafe { RegisterClassW(&class) };
    if atom == 0 {
        return Err(windows::core::Error::from_thread());
    }
    let _ = REGISTERED.set(());
    Ok(())
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_LBUTTONDOWN => {
            let mut cursor = POINT::default();
            let mut rect = RECT::default();
            if unsafe { GetCursorPos(&mut cursor) }.is_ok()
                && unsafe { GetWindowRect(hwnd, &mut rect) }.is_ok()
            {
                let center = [(rect.left + rect.right) / 2, (rect.top + rect.bottom) / 2];
                *drag_offset() = Some([cursor.x - center[0], cursor.y - center[1]]);
                unsafe {
                    SetCapture(hwnd);
                }
                if unsafe { GetCapture() } == hwnd {
                    WIDGET_DRAG_ACTIVE.store(true, Ordering::Release);
                } else {
                    drag_offset().take();
                }
            }
            return LRESULT(0);
        }
        WM_MOUSEMOVE => {
            if let Some(offset) = *drag_offset() {
                send_drag_position(offset);
                return LRESULT(0);
            }
        }
        WM_LBUTTONUP => {
            if let Some(offset) = drag_offset().take() {
                send_drag_position(offset);
            }
            unsafe {
                let _ = ReleaseCapture();
            }
            WIDGET_DRAG_ACTIVE.store(false, Ordering::Release);
            return LRESULT(0);
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            drag_offset().take();
            WIDGET_DRAG_ACTIVE.store(false, Ordering::Release);
        }
        _ => {}
    }
    unsafe { DefWindowProcW(hwnd, message, wparam, lparam) }
}

fn drag_offset() -> std::sync::MutexGuard<'static, Option<[i32; 2]>> {
    DRAG_OFFSET
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn send_drag_position(offset: [i32; 2]) {
    let mut cursor = POINT::default();
    if unsafe { GetCursorPos(&mut cursor) }.is_ok() {
        if let Some(sender) = EVENT_SENDER.get() {
            let _ = sender.try_send(PlatformEvent::MoveRequested {
                point: [(cursor.x - offset[0]) as f64, (cursor.y - offset[1]) as f64],
            });
        }
    }
}
