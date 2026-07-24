use parking_lot::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::{HINSTANCE, LPARAM, LRESULT, WPARAM};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT,
    LLKHF_INJECTED, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

const DOUBLE_TAP_WINDOW: Duration = Duration::from_millis(350);
const VK_CONTROL_CODE: u32 = 0x11;
const VK_LCONTROL_CODE: u32 = 0xA2;
const VK_RCONTROL_CODE: u32 = 0xA3;

struct DoubleCtrlState {
    keys_down: [bool; 256],
    first_release: Option<Duration>,
    first_press_valid: bool,
}

impl Default for DoubleCtrlState {
    fn default() -> Self {
        Self {
            keys_down: [false; 256],
            first_release: None,
            first_press_valid: false,
        }
    }
}

impl DoubleCtrlState {
    fn on_key(&mut self, vk: u32, pressed: bool, at: Duration) -> bool {
        let Some(slot) = self.keys_down.get_mut(vk as usize) else {
            self.cancel();
            return false;
        };
        if *slot == pressed {
            return false;
        }
        *slot = pressed;

        if !is_ctrl(vk) {
            if pressed {
                self.cancel();
            }
            return false;
        }

        if pressed {
            if self.other_key_is_down() {
                self.cancel();
                return false;
            }
            let trigger = self
                .first_release
                .and_then(|first| at.checked_sub(first))
                .is_some_and(|elapsed| elapsed <= DOUBLE_TAP_WINDOW);
            self.first_release = None;
            self.first_press_valid = !trigger;
            trigger
        } else {
            if self.first_press_valid && !self.other_key_is_down() {
                self.first_release = Some(at);
            }
            self.first_press_valid = false;
            false
        }
    }

    fn other_key_is_down(&self) -> bool {
        self.keys_down
            .iter()
            .enumerate()
            .any(|(vk, down)| *down && !is_ctrl(vk as u32))
    }

    fn cancel(&mut self) {
        self.first_release = None;
        self.first_press_valid = false;
    }
}

fn is_ctrl(vk: u32) -> bool {
    matches!(
        vk,
        VK_CONTROL_CODE | VK_LCONTROL_CODE | VK_RCONTROL_CODE
    )
}

struct HookState {
    detector: DoubleCtrlState,
    started: Instant,
    installed: bool,
}

impl Default for HookState {
    fn default() -> Self {
        Self {
            detector: DoubleCtrlState::default(),
            started: Instant::now(),
            installed: false,
        }
    }
}

static HOOK_STATE: OnceLock<Arc<Mutex<HookState>>> = OnceLock::new();
static TRIGGERED: AtomicBool = AtomicBool::new(false);

pub struct CtrlDoubleTapController {
    hook: HHOOK,
    shared: Arc<Mutex<HookState>>,
}

impl CtrlDoubleTapController {
    pub fn new() -> windows::core::Result<Self> {
        let shared = HOOK_STATE
            .get_or_init(|| Arc::new(Mutex::new(HookState::default())))
            .clone();
        {
            let mut state = shared.lock();
            *state = HookState::default();
        }
        TRIGGERED.store(false, Ordering::Release);

        let module = unsafe { GetModuleHandleW(None)? };
        let hook = unsafe {
            SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(keyboard_hook),
                Some(HINSTANCE(module.0)),
                0,
            )?
        };
        shared.lock().installed = true;
        Ok(Self { hook, shared })
    }

    pub fn take_triggered(&self) -> bool {
        TRIGGERED.swap(false, Ordering::AcqRel)
    }
}

impl Drop for CtrlDoubleTapController {
    fn drop(&mut self) {
        let _ = unsafe { UnhookWindowsHookEx(self.hook) };
        self.shared.lock().installed = false;
        TRIGGERED.store(false, Ordering::Release);
    }
}

