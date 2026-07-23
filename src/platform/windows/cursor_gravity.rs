use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::Instant;
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_ABSOLUTE, MOUSEEVENTF_MOVE,
    MOUSEEVENTF_VIRTUALDESK, MOUSEINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetCursorInfo, GetSystemMetrics, SetWindowsHookExW, ShowCursor,
    UnhookWindowsHookEx, CURSORINFO, CURSOR_SHOWING, HHOOK, LLMHF_INJECTED, MSLLHOOKSTRUCT,
    SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, WH_MOUSE_LL,
    WM_MOUSEMOVE,
};

pub const TRAIL_SAMPLES: usize = 16;
const FIELD_RADIUS_MULTIPLIER: f64 = 6.0;
const MAX_RESISTANCE: f64 = 0.10;
const MAX_CORRECTION_PX: f64 = 4.0;
const ESCAPE_SPEED_PX: f64 = 34.0;
const HIDDEN_SECONDS: f32 = 0.150;
const ABSORB_SECONDS: f32 = 0.160;
const ABSORB_RADIUS_MULTIPLIER: f64 = 2.20;
const CURSOR_GRAVITY_TAG: usize = 0x4248_5452;

#[derive(Clone, Copy, Debug, Default)]
pub struct CursorSample {
    pub point: [f64; 2],
    pub age: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct CursorSnapshot {
    pub active: bool,
    pub point: [f64; 2],
    pub velocity: [f32; 2],
    pub strength: f32,
    pub absorption: f32,
    pub trail: [CursorSample; TRAIL_SAMPLES],
}

impl Default for CursorSnapshot {
    fn default() -> Self {
        Self {
            active: false,
            point: [0.0; 2],
            velocity: [0.0; 2],
            strength: 0.0,
            absorption: 0.0,
            trail: [CursorSample::default(); TRAIL_SAMPLES],
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct Field {
    center: [f64; 2],
    radius_px: f64,
    enabled: bool,
    suspended: bool,
}

#[derive(Clone, Copy)]
struct TimedSample {
    point: [f64; 2],
    at: Instant,
}

#[derive(Clone, Copy, Debug)]
enum Phase {
    Inactive,
    Attracting,
    Absorbing(Instant),
    Hidden(Instant),
}

struct SharedState {
    field: Field,
    previous: Option<[f64; 2]>,
    snapshot: CursorSnapshot,
    trail: [Option<TimedSample>; TRAIL_SAMPLES],
    phase: Phase,
    cursor_hidden_by_us: bool,
    visibility_safe: bool,
    installed: bool,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            field: Field::default(),
            previous: None,
            snapshot: CursorSnapshot::default(),
            trail: [None; TRAIL_SAMPLES],
            phase: Phase::Inactive,
            cursor_hidden_by_us: false,
            visibility_safe: true,
            installed: false,
        }
    }
}

static HOOK_STATE: OnceLock<Arc<Mutex<SharedState>>> = OnceLock::new();
static RECOVERY_REQUESTED: AtomicBool = AtomicBool::new(false);

pub struct CursorGravityController {
    hook: HHOOK,
    shared: Arc<Mutex<SharedState>>,
}

impl CursorGravityController {
    pub fn new() -> windows::core::Result<Self> {
        let shared = HOOK_STATE
            .get_or_init(|| Arc::new(Mutex::new(SharedState::default())))
            .clone();
        {
            let mut state = shared.lock();
            state.restore_cursor();
            *state = SharedState::default();
        }

        let module = unsafe { GetModuleHandleW(None)? };
        let hook = unsafe {
            SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook), Some(HINSTANCE(module.0)), 0)?
        };
        shared.lock().installed = true;
        Ok(Self { hook, shared })
    }

    pub fn set_field(&self, center: [f64; 2], radius_px: f64, enabled: bool) {
        let mut state = self.shared.lock();
        state.field.center = center;
        state.field.radius_px = radius_px.max(1.0);
        state.field.enabled = enabled;
        if !enabled {
            state.deactivate();
        }
    }

    pub fn set_suspended(&self, suspended: bool) {
        let mut state = self.shared.lock();
        state.field.suspended = suspended;
        if suspended {
            state.deactivate();
        }
    }

    pub fn snapshot(&self) -> CursorSnapshot {
        let mut state = self.shared.lock();
        if RECOVERY_REQUESTED.swap(false, Ordering::AcqRel) {
            state.deactivate();
        }
        state.refresh_timing(Instant::now());
        state.snapshot
    }
}

