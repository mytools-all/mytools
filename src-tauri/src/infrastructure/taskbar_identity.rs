use std::path::{Path, PathBuf};

pub fn materialize_icon(root: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let fingerprint = bytes.iter().fold(0xcbf29ce484222325u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    });
    std::fs::create_dir_all(root).map_err(|error| error.to_string())?;
    let path = root.join(format!("mytools-{fingerprint:016x}.ico"));
    if std::fs::read(&path).ok().as_deref() != Some(bytes) {
        std::fs::write(&path, bytes).map_err(|error| error.to_string())?;
    }
    Ok(path)
}

#[cfg(windows)]
pub fn apply(hwnd: isize, identifier: &str, icon: &Path) -> Result<(), String> {
    use windows::{
        core::PROPVARIANT,
        Win32::{
            Foundation::{HWND, RPC_E_CHANGED_MODE},
            Storage::EnhancedStorage::{
                PKEY_AppUserModel_ID, PKEY_AppUserModel_RelaunchCommand,
                PKEY_AppUserModel_RelaunchDisplayNameResource,
                PKEY_AppUserModel_RelaunchIconResource,
            },
            System::{
                Com::{
                    CoInitializeEx, CoUninitialize,
                    StructuredStorage::{PropVariantChangeType, PVCHF_DEFAULT},
                    COINIT_APARTMENTTHREADED,
                },
                Variant::VT_LPWSTR,
            },
            UI::Shell::PropertiesSystem::{IPropertyStore, SHGetPropertyStoreForWindow},
        },
    };
    let initialized = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if initialized.is_err() && initialized != RPC_E_CHANGED_MODE {
        return Err(initialized.to_string());
    }
    struct Apartment(bool);
    impl Drop for Apartment {
        fn drop(&mut self) {
            if self.0 {
                unsafe { CoUninitialize() };
            }
        }
    }
    let _apartment = Apartment(initialized.is_ok());
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let values = [
        (
            PKEY_AppUserModel_RelaunchIconResource,
            format!("{},0", icon.display()),
        ),
        (
            PKEY_AppUserModel_RelaunchCommand,
            format!("\"{}\"", exe.display()),
        ),
        (
            PKEY_AppUserModel_RelaunchDisplayNameResource,
            "MyTools".into(),
        ),
        (PKEY_AppUserModel_ID, identifier.into()),
    ];
    let store: IPropertyStore =
        unsafe { SHGetPropertyStoreForWindow(HWND(hwnd)) }.map_err(|error| error.to_string())?;
    for (key, text) in values {
        let source = PROPVARIANT::from(text.as_str());
        let mut value = PROPVARIANT::new();
        unsafe {
            PropVariantChangeType(&mut value, &source, PVCHF_DEFAULT, VT_LPWSTR)
                .map_err(|error| error.to_string())?;
            store
                .SetValue(&key, &value)
                .map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn icon_source_changes_with_content_and_repairs_corrupt_files() {
        let root = tempfile::tempdir().unwrap();
        let first = materialize_icon(root.path(), b"first").unwrap();
        assert_eq!(materialize_icon(root.path(), b"first").unwrap(), first);
        let second = materialize_icon(root.path(), b"second").unwrap();
        assert_ne!(first, second);
        assert_eq!(std::fs::read(&second).unwrap(), b"second");
        std::fs::write(&first, b"corrupt").unwrap();
        materialize_icon(root.path(), b"first").unwrap();
        assert_eq!(std::fs::read(&first).unwrap(), b"first");
    }
}
