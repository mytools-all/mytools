#[cfg(windows)]
mod native {
    use std::{
        cell::{Cell, RefCell},
        collections::HashMap,
        os::windows::ffi::OsStringExt,
        path::PathBuf,
        sync::Arc,
    };
    use windows::{
        core::{implement, Result},
        Win32::{
            Foundation::{BOOL, HWND, LPARAM, POINTL},
            System::{
                Com::{IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL},
                Ole::{
                    IDropTarget, IDropTarget_Impl, RegisterDragDrop, ReleaseStgMedium,
                    RevokeDragDrop, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE,
                },
                SystemServices::MODIFIERKEYS_FLAGS,
            },
            UI::{
                Shell::{DragQueryFileW, HDROP},
                WindowsAndMessaging::{EnumChildWindows, IsWindow},
            },
        },
    };

    type Receiver = Arc<dyn Fn(Vec<PathBuf>) + Send + Sync>;
    thread_local! { static TARGETS: RefCell<HashMap<isize, IDropTarget>> = RefCell::new(HashMap::new()); }
    thread_local! { static OLE_READY: bool = unsafe { windows::Win32::System::Ole::OleInitialize(None).is_ok() }; }

    #[implement(IDropTarget)]
    struct FileTarget {
        valid: Cell<bool>,
        receive: Receiver,
    }

    fn paths(data: Option<&IDataObject>) -> Vec<PathBuf> {
        let Some(data) = data else {
            return Vec::new();
        };
        let format = FORMATETC {
            cfFormat: 15,
            ptd: std::ptr::null_mut(),
            dwAspect: DVASPECT_CONTENT.0,
            lindex: -1,
            tymed: TYMED_HGLOBAL.0 as u32,
        };
        let Ok(mut medium) = (unsafe { data.GetData(&format) }) else {
            return Vec::new();
        };
        let mut out = Vec::new();
        unsafe {
            let drop = HDROP(medium.u.hGlobal.0 as isize);
            let count = DragQueryFileW(drop, u32::MAX, None).min(1000);
            for index in 0..count {
                let length = DragQueryFileW(drop, index, None);
                if length == 0 || length > 32767 {
                    continue;
                }
                let mut text = vec![0u16; length as usize + 1];
                DragQueryFileW(drop, index, Some(&mut text));
                out.push(PathBuf::from(std::ffi::OsString::from_wide(
                    &text[..length as usize],
                )));
            }
            ReleaseStgMedium(&mut medium);
        }
        out
    }

    impl IDropTarget_Impl for FileTarget {
        fn DragEnter(
            &self,
            data: Option<&IDataObject>,
            _: MODIFIERKEYS_FLAGS,
            _: &POINTL,
            effect: *mut DROPEFFECT,
        ) -> Result<()> {
            self.valid.set(!paths(data).is_empty());
            unsafe {
                *effect = if self.valid.get() {
                    DROPEFFECT_COPY
                } else {
                    DROPEFFECT_NONE
                };
            }
            Ok(())
        }
        fn DragOver(
            &self,
            _: MODIFIERKEYS_FLAGS,
            _: &POINTL,
            effect: *mut DROPEFFECT,
        ) -> Result<()> {
            unsafe {
                *effect = if self.valid.get() {
                    DROPEFFECT_COPY
                } else {
                    DROPEFFECT_NONE
                };
            }
            Ok(())
        }
        fn DragLeave(&self) -> Result<()> {
            self.valid.set(false);
            Ok(())
        }
        fn Drop(
            &self,
            data: Option<&IDataObject>,
            _: MODIFIERKEYS_FLAGS,
            _: &POINTL,
            effect: *mut DROPEFFECT,
        ) -> Result<()> {
            let paths = paths(data);
            unsafe {
                *effect = if paths.is_empty() {
                    DROPEFFECT_NONE
                } else {
                    DROPEFFECT_COPY
                };
            }
            if !paths.is_empty() {
                (self.receive)(paths);
            }
            self.valid.set(false);
            Ok(())
        }
    }

    pub fn refresh(window: &tauri::WebviewWindow, receive: Receiver) {
        if !OLE_READY.with(|ready| *ready) {
            return;
        }
        let Ok(root) = window.hwnd() else {
            return;
        };
        let root = HWND(root.0 as isize);
        let mut handles = vec![root];
        unsafe extern "system" fn collect(hwnd: HWND, value: LPARAM) -> BOOL {
            (*(value.0 as *mut Vec<HWND>)).push(hwnd);
            BOOL(1)
        }
        unsafe {
            let _ = EnumChildWindows(root, Some(collect), LPARAM(&mut handles as *mut _ as isize));
        }
        TARGETS.with(|targets| {
            let mut targets=targets.borrow_mut();
            targets.retain(|handle,_|unsafe{IsWindow(HWND(*handle)).as_bool()});
            for hwnd in handles {
                if targets.contains_key(&hwnd.0){continue;}
                let target:IDropTarget=FileTarget{valid:Cell::new(false),receive:receive.clone()}.into();
                unsafe {
                    let _=RevokeDragDrop(hwnd);
                    match RegisterDragDrop(hwnd,&target) {
                        Ok(())=>{targets.insert(hwnd.0,target);tracing::debug!(window=window.label(),"registered native file drop target");}
                        Err(error)=>tracing::debug!(window=window.label(),%error,"file drop target registration failed"),
                    }
                }
            }
        });
    }
    #[cfg(test)]
    mod tests {
        use super::*;
        use std::{os::windows::ffi::OsStrExt, sync::Mutex};
        use windows::{core::PCWSTR, Win32::{System::Ole::{OleInitialize, OleUninitialize}, UI::Shell::{SHCreateItemFromParsingName, IShellItem, BHID_DataObject}}};

        #[test]
        fn shell_file_and_folder_objects_are_accepted_without_moving_sources() {
            unsafe { OleInitialize(None).unwrap(); }
            struct OleGuard;
            impl Drop for OleGuard { fn drop(&mut self) { unsafe { OleUninitialize(); } } }
            let _guard=OleGuard;
            let temp=tempfile::tempdir().unwrap();
            let file=temp.path().join("fixture.txt");
            let folder=temp.path().join("fixture-folder");
            std::fs::write(&file,"unchanged").unwrap();
            std::fs::create_dir(&folder).unwrap();
            for path in [&file,&folder] {
                let text:Vec<u16>=path.as_os_str().encode_wide().chain(Some(0)).collect();
                let item:IShellItem=unsafe { SHCreateItemFromParsingName(PCWSTR(text.as_ptr()),None).unwrap() };
                let data:IDataObject=unsafe { item.BindToHandler(None,&BHID_DataObject).unwrap() };
                let received=Arc::new(Mutex::new(Vec::new()));
                let output=received.clone();
                let target=FileTarget{valid:Cell::new(false),receive:Arc::new(move|paths|*output.lock().unwrap()=paths)};
                let point=POINTL{x:0,y:0};
                let mut effect=DROPEFFECT_NONE;
                target.DragEnter(Some(&data),MODIFIERKEYS_FLAGS(0),&point,&mut effect).unwrap();
                assert_eq!(effect,DROPEFFECT_COPY);
                target.Drop(Some(&data),MODIFIERKEYS_FLAGS(0),&point,&mut effect).unwrap();
                assert_eq!(effect,DROPEFFECT_COPY);
                assert_eq!(received.lock().unwrap()[0].canonicalize().unwrap(),path.canonicalize().unwrap());
                assert!(path.exists());
            }
            assert_eq!(std::fs::read_to_string(file).unwrap(),"unchanged");
        }
    }


}

#[cfg(windows)]
pub use native::refresh;
