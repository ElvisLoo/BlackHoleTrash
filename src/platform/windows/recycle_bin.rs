use super::ole_drop_target::validate_path;
use super::{EventSender, PlatformEvent};
use std::os::windows::ffi::OsStrExt;
use std::path::PathBuf;
use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
    COINIT_APARTMENTTHREADED,
};
use windows::Win32::UI::Shell::{
    FileOperation, IFileOperation, IShellItem, SHCreateItemFromParsingName, FOFX_EARLYFAILURE,
    FOFX_RECYCLEONDELETE, FOF_NOCONFIRMATION, FOF_NOERRORUI, FOF_SILENT,
};
use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK, MB_SETFOREGROUND};

#[derive(Debug, Clone)]
pub struct RecycleResult {
    pub generation: u64,
    pub paths: Vec<PathBuf>,
    pub succeeded: bool,
    pub aborted: bool,
    pub message: Option<String>,
}

pub fn recycle_async(generation: u64, paths: Vec<PathBuf>, sender: EventSender) {
    std::thread::spawn(move || {
        let result = recycle_worker(generation, paths);
        let _ = sender.send(PlatformEvent::RecycleFinished(result));
    });
}

pub fn show_recycle_failure(result: &RecycleResult) {
    let message = HSTRING::from(format!(
        "Black Hole Trash could not move {} item(s) to the Recycle Bin.\n\n{}",
        result.paths.len(),
        result
            .message
            .as_deref()
            .unwrap_or("Windows Shell cancelled the operation")
    ));
    unsafe {
        let _ = MessageBoxW(
            None,
            PCWSTR(message.as_ptr()),
            w!("Black Hole Trash"),
            MB_OK | MB_ICONERROR | MB_SETFOREGROUND,
        );
    }
}

struct ComApartment;

impl ComApartment {
    fn initialize() -> Result<Self, String> {
        unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) }
            .ok()
            .map_err(|error| format!("COM initialization: {error}"))?;
        Ok(Self)
    }
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}

fn recycle_worker(generation: u64, paths: Vec<PathBuf>) -> RecycleResult {
    let outcome = (|| -> Result<bool, String> {
        if paths.is_empty() {
            return Err("the drop did not contain any filesystem items".into());
        }
        for path in &paths {
            validate_path(path).map_err(str::to_string)?;
        }
        let _apartment = ComApartment::initialize()?;
        let operation: IFileOperation =
            unsafe { CoCreateInstance(&FileOperation, None, CLSCTX_INPROC_SERVER) }
                .map_err(|error| format!("IFileOperation: {error}"))?;
        unsafe {
            operation
                .SetOperationFlags(
                    FOFX_RECYCLEONDELETE
                        | FOFX_EARLYFAILURE
                        | FOF_NOCONFIRMATION
                        | FOF_NOERRORUI
                        | FOF_SILENT,
                )
                .map_err(|error| format!("SetOperationFlags: {error}"))?;
            for path in &paths {
                let wide: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
                let item: IShellItem = SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None)
                    .map_err(|error| format!("Shell item {}: {error}", path.display()))?;
                operation
                    .DeleteItem(&item, None)
                    .map_err(|error| format!("queue recycle {}: {error}", path.display()))?;
            }
            operation
                .PerformOperations()
                .map_err(|error| format!("PerformOperations: {error}"))?;
            let aborted = operation
                .GetAnyOperationsAborted()
                .map_err(|error| format!("GetAnyOperationsAborted: {error}"))?
                .as_bool();
            Ok(aborted)
        }
    })();

    match outcome {
        Ok(aborted) => RecycleResult {
            generation,
            paths,
            succeeded: !aborted,
            aborted,
            message: aborted.then(|| "Windows Shell cancelled the recycle operation".into()),
        },
        Err(message) => RecycleResult {
            generation,
            paths,
            succeeded: false,
            aborted: false,
            message: Some(message),
        },
    }
}
