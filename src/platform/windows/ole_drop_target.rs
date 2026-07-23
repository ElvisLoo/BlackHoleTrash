use super::{EventSender, PlatformEvent};
use parking_lot::Mutex;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use windows::core::{implement, Ref, PCWSTR};
use windows::Win32::Foundation::{HWND, POINT, POINTL, RECT};
use windows::Win32::Storage::FileSystem::GetDriveTypeW;
use windows::Win32::System::Com::{
    CoCreateInstance, IDataObject, CLSCTX_INPROC_SERVER, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL,
};
use windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, ReleaseStgMedium, DROPEFFECT, DROPEFFECT_MOVE, DROPEFFECT_NONE,
};
use windows::Win32::UI::Shell::{CLSID_DragDropHelper, DragQueryFileW, IDropTargetHelper, HDROP};
use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;

const CF_HDROP: u16 = 15;
const CAPTURE_RADIUS_PX: f64 = 40.0;
const DRIVE_FIXED_TYPE: u32 = 3;

#[derive(Debug, Clone)]
struct DragSession {
    generation: u64,
    paths: Vec<PathBuf>,
    accepted: bool,
}

#[implement(IDropTarget)]
pub struct OleDropTarget {
    hwnd: HWND,
    sender: EventSender,
    generation: AtomicU64,
    session: Mutex<Option<DragSession>>,
    helper: Option<IDropTargetHelper>,
}

impl OleDropTarget {
    pub fn new(hwnd: HWND, sender: EventSender) -> Self {
        let helper =
            unsafe { CoCreateInstance(&CLSID_DragDropHelper, None, CLSCTX_INPROC_SERVER) }.ok();
        Self {
            hwnd,
            sender,
            generation: AtomicU64::new(0),
            session: Mutex::new(None),
            helper,
        }
    }

    fn effect(&self, point: &POINTL, accepted: bool) -> DROPEFFECT {
        if accepted && self.point_is_captured(point) {
            DROPEFFECT_MOVE
        } else {
            DROPEFFECT_NONE
        }
    }

    fn point_is_captured(&self, point: &POINTL) -> bool {
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(self.hwnd, &mut rect) }.is_err() {
            return false;
        }
        let center = [
            (rect.left + rect.right) as f64 * 0.5,
            (rect.top + rect.bottom) as f64 * 0.5,
        ];
        let dx = point.x as f64 - center[0];
        let dy = point.y as f64 - center[1];
        dx * dx + dy * dy <= CAPTURE_RADIUS_PX * CAPTURE_RADIUS_PX
    }

    fn restore_drag_image(&self) {
        if let Some(helper) = &self.helper {
            unsafe {
                let _ = helper.Show(true);
            }
        }
    }
}

