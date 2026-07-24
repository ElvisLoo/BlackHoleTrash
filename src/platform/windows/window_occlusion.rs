use windows::core::BOOL;
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::Graphics::Dwm::{
    DwmGetWindowAttribute, DWMWA_CLOAKED, DWMWA_EXTENDED_FRAME_BOUNDS,
};
use windows::Win32::System::Threading::GetCurrentProcessId;
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetWindowRect, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
};

#[derive(Clone, Copy, Debug)]
struct Rect {
    left: f64,
    top: f64,
    right: f64,
    bottom: f64,
}

impl Rect {
    fn new(left: f64, top: f64, right: f64, bottom: f64) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    fn is_valid(self) -> bool {
        self.left < self.right
            && self.top < self.bottom
            && self.left.is_finite()
            && self.top.is_finite()
            && self.right.is_finite()
            && self.bottom.is_finite()
    }
}

impl From<RECT> for Rect {
    fn from(value: RECT) -> Self {
        Self::new(
            value.left as f64,
            value.top as f64,
            value.right as f64,
            value.bottom as f64,
        )
    }
}

fn circle_intersects_rect(center: [f64; 2], radius: f64, rect: Rect) -> bool {
    if !rect.is_valid()
        || !center[0].is_finite()
        || !center[1].is_finite()
        || !radius.is_finite()
        || radius <= 0.0
    {
        return false;
    }
    let x = center[0].clamp(rect.left, rect.right);
    let y = center[1].clamp(rect.top, rect.bottom);
    let dx = center[0] - x;
    let dy = center[1] - y;
    dx * dx + dy * dy <= radius * radius
}

#[derive(Clone, Copy, Debug, Default)]
struct WindowFacts {
    same_process: bool,
    visible: bool,
    minimized: bool,
    cloaked: bool,
    ignored_class: bool,
}

fn is_occluding_candidate(facts: WindowFacts) -> bool {
    facts.visible
        && !facts.same_process
        && !facts.minimized
        && !facts.cloaked
        && !facts.ignored_class
}

fn is_ignored_class(class: &str) -> bool {
    [
        "Progman",
        "WorkerW",
        "Shell_TrayWnd",
        "Shell_SecondaryTrayWnd",
        "tooltips_class32",
        "#32768",
        "SysShadow",
        "NotifyIconOverflowWindow",
    ]
    .iter()
    .any(|ignored| class.eq_ignore_ascii_case(ignored))
}

struct EnumContext {
    process_id: u32,
    center: [f64; 2],
    radius: f64,
    occluded: bool,
}

pub fn is_black_hole_occluded(
    center: [f64; 2],
    radius: f64,
) -> windows::core::Result<bool> {
    if !center[0].is_finite()
        || !center[1].is_finite()
        || !radius.is_finite()
        || radius <= 0.0
    {
        return Ok(false);
    }
    let mut context = EnumContext {
        process_id: unsafe { GetCurrentProcessId() },
        center,
        radius,
        occluded: false,
    };
    let result = unsafe {
        EnumWindows(
            Some(enum_window),
            LPARAM((&mut context as *mut EnumContext) as isize),
        )
    };
    if context.occluded {
        Ok(true)
    } else {
        result.map(|_| false)
    }
}

unsafe extern "system" fn enum_window(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let context = unsafe { &mut *(lparam.0 as *mut EnumContext) };
    let mut process_id = 0u32;
    unsafe {
        GetWindowThreadProcessId(hwnd, Some(&mut process_id));
    }
    let class = window_class(hwnd);
    let facts = WindowFacts {
        same_process: process_id == context.process_id,
        visible: unsafe { IsWindowVisible(hwnd).as_bool() },
        minimized: unsafe { IsIconic(hwnd).as_bool() },
        cloaked: window_is_cloaked(hwnd),
        ignored_class: is_ignored_class(&class),
    };
    if !is_occluding_candidate(facts) {
        return BOOL(1);
    }

    let Some(rect) = window_rect(hwnd) else {
        context.occluded = true;
        return BOOL(0);
    };
    if circle_intersects_rect(context.center, context.radius, rect) {
        context.occluded = true;
        return BOOL(0);
    }
    BOOL(1)
}

fn window_class(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let len = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buffer[..len as usize])
    }
}

fn window_is_cloaked(hwnd: HWND) -> bool {
    let mut cloaked = 0u32;
    unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_CLOAKED,
            &mut cloaked as *mut u32 as *mut std::ffi::c_void,
            std::mem::size_of::<u32>() as u32,
        )
    }
    .is_ok()
        && cloaked != 0
}

fn window_rect(hwnd: HWND) -> Option<Rect> {
    let mut native = RECT::default();
    let dwm_result = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut native as *mut RECT as *mut std::ffi::c_void,
            std::mem::size_of::<RECT>() as u32,
        )
    };
    if dwm_result.is_err() && unsafe { GetWindowRect(hwnd, &mut native) }.is_err() {
        return None;
    }
    let rect = Rect::from(native);
    rect.is_valid().then_some(rect)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn circle_intersects_overlapping_rectangle() {
        assert!(circle_intersects_rect(
            [0.0, 0.0],
            10.0,
            Rect::new(5.0, -2.0, 15.0, 2.0)
        ));
    }

    #[test]
    fn circle_does_not_intersect_separated_rectangle() {
        assert!(!circle_intersects_rect(
            [0.0, 0.0],
            10.0,
            Rect::new(11.0, -2.0, 15.0, 2.0)
        ));
    }

    #[test]
    fn circle_tangent_to_rectangle_counts_as_covered() {
        assert!(circle_intersects_rect(
            [0.0, 0.0],
            10.0,
            Rect::new(10.0, -1.0, 15.0, 1.0)
        ));
    }

    #[test]
    fn bounding_box_corner_does_not_cover_circle() {
        assert!(!circle_intersects_rect(
            [0.0, 0.0],
            10.0,
            Rect::new(9.0, 9.0, 12.0, 12.0)
        ));
    }

    #[test]
    fn circle_center_inside_rectangle_counts_as_covered() {
        assert!(circle_intersects_rect(
            [0.0, 0.0],
            10.0,
            Rect::new(-1.0, -1.0, 1.0, 1.0)
        ));
    }

    #[test]
    fn invalid_rectangle_never_covers_circle() {
        assert!(!circle_intersects_rect(
            [0.0, 0.0],
            10.0,
            Rect::new(5.0, 5.0, 5.0, 9.0)
        ));
    }

    #[test]
    fn only_visible_ordinary_windows_are_candidates() {
        let ordinary = WindowFacts {
            visible: true,
            ..WindowFacts::default()
        };
        assert!(is_occluding_candidate(ordinary));

        for ignored in [
            WindowFacts {
                same_process: true,
                ..ordinary
            },
            WindowFacts {
                visible: false,
                ..ordinary
            },
            WindowFacts {
                minimized: true,
                ..ordinary
            },
            WindowFacts {
                cloaked: true,
                ..ordinary
            },
            WindowFacts {
                ignored_class: true,
                ..ordinary
            },
        ] {
            assert!(!is_occluding_candidate(ignored));
        }
    }

    #[test]
    fn shell_and_transient_class_names_are_ignored() {
        for class in [
            "Progman",
            "WorkerW",
            "Shell_TrayWnd",
            "Shell_SecondaryTrayWnd",
            "tooltips_class32",
            "#32768",
        ] {
            assert!(is_ignored_class(class), "class should be ignored: {class}");
        }
        assert!(!is_ignored_class("Notepad"));
        assert!(!is_ignored_class("Chrome_WidgetWin_1"));
    }
}