unsafe extern "system" fn keyboard_hook(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }
    let message = w_param.0 as u32;
    let pressed = matches!(message, WM_KEYDOWN | WM_SYSKEYDOWN);
    if !pressed && !matches!(message, WM_KEYUP | WM_SYSKEYUP) {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }

    let event = unsafe { &*(l_param.0 as *const KBDLLHOOKSTRUCT) };
    if event.flags & LLKHF_INJECTED != Default::default() {
        return unsafe { CallNextHookEx(None, n_code, w_param, l_param) };
    }
    if let Some(shared) = HOOK_STATE.get() {
        if let Some(mut state) = shared.try_lock() {
            if state.installed {
                let at = state.started.elapsed();
                if state.detector.on_key(event.vkCode, pressed, at) {
                    TRIGGERED.store(true, Ordering::Release);
                }
            }
        }
    }
    unsafe { CallNextHookEx(None, n_code, w_param, l_param) }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    const VK_CONTROL: u32 = 0x11;
    const VK_LCONTROL: u32 = 0xA2;
    const VK_RCONTROL: u32 = 0xA3;

    fn ms(value: u64) -> Duration {
        Duration::from_millis(value)
    }

    #[test]
    fn two_clean_ctrl_taps_trigger_once() {
        let mut state = DoubleCtrlState::default();

        assert!(!state.on_key(VK_CONTROL, true, ms(0)));
        assert!(!state.on_key(VK_CONTROL, false, ms(40)));
        assert!(state.on_key(VK_CONTROL, true, ms(250)));
        assert!(!state.on_key(VK_CONTROL, true, ms(260)));
        assert!(!state.on_key(VK_CONTROL, false, ms(280)));
    }

    #[test]
    fn left_and_right_ctrl_are_the_same_gesture() {
        let mut state = DoubleCtrlState::default();

        assert!(!state.on_key(VK_LCONTROL, true, ms(0)));
        assert!(!state.on_key(VK_LCONTROL, false, ms(25)));
        assert!(state.on_key(VK_RCONTROL, true, ms(200)));
    }

    #[test]
    fn expired_tap_becomes_the_start_of_a_new_pair() {
        let mut state = DoubleCtrlState::default();

        assert!(!state.on_key(VK_CONTROL, true, ms(0)));
        assert!(!state.on_key(VK_CONTROL, false, ms(20)));
        assert!(!state.on_key(VK_CONTROL, true, ms(500)));
        assert!(!state.on_key(VK_CONTROL, false, ms(520)));
        assert!(state.on_key(VK_CONTROL, true, ms(700)));
    }

    #[test]
    fn held_ctrl_repeat_does_not_count_as_a_second_tap() {
        let mut state = DoubleCtrlState::default();

        assert!(!state.on_key(VK_CONTROL, true, ms(0)));
        assert!(!state.on_key(VK_CONTROL, true, ms(30)));
        assert!(!state.on_key(VK_CONTROL, true, ms(60)));
    }

    #[test]
    fn another_key_between_taps_cancels_the_candidate() {
        let mut state = DoubleCtrlState::default();

        assert!(!state.on_key(VK_CONTROL, true, ms(0)));
        assert!(!state.on_key(VK_CONTROL, false, ms(20)));
        assert!(!state.on_key(0x43, true, ms(100)));
        assert!(!state.on_key(0x43, false, ms(130)));
        assert!(!state.on_key(VK_CONTROL, true, ms(200)));
    }

    #[test]
    fn pressed_modifier_prevents_a_ctrl_candidate() {
        for modifier in [0x10, 0x12, 0x5B] {
            let mut state = DoubleCtrlState::default();
            assert!(!state.on_key(modifier, true, ms(0)));
            assert!(!state.on_key(VK_CONTROL, true, ms(10)));
            assert!(!state.on_key(VK_CONTROL, false, ms(30)));
            assert!(!state.on_key(modifier, false, ms(40)));
            assert!(!state.on_key(VK_CONTROL, true, ms(100)));
        }
    }
}