impl IDropTarget_Impl for OleDropTarget_Impl {
    fn DragEnter(
        &self,
        data: Ref<IDataObject>,
        _keys: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let generation = self.generation.fetch_add(1, Ordering::Relaxed) + 1;
        let paths = data
            .ok()
            .and_then(paths_from_data_object)
            .unwrap_or_default();
        let accepted = !paths.is_empty() && paths.iter().all(|path| validate_path(path).is_ok());
        *self.session.lock() = Some(DragSession {
            generation,
            paths: paths.clone(),
            accepted,
        });
        let native = POINT {
            x: point.x,
            y: point.y,
        };
        let chosen = self.effect(point, accepted);
        unsafe {
            if !effect.is_null() {
                effect.write(chosen);
            }
            if let (Some(helper), Ok(data)) = (&self.helper, data.ok()) {
                let _ = helper.DragEnter(self.hwnd, data, &native, chosen);
                let _ = helper.Show(chosen == DROPEFFECT_NONE);
            }
        }
        if accepted {
            let _ = self.sender.try_send(PlatformEvent::DragEntered {
                generation,
                paths,
                point: [point.x as f64, point.y as f64],
            });
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _keys: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let session = self.session.lock().clone();
        let chosen = self.effect(point, session.as_ref().is_some_and(|value| value.accepted));
        let native = POINT {
            x: point.x,
            y: point.y,
        };
        unsafe {
            if !effect.is_null() {
                effect.write(chosen);
            }
            if let Some(helper) = &self.helper {
                let _ = helper.DragOver(&native, chosen);
                let _ = helper.Show(chosen == DROPEFFECT_NONE);
            }
        }
        if let Some(session) = session.filter(|value| value.accepted) {
            let _ = self.sender.try_send(PlatformEvent::DragMoved {
                generation: session.generation,
                point: [point.x as f64, point.y as f64],
            });
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        if let Some(session) = self.session.lock().take() {
            let _ = self.sender.try_send(PlatformEvent::DragLeft {
                generation: session.generation,
            });
        }
        if let Some(helper) = &self.helper {
            unsafe {
                let _ = helper.DragLeave();
            }
        }
        self.restore_drag_image();
        Ok(())
    }

    fn Drop(
        &self,
        data: Ref<IDataObject>,
        _keys: windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS,
        point: &POINTL,
        effect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        let session = self.session.lock().take();
        let accepted =
            session.as_ref().is_some_and(|value| value.accepted) && self.point_is_captured(point);
        let chosen = if accepted {
            DROPEFFECT_MOVE
        } else {
            DROPEFFECT_NONE
        };
        let native = POINT {
            x: point.x,
            y: point.y,
        };
        unsafe {
            if !effect.is_null() {
                effect.write(chosen);
            }
            if let (Some(helper), Ok(data)) = (&self.helper, data.ok()) {
                let _ = helper.Drop(data, &native, chosen);
            }
        }
        self.restore_drag_image();
        if let Some(session) = session.filter(|_| accepted) {
            let fresh = data
                .ok()
                .and_then(paths_from_data_object)
                .unwrap_or_else(|_| session.paths.clone());
            let unchanged =
                fresh == session.paths && fresh.iter().all(|path| validate_path(path).is_ok());
            if unchanged {
                let _ = self.sender.try_send(PlatformEvent::Dropped {
                    generation: session.generation,
                    paths: fresh,
                    point: [point.x as f64, point.y as f64],
                });
            } else {
                let _ = self.sender.try_send(PlatformEvent::DragLeft {
                    generation: session.generation,
                });
            }
        }
        Ok(())
    }
}

fn paths_from_data_object(data: &IDataObject) -> windows::core::Result<Vec<PathBuf>> {
    let format = FORMATETC {
        cfFormat: CF_HDROP,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };
    let mut medium = unsafe { data.GetData(&format) }?;
    let hdrop = HDROP(unsafe { medium.u.hGlobal.0 });
    let count = unsafe { DragQueryFileW(hdrop, u32::MAX, None) };
    let mut paths = Vec::with_capacity(count as usize);
    for index in 0..count {
        let length = unsafe { DragQueryFileW(hdrop, index, None) };
        if length == 0 {
            continue;
        }
        let mut buffer = vec![0u16; length as usize + 1];
        let written = unsafe { DragQueryFileW(hdrop, index, Some(&mut buffer)) };
        buffer.truncate(written as usize);
        paths.push(PathBuf::from(String::from_utf16_lossy(&buffer)));
    }
    unsafe {
        ReleaseStgMedium(&mut medium);
    }
    Ok(paths)
}

pub fn validate_path(path: &Path) -> Result<(), &'static str> {
    if !path.exists() {
        return Err("the item no longer exists");
    }
    let text = path.as_os_str().to_string_lossy();
    if text.starts_with("\\\\") {
        return Err("network items cannot be safely recycled");
    }
    let canonical = path
        .canonicalize()
        .map_err(|_| "the path cannot be resolved")?;
    if canonical.components().any(|part| {
        part.as_os_str()
            .to_string_lossy()
            .eq_ignore_ascii_case("$Recycle.Bin")
    }) {
        return Err("items already inside the Recycle Bin are not accepted");
    }
    if canonical.file_name().is_none() {
        return Err("drive roots cannot be recycled");
    }
    let volume_root = canonical
        .ancestors()
        .last()
        .ok_or("the volume root cannot be resolved")?;
    let volume_wide: Vec<u16> = volume_root
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    if unsafe { GetDriveTypeW(PCWSTR(volume_wide.as_ptr())) } != DRIVE_FIXED_TYPE {
        return Err("only local fixed-drive items can be safely recycled");
    }
    let current = std::env::current_exe()
        .map_err(|_| "the application path is unavailable")?
        .canonicalize()
        .map_err(|_| "the application path cannot be resolved")?;
    if canonical == current
        || current
            .parent()
            .is_some_and(|root| canonical.starts_with(root))
    {
        return Err("Black Hole Trash cannot recycle its own installation");
    }
    if let Ok(windows_dir) = std::env::var("WINDIR") {
        let windows_dir = Path::new(&windows_dir)
            .canonicalize()
            .map_err(|_| "the Windows directory cannot be resolved")?;
        if canonical.starts_with(windows_dir) {
            return Err("Windows system files are protected");
        }
    }
    let metadata = std::fs::metadata(&canonical).map_err(|_| "metadata is unavailable")?;
    if !(metadata.is_file() || metadata.is_dir()) {
        return Err("the Shell item is not a filesystem file or folder");
    }
    Ok(())
}