impl Drop for CursorGravityController {
    fn drop(&mut self) {
        let _ = unsafe { UnhookWindowsHookEx(self.hook) };
        let mut state = self.shared.lock();
        state.restore_cursor();
        state.installed = false;
        state.field.enabled = false;
        state.deactivate();
    }
}

impl SharedState {
    fn try_hide_cursor(&mut self) -> bool {
        if self.cursor_hidden_by_us {
            return true;
        }
        if !self.visibility_safe {
            return false;
        }

        let mut info = CURSORINFO {
            cbSize: std::mem::size_of::<CURSORINFO>() as u32,
            ..Default::default()
        };
        if unsafe { GetCursorInfo(&mut info) }.is_err() || info.flags.0 & CURSOR_SHOWING.0 == 0 {
            return false;
        }

        let display_count = unsafe { ShowCursor(false) };
        if display_count < 0 {
            self.cursor_hidden_by_us = true;
            true
        } else {
            unsafe {
                ShowCursor(true);
            }
            self.visibility_safe = false;
            false
        }
    }

    fn restore_cursor(&mut self) {
        if self.cursor_hidden_by_us {
            unsafe {
                ShowCursor(true);
            }
            self.cursor_hidden_by_us = false;
        }
    }

    fn deactivate(&mut self) {
        self.restore_cursor();
        self.phase = Phase::Inactive;
        self.snapshot.active = false;
        self.snapshot.strength = 0.0;
        self.snapshot.absorption = 0.0;
        self.trail = [None; TRAIL_SAMPLES];
    }

    fn refresh_timing(&mut self, now: Instant) {
        match self.phase {
            Phase::Absorbing(started) => {
                let progress = (now - started).as_secs_f32() / ABSORB_SECONDS;
                self.snapshot.absorption = progress.clamp(0.0, 1.0);
                if progress >= 1.0 {
                    self.phase = Phase::Hidden(now);
                    self.snapshot.active = false;
                }
            }
            Phase::Hidden(started) if (now - started).as_secs_f32() >= HIDDEN_SECONDS => {
                self.deactivate();
            }
            _ => {}
        }
        self.refresh_trail_ages(now);
    }

    fn begin_sample(&mut self, previous: [f64; 2], point: [f64; 2], strength: f32, now: Instant) {
        for i in (1..TRAIL_SAMPLES).rev() {
            self.trail[i] = self.trail[i - 1];
        }
        let sample = TimedSample { point, at: now };
        self.trail[0] = Some(sample);
        for item in self.trail.iter_mut().skip(1) {
            if item.is_none() {
                *item = Some(sample);
            }
        }

        self.snapshot.active = true;
        self.snapshot.point = point;
        self.snapshot.velocity = [
            (point[0] - previous[0]) as f32,
            (point[1] - previous[1]) as f32,
        ];
        self.snapshot.strength = strength;
        self.refresh_trail_ages(now);
    }

    fn refresh_trail_ages(&mut self, now: Instant) {
        for (out, sample) in self.snapshot.trail.iter_mut().zip(self.trail) {
            if let Some(sample) = sample {
                *out = CursorSample {
                    point: sample.point,
                    age: (now - sample.at).as_secs_f32(),
                };
            } else {
                *out = CursorSample::default();
            }
        }
    }
}

fn smoothstep01(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn shape_move(previous: [f64; 2], requested: [f64; 2], field: Field) -> ([f64; 2], f32) {
    let delta = [requested[0] - previous[0], requested[1] - previous[1]];
    let rel = [
        requested[0] - field.center[0],
        requested[1] - field.center[1],
    ];
    let distance = rel[0].hypot(rel[1]);
    let field_radius = (field.radius_px * FIELD_RADIUS_MULTIPLIER).max(1.0);
    if !field.enabled || field.suspended || distance >= field_radius {
        return (requested, 0.0);
    }

    let direction = if distance > 0.001 {
        [rel[0] / distance, rel[1] / distance]
    } else {
        [0.0, 0.0]
    };
    let outward_speed = delta[0] * direction[0] + delta[1] * direction[1];
    if outward_speed >= ESCAPE_SPEED_PX {
        return (requested, 0.0);
    }

    let strength = smoothstep01(1.0 - distance / field_radius);
    let resistance = MAX_RESISTANCE * strength;
    let radial = (1.6 * strength).min(MAX_CORRECTION_PX);
    let tangent = 0.75 * strength;
    let correction = [
        -direction[0] * radial - direction[1] * tangent,
        -direction[1] * radial + direction[0] * tangent,
    ];
    (
        [
            previous[0] + delta[0] * (1.0 - resistance) + correction[0],
            previous[1] + delta[1] * (1.0 - resistance) + correction[1],
        ],
        strength as f32,
    )
}

fn inject_cursor_pos(point: [i32; 2]) -> bool {
    let virtual_x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
    let virtual_y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
    let virtual_w = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) }.max(2);
    let virtual_h = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) }.max(2);
    let normalize = |value: i32, origin: i32, extent: i32| {
        (((value as i64 - origin as i64) * 65_535) / (extent as i64 - 1)).clamp(0, 65_535) as i32
    };
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: normalize(point[0], virtual_x, virtual_w),
                dy: normalize(point[1], virtual_y, virtual_h),
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE | MOUSEEVENTF_ABSOLUTE | MOUSEEVENTF_VIRTUALDESK,
                time: 0,
                dwExtraInfo: CURSOR_GRAVITY_TAG,
            },
        },
    };
    (unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) }) == 1
}

