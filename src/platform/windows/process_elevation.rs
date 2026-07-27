use windows::{
    core::{w, HSTRING},
    Win32::{
        Foundation::{CloseHandle, HANDLE},
        Security::{GetTokenInformation, TokenElevation, TOKEN_ELEVATION, TOKEN_QUERY},
        System::Threading::{GetCurrentProcess, OpenProcessToken},
        UI::WindowsAndMessaging::{MessageBoxW, MB_ICONWARNING, MB_OK, MB_SETFOREGROUND},
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ProcessElevation {
    Standard,
    Elevated,
}

fn elevation_from_flag(token_is_elevated: u32) -> ProcessElevation {
    if token_is_elevated == 0 {
        ProcessElevation::Standard
    } else {
        ProcessElevation::Elevated
    }
}

fn elevated_warning_message() -> &'static str {
    "当前程序正在以管理员权限运行，Windows UIPI 会阻止普通权限的文件资源管理器向黑洞拖放文件。\n\n\
     Black Hole Trash 不需要管理员权限，已停止启动。\n\n\
     请关闭此提示后直接双击 BlackHoleTrash.exe，或从开始菜单正常打开。"
}

fn query_failure_message(detail: &str) -> String {
    format!(
        "无法确认当前进程权限，Black Hole Trash 已停止启动，以避免文件拖放无响应。\n\n\
         请直接双击 BlackHoleTrash.exe，或从开始菜单正常打开。\n\n检测错误：{detail}"
    )
}

fn current_process_elevation() -> Result<ProcessElevation, String> {
    let mut token = HANDLE::default();
    unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
        .map_err(|error| format!("OpenProcessToken failed: {error}"))?;

    let result = (|| {
        let mut elevation = TOKEN_ELEVATION::default();
        let mut returned = 0;
        unsafe {
            GetTokenInformation(
                token,
                TokenElevation,
                Some((&mut elevation as *mut TOKEN_ELEVATION).cast()),
                std::mem::size_of::<TOKEN_ELEVATION>() as u32,
                &mut returned,
            )
        }
        .map_err(|error| format!("GetTokenInformation(TokenElevation) failed: {error}"))?;

        let expected = std::mem::size_of::<TOKEN_ELEVATION>() as u32;
        if returned < expected {
            return Err(format!(
                "GetTokenInformation(TokenElevation) returned {returned} bytes; expected {expected}"
            ));
        }
        Ok(elevation_from_flag(elevation.TokenIsElevated))
    })();

    let _ = unsafe { CloseHandle(token) };
    result
}

fn show_elevated_warning() {
    let message = HSTRING::from(elevated_warning_message());
    unsafe {
        MessageBoxW(
            None,
            &message,
            w!("Black Hole Trash - 请勿以管理员身份运行"),
            MB_OK | MB_ICONWARNING | MB_SETFOREGROUND,
        );
    }
}

fn show_query_failure(error: &str) {
    let message = HSTRING::from(query_failure_message(error));
    unsafe {
        MessageBoxW(
            None,
            &message,
            w!("Black Hole Trash - 无法检查启动权限"),
            MB_OK | MB_ICONWARNING | MB_SETFOREGROUND,
        );
    }
}

pub fn allow_startup() -> bool {
    match current_process_elevation() {
        Ok(ProcessElevation::Standard) => true,
        Ok(ProcessElevation::Elevated) => {
            eprintln!("Black Hole Trash refuses to run with an elevated process token");
            show_elevated_warning();
            false
        }
        Err(error) => {
            eprintln!("startup privilege check failed: {error}");
            show_query_failure(&error);
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_token_elevation_is_standard_user() {
        assert_eq!(elevation_from_flag(0), ProcessElevation::Standard);
    }

    #[test]
    fn any_nonzero_token_elevation_is_elevated() {
        assert_eq!(elevation_from_flag(1), ProcessElevation::Elevated);
        assert_eq!(elevation_from_flag(u32::MAX), ProcessElevation::Elevated);
    }

    #[test]
    fn elevated_warning_explains_uipi_and_normal_launch() {
        let message = elevated_warning_message();
        assert!(message.contains("管理员权限"));
        assert!(message.contains("UIPI"));
        assert!(message.contains("双击"));
        assert!(message.contains("开始菜单"));
    }

    #[test]
    fn query_failure_message_stops_uncertain_startup() {
        let message = query_failure_message("OpenProcessToken failed");
        assert!(message.contains("无法确认当前进程权限"));
        assert!(message.contains("已停止启动"));
        assert!(message.contains("OpenProcessToken failed"));
    }
}