unsafe extern "system" fn mouse_hook(n_code: i32, w_param: WPARAM, l_param: LPARAM) -> LRESULT {
    if n_code < 0 || w_param.0 as u32 != WM_MOUSEMOVE {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }
    let event = unsafe { &*(l_param.0 as *const MSLLHOOKSTRUCT) };
    if event.flags & LLMHF_INJECTED != 0 {
        if event.dwExtraInfo != CURSOR_GRAVITY_TAG {
            RECOVERY_REQUESTED.store(true, Ordering::Release);
            if let Some(shared) = HOOK_STATE.get() {
                if let Some(mut state) = shared.try_lock() {
                    state.deactivate();
                    state.previous = Some([event.pt.x as f64, event.pt.y as f64]);
                    state.snapshot.point = [event.pt.x as f64, event.pt.y as f64];
                    RECOVERY_REQUESTED.store(false, Ordering::Release);
                }
            }
        }
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    let Some(shared) = HOOK_STATE.get() else {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    };
    let Some(mut state) = shared.try_lock() else {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    };
    if !state.installed {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }
    if RECOVERY_REQUESTED.swap(false, Ordering::AcqRel) {
        state.deactivate();
    }

    let now = Instant::now();
    let requested = [event.pt.x as f64, event.pt.y as f64];
    state.refresh_timing(now);
    if matches!(state.phase, Phase::Hidden(_)) {
        state.deactivate();
        state.previous = Some(requested);
        state.snapshot.point = requested;
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    let previous = state.previous.unwrap_or(requested);
    let (adjusted, strength) = shape_move(previous, requested, state.field);
    if strength <= 0.0 || !state.try_hide_cursor() {
        state.deactivate();
        state.previous = Some(requested);
        state.snapshot.point = requested;
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    let distance = (adjusted[0] - state.field.center[0]).hypot(adjusted[1] - state.field.center[1]);
    if distance <= state.field.radius_px * ABSORB_RADIUS_MULTIPLIER {
        if !matches!(state.phase, Phase::Absorbing(_)) {
            state.phase = Phase::Absorbing(now);
        }
    } else {
        state.phase = Phase::Attracting;
        state.snapshot.absorption = 0.0;
    }
    state.begin_sample(previous, adjusted, strength, now);

    let replacement = [adjusted[0].round() as i32, adjusted[1].round() as i32];
    if replacement != [event.pt.x, event.pt.y] {
        if inject_cursor_pos(replacement) {
            let actual = [replacement[0] as f64, replacement[1] as f64];
            state.previous = Some(actual);
            state.snapshot.point = actual;
            return LRESULT(1);
        }
        state.deactivate();
    }

    state.previous = Some(requested);
    state.snapshot.point = requested;
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outside_field_is_unchanged() {
        let field = Field {
            center: [0.0, 0.0],
            radius_px: 20.0,
            enabled: true,
            suspended: false,
        };
        assert_eq!(
            shape_move([200.0, 0.0], [210.0, 0.0], field).0,
            [210.0, 0.0]
        );
    }

    #[test]
    fn resistance_never_exceeds_ten_percent_before_force() {
        let field = Field {
            center: [0.0, 0.0],
            radius_px: 20.0,
            enabled: true,
            suspended: false,
        };
        let (point, strength) = shape_move([30.0, 0.0], [40.0, 0.0], field);
        assert!(strength > 0.0);
        assert!(point[0] >= 30.0 + 10.0 * (1.0 - MAX_RESISTANCE) - MAX_CORRECTION_PX);
    }

    #[test]
    fn fast_outward_flick_escapes() {
        let field = Field {
            center: [0.0, 0.0],
            radius_px: 20.0,
            enabled: true,
            suspended: false,
        };
        assert_eq!(
            shape_move([10.0, 0.0], [60.0, 0.0], field),
            ([60.0, 0.0], 0.0)
        );
    }
}
