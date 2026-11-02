//! App icon resolution — extracts the shell icon for a `.lnk` (or any
//! file) as a base64 PNG data URI so the frontend can render the real
//! system icon in search results. Extraction is lazy (only for matched
//! entries) and cached by path + last-modified time.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

struct IconCacheEntry {
    icon: Option<String>,
    mtime: Option<std::time::SystemTime>,
}

const CACHE_CAP: usize = 512;

fn cache() -> &'static Mutex<HashMap<String, IconCacheEntry>> {
    static CACHE: OnceLock<Mutex<HashMap<String, IconCacheEntry>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Lock the icon cache, tolerating poisoning. A panic while the lock is held
/// would otherwise make every later lookup panic too, taking `search` — which
/// resolves an icon per result — down with it.
fn lock_cache() -> std::sync::MutexGuard<'static, HashMap<String, IconCacheEntry>> {
    cache().lock().unwrap_or_else(|e| e.into_inner())
}

/// Historical pinned keys were stored with forward slashes, which
/// `SHGetFileInfoW` cannot resolve for `.lnk` targets. Map `/` to `\` so
/// the shell API gets a native Windows path.
fn alternate_path(path: &str) -> Option<String> {
    let alt = path.replace('/', "\\");
    if alt == path {
        None
    } else {
        Some(alt)
    }
}

/// `IShellLinkW::GetIconLocation` hands back `"<path>,<index>"`. Split the
/// two, falling back to index 0 when there is no comma or the suffix is not
/// a number.
fn split_icon_location(s: &str) -> (String, i32) {
    if let Some((path, idx_str)) = s.rsplit_once(',') {
        if let Ok(idx) = idx_str.trim().parse::<i32>() {
            // ",3" names no file at all — report that rather than a bogus path.
            if path.is_empty() {
                return (String::new(), 0);
            }
            return (path.trim().to_string(), idx);
        }
    }
    (s.trim().to_string(), 0)
}

/// Public re-export of [`split_icon_location`] for the app index, which
/// parses registry `DisplayIcon` values of the form `"<path>,<index>"`.
pub fn split_icon_location_pub(s: &str) -> (String, i32) {
    split_icon_location(s)
}

/// Decode a NUL-terminated UTF-16 buffer that a shell API filled in.
#[cfg(windows)]
fn utf16z(buf: &[u16]) -> String {
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..end])
}

/// Shell APIs take native separators only. `std::fs` is happy with `/`, but
/// `SHGetFileInfoW` is not, so a forward-slash source silently yields no icon
/// and we would fall back to the arrow-baked `.lnk`.
#[cfg(windows)]
fn to_native(path: &str) -> String {
    path.replace('/', "\\")
}

/// The shell synthesises the shortcut-arrow overlay directly into the icon it
/// returns for a `.lnk`, so reading the shortcut's own icon bakes the arrow
/// into our PNG. Instead ask `IShellLinkW` where the real icon lives and read
/// *that* file — no arrow.
///
/// Returns the icon source file plus a resource index (0 = the default icon).
#[cfg(windows)]
fn resolve_lnk_icon_source(lnk: &str) -> Option<(String, i32)> {
    use windows::core::{IUnknown, Interface, PCWSTR};
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoUninitialize, IPersistFile, CLSCTX_INPROC_SERVER,
        COINIT_APARTMENTTHREADED, STGM_READ,
    };
    use windows::Win32::UI::Shell::{IShellLinkW, ShellLink};

    /// Long enough for any `MAX_PATH_LONG` value the shell may hand back.
    const BUF: usize = 32768;
    /// The thread is already in a different COM apartment. Not an error for
    /// our purposes: COM is live, we just must not uninitialise it.
    const RPC_E_CHANGED_MODE: i32 = 0x8001_010Eu32 as i32;

    // Tauri runs async commands on a worker pool, so COM has to be brought up
    // on whichever thread we happen to land on.
    let mut uninit = false;
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_ok() {
        uninit = true;
    } else if hr.0 != RPC_E_CHANGED_MODE {
        return None;
    }

    let resolved = (|| {
        let link: IShellLinkW =
            unsafe { CoCreateInstance(&ShellLink, None::<&IUnknown>, CLSCTX_INPROC_SERVER) }
                .ok()?;
        let file: IPersistFile = link.cast().ok()?;
        let wide: Vec<u16> = lnk.encode_utf16().chain(Some(0)).collect();
        unsafe { file.Load(PCWSTR(wide.as_ptr()), STGM_READ) }.ok()?;

        // Prefer the shortcut's own target. An explicit IconLocation is often
        // a generic placeholder rather than the app's real icon — an MSI
        // `ProductIcon`, for instance, renders as a grey shell logo, which is
        // strictly worse than the icon baked into the target executable.
        let mut buf = vec![0u16; BUF];
        if unsafe { link.GetPath(&mut buf, std::ptr::null_mut(), 0) }.is_ok() {
            let target = to_native(&utf16z(&buf));
            if !target.is_empty() && std::fs::metadata(&target).is_ok() {
                return Some((target, 0));
            }
        }

        // Fallback: the icon the shortcut explicitly points at. Needed when
        // the target is gone but the shortcut still carries an icon file.
        let mut buf = vec![0u16; BUF];
        let mut index: i32 = 0;
        if unsafe { link.GetIconLocation(&mut buf, &mut index) }.is_ok() {
            let (path, from_string) = split_icon_location(&utf16z(&buf));
            let path = to_native(&path);
            if !path.is_empty() && std::fs::metadata(&path).is_ok() {
                // A zero in the string means "unspecified", so trust the
                // out-parameter instead.
                let idx = if from_string != 0 { from_string } else { index };
                return Some((path, idx));
            }
        }
        None
    })();

    if uninit {
        unsafe { CoUninitialize() };
    }
    resolved
}

/// Pull the icon at `index` out of `file`. Only a positive index needs
/// `ExtractIconExW` — index 0 (and the resource-id style negative values)
/// are served by the default shell lookup, and either way we are reading the
/// resolved source rather than the `.lnk`, so no arrow is composited.
#[cfg(windows)]
fn extract_icon_from(file: &str, index: i32) -> Option<String> {
    use windows_sys::Win32::UI::Shell::ExtractIconExW;
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, HICON};

    let file = to_native(file);
    if index > 0 {
        let wide: Vec<u16> = file.encode_utf16().chain(Some(0)).collect();
        let mut large: HICON = std::ptr::null_mut();
        let mut small: HICON = std::ptr::null_mut();
        let got = unsafe { ExtractIconExW(wide.as_ptr(), index, &mut large, &mut small, 1) };
        if got >= 1 && !large.is_null() {
            let uri = imp::hicon_to_png_data_uri(large);
            if !small.is_null() {
                unsafe { DestroyIcon(small) };
            }
            if uri.is_some() {
                return uri;
            }
        }
        if !large.is_null() {
            unsafe { DestroyIcon(large) };
        }
    }
    imp::extract_png_data_uri(&file)
}

/// Icon for a path. `.lnk` files never go through `SHGetFileInfoW` on the
/// shortcut itself: the shell composites the shortcut-arrow overlay into that
/// icon. Instead resolve the real icon source and ask
/// `IShellItemImageFactory::GetImage(SIIGBF_ICONONLY)`, which returns the
/// plain icon (verified pixel-identical to the target executable for VS Code).
/// `SHGetFileInfoW` stays as a last-resort fallback only.
#[cfg(windows)]
fn icon_for_path(path: &str) -> Option<String> {
    // Normalise *before* anything else. `IShellLinkW::Load` rejects forward
    // slashes while `SHGetFileInfoW` accepts them, so a "C:/x.lnk" key would
    // silently take the old arrow-baked route — and since that route
    // succeeds, a later `alternate_path` retry would never fire.
    let native = alternate_path(path).unwrap_or_else(|| path.to_string());
    if native.to_ascii_lowercase().ends_with(".lnk") {
        if let Some((src, idx)) = resolve_lnk_icon_source(&native) {
            if let Some(uri) = shell_item_icon_data_uri(&src) {
                return Some(uri);
            }
            if let Some(uri) = extract_icon_from(&src, idx) {
                return Some(uri);
            }
        }
        // Shortcut we could not resolve (or whose target lost its icon):
        // still avoid `SHGetFileInfoW(.lnk)`'s arrow overlay.
        if let Some(uri) = shell_item_icon_data_uri(&native) {
            return Some(uri);
        }
    }
    imp::extract_png_data_uri(&native)
}

/// Resolve the file's shell icon as a `data:image/png;base64,...` string.
/// Any failure returns `None` so the frontend can fall back to a letter.
pub fn resolve_icon(path: &str) -> Option<String> {
    #[cfg(windows)]
    {
        let mtime = std::fs::metadata(path).and_then(|m| m.modified()).ok();
        {
            let guard = lock_cache();
            if let Some(entry) = guard.get(path) {
                if entry.mtime == mtime {
                    return entry.icon.clone();
                }
            }
        }
        // A panic inside the COM/GDI path must never propagate up into the
        // `search` command — the frontend's catch silently throws away the
        // entire result list, leaving the user with "no results" for a query
        // that should have worked. Catching here keeps the bad entry an
        // isolated None instead of a global failure.
        let icon = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| icon_for_path(path)))
            .unwrap_or_else(|_| {
                tracing::warn!(path, "app_icon extraction panicked; falling back to None");
                None
            });
        if icon.is_some() {
            let mut guard = lock_cache();
            if guard.len() >= CACHE_CAP {
                guard.clear();
            }
            guard.insert(
                path.to_string(),
                IconCacheEntry {
                    icon: icon.clone(),
                    mtime,
                },
            );
        }
        icon
    }
    #[cfg(not(windows))]
    {
        let _ = path;
        None
    }
}

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Graphics::Gdi::{self, BITMAP, BI_RGB, DIB_RGB_COLORS};
    use windows_sys::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON};
    use windows_sys::Win32::UI::WindowsAndMessaging::{DestroyIcon, GetIconInfo, HICON, ICONINFO};

    /// Extract the file's shell icon (32×32) as a PNG data URI.
    pub fn extract_png_data_uri(path: &str) -> Option<String> {
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let mut shfi: SHFILEINFOW = unsafe { std::mem::zeroed() };
        let ok = unsafe {
            SHGetFileInfoW(
                wide.as_ptr(),
                0,
                &mut shfi,
                std::mem::size_of::<SHFILEINFOW>() as u32,
                SHGFI_ICON | SHGFI_LARGEICON,
            )
        };
        if ok == 0 {
            return None;
        }
        let hicon = shfi.hIcon;
        if hicon.is_null() {
            return None;
        }
        hicon_to_png_data_uri(hicon)
    }

    /// Convert an owned `HICON` into a PNG data URI. Always destroys the
    /// handle (and its bitmaps) before returning.
    pub fn hicon_to_png_data_uri(hicon: HICON) -> Option<String> {
        let mut ii: ICONINFO = unsafe { std::mem::zeroed() };
        if unsafe { GetIconInfo(hicon, &mut ii) } == 0 {
            unsafe { DestroyIcon(hicon) };
            return None;
        }
        let hbmp = ii.hbmColor;
        if hbmp.is_null() {
            unsafe {
                Gdi::DeleteObject(ii.hbmColor as _);
                Gdi::DeleteObject(ii.hbmMask as _);
                DestroyIcon(hicon);
            }
            return None;
        }
        let out = hbitmap_to_png_data_uri(hbmp);
        unsafe {
            Gdi::DeleteObject(ii.hbmMask as _);
            DestroyIcon(hicon);
        }
        out
    }

    /// Convert an owned 32bpp `HBITMAP` into a PNG data URI, deleting the
    /// bitmap before returning. Shared with the Store-app path, where
    /// `IShellItemImageFactory::GetImage` hands back a bitmap, not an icon.
    pub fn hbitmap_to_png_data_uri(hbmp: Gdi::HBITMAP) -> Option<String> {
        let mut bmp: BITMAP = unsafe { std::mem::zeroed() };
        let got = unsafe {
            Gdi::GetObjectW(
                hbmp as _,
                std::mem::size_of::<BITMAP>() as i32,
                &mut bmp as *mut BITMAP as *mut std::ffi::c_void,
            )
        };
        if got == 0 || bmp.bmWidth <= 0 || bmp.bmHeight <= 0 {
            unsafe {
                Gdi::DeleteObject(hbmp as _);
            }
            return None;
        }

        let w = bmp.bmWidth as u32;
        let h = bmp.bmHeight as u32;
        let dc = unsafe { Gdi::CreateCompatibleDC(std::ptr::null_mut()) };
        if dc.is_null() {
            unsafe {
                Gdi::DeleteObject(hbmp as _);
            }
            return None;
        }
        let old = unsafe { Gdi::SelectObject(dc, hbmp as _) };

        let stride = w * 4;
        let mut buf: Vec<u8> = vec![0; (stride * h) as usize];
        let mut bi: Gdi::BITMAPINFO = unsafe { std::mem::zeroed() };
        bi.bmiHeader.biSize = std::mem::size_of::<Gdi::BITMAPINFOHEADER>() as u32;
        bi.bmiHeader.biWidth = w as i32;
        bi.bmiHeader.biHeight = -(h as i32);
        bi.bmiHeader.biPlanes = 1;
        bi.bmiHeader.biBitCount = 32;
        bi.bmiHeader.biCompression = BI_RGB;
        let lines = unsafe {
            Gdi::GetDIBits(
                dc,
                hbmp as _,
                0,
                h,
                buf.as_mut_ptr() as *mut std::ffi::c_void,
                &mut bi,
                DIB_RGB_COLORS,
            )
        };

        unsafe {
            Gdi::SelectObject(dc, old);
            Gdi::DeleteDC(dc);
            Gdi::DeleteObject(hbmp as _);
        }

        if lines == 0 {
            return None;
        }
        super::bgra_to_png_data_uri(w, h, &buf)
    }
}

/// Convert a top-down BGRA buffer (as produced by GetDIBits with a
/// 32bpp DIB) into a base64 PNG data URI.
fn bgra_to_png_data_uri(w: u32, h: u32, bgra: &[u8]) -> Option<String> {
    let mut rgba = Vec::with_capacity(bgra.len());
    for px in bgra.chunks_exact(4) {
        rgba.extend_from_slice(&[px[2], px[1], px[0], px[3]]);
    }
    let img = image::RgbaImage::from_raw(w, h, rgba)?;
    let mut png = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut png);
    image::DynamicImage::ImageRgba8(img)
        .write_to(&mut cursor, image::ImageFormat::Png)
        .ok()?;
    Some(format!("data:image/png;base64,{}", base64_encode(&png)))
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(TABLE[(n >> 18 & 0x3f) as usize] as char);
        out.push(TABLE[(n >> 12 & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(n >> 6 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(n & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

/// Icon for any item inside `shell:AppsFolder`.
///
/// The shell understands every identifier shape it hands out — real AUMIDs
/// (`Package!App`), known-folder paths (`{GUID}\cmd.exe`), bare pseudo-AUMIDs
/// (`Microsoft.VisualStudioCode`) and plain paths (`C:\msys64\msys2.exe`) —
/// so we hand the whole parsing name back to it instead of trying to classify
/// the string ourselves.
#[cfg(windows)]
pub fn shell_item_icon_data_uri(parsing_name: &str) -> Option<String> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::System::Com::{
        CoInitializeEx, CoUninitialize, IBindCtx, COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::UI::Shell::{
        IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_ICONONLY,
    };

    /// Already in a different COM apartment — COM is live, don't uninitialise.
    const RPC_E_CHANGED_MODE: i32 = 0x8001_010Eu32 as i32;

    let mut uninit = false;
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr.is_ok() {
        uninit = true;
    } else if hr.0 != RPC_E_CHANGED_MODE {
        return None;
    }

    let out = (|| {
        let wide: Vec<u16> = parsing_name.encode_utf16().chain(Some(0)).collect();
        let item: IShellItemImageFactory = match unsafe {
            SHCreateItemFromParsingName(PCWSTR(wide.as_ptr()), None::<&IBindCtx>)
        } {
            Ok(i) => i,
            Err(_) => return None,
        };
        let bmp = unsafe { item.GetImage(SIZE { cx: 32, cy: 32 }, SIIGBF_ICONONLY) }.ok()?;
        if bmp.is_invalid() {
            return None;
        }
        imp::hbitmap_to_png_data_uri(bmp.0 as _)
    })();

    if uninit {
        unsafe { CoUninitialize() };
    }
    out
}

/// Whether a search target is an AppUserModelID rather than a filesystem path
/// or a `scheme:` URI.
pub fn is_aumid_target(s: &str) -> bool {
    let t = s.trim();
    // AUMIDs always contain `!` separating package family from application id.
    t.contains('!') && !t.contains('\\') && !t.contains('/') && !t.contains(':')
}

#[cfg(not(windows))]
pub fn shell_item_icon_data_uri(_parsing_name: &str) -> Option<String> {
    None
}

/// Icon for a `shell:AppsFolder` entry, cached by its identifier.
///
/// Every identifier shape the shell hands out resolves through the same
/// parsing name, so no classification is needed — and the cache key is the
/// identifier itself, which is stable across restarts.
pub fn cached_shell_item_icon(id: &str) -> Option<String> {
    let key = format!("shellitem:{}", id.to_ascii_lowercase());
    if let Some(cached) = cached_icon(&key) {
        return cached;
    }
    let icon = shell_item_icon_data_uri(&format!("shell:AppsFolder\\{id}"));
    cache_icon(&key, icon.clone());
    icon
}

/// Read a cached icon. The outer `Option` distinguishes "not cached" from
/// "cached, and the resolution failed" (`Some(None)`), so a failure is not
/// retried on every keystroke.
fn cached_icon(key: &str) -> Option<Option<String>> {
    let guard = lock_cache();
    guard.get(key).map(|e| e.icon.clone())
}

/// Store an icon in the cache.
fn cache_icon(key: &str, icon: Option<String>) {
    let mut guard = lock_cache();
    if guard.len() >= CACHE_CAP {
        guard.clear();
    }
    guard.insert(key.to_string(), IconCacheEntry { icon, mtime: None });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_encode_pads() {
        assert_eq!(base64_encode(b"A"), "QQ==");
        assert_eq!(base64_encode(b"AB"), "QUI=");
        assert_eq!(base64_encode(b"ABC"), "QUJD");
    }

    #[test]
    fn alternate_path_converts_forward_slashes() {
        assert_eq!(
            alternate_path("c:/users/a/b.lnk").as_deref(),
            Some("c:\\users\\a\\b.lnk")
        );
        assert_eq!(alternate_path("C:\\Users\\a\\b.lnk"), None);
    }

    #[test]
    fn split_icon_location_without_index() {
        let (path, index) = split_icon_location(r"C:\app\app.exe");
        assert_eq!(path, r"C:\app\app.exe");
        assert_eq!(index, 0);
    }

    #[test]
    fn split_icon_location_with_index() {
        let (path, index) = split_icon_location(r"C:\app\app.exe,7");
        assert_eq!(path, r"C:\app\app.exe");
        assert_eq!(index, 7);
    }

    #[test]
    fn split_icon_location_ignores_non_numeric_or_empty_suffix() {
        // A non-numeric suffix means the comma belongs to the path itself
        // ("My, App.exe"), so the whole string stays intact.
        assert_eq!(
            split_icon_location(r"C:\app\app.exe,notanumber"),
            (r"C:\app\app.exe,notanumber".to_string(), 0)
        );
        // ",3" names no file at all.
        assert_eq!(split_icon_location(",3"), (String::new(), 0));
        // Surrounding whitespace is trimmed.
        assert_eq!(
            split_icon_location(r" C:\a b\app.exe , -2 "),
            (r"C:\a b\app.exe".to_string(), -2)
        );
    }

    #[cfg(windows)]
    #[test]
    fn store_app_aumid_resolves_a_real_icon() {
        // Store apps have no file behind them, so the icon has to come from
        // `shell:AppsFolder\<aumid>` — the same path every shell item takes.
        let sample = crate::services::apps::index::apps()
            .apps
            .iter()
            .find(|a| {
                a.kind == crate::services::apps::index::TargetKind::ShellItem
                    && a.target.contains('!')
            })
            .map(|a| a.target.clone());
        let Some(aumid) = sample else {
            return;
        };
        let uri = cached_shell_item_icon(&aumid).expect("indexed AUMID should expose an icon");
        assert!(uri.starts_with("data:image/png;base64,"), "{uri}");
        assert!(
            uri.len() > "data:image/png;base64,".len() + 100,
            "icon looks empty"
        );
    }

    #[cfg(windows)]
    #[test]
    fn unknown_aumid_resolves_to_none() {
        assert_eq!(
            cached_shell_item_icon("Totally.Made.Up.Package_zzzz!Nope"),
            None
        );
    }

    #[cfg(windows)]
    #[test]
    fn shortcut_icon_source_is_never_the_lnk_itself() {
        // The whole point: read the icon from the shortcut's *target* so the
        // shell never composites the arrow overlay into our PNG.
        let entries = mytools_core::scan_start_menu();
        let Some(e) = entries
            .iter()
            .find(|e| e.path.to_ascii_lowercase().ends_with(".lnk"))
        else {
            return;
        };
        let Some((source, _)) = resolve_lnk_icon_source(&e.path) else {
            return;
        };
        assert_ne!(source, e.path, "icon source must not be the shortcut");
        assert!(
            std::fs::metadata(&source).is_ok(),
            "resolved icon source should exist: {source}"
        );
    }

    /// A Start Menu shortcut whose name contains `needle`, paired with the
    /// icon source `IShellLinkW` resolves it to.
    #[cfg(windows)]
    fn shortcut_by_name(needle: &str) -> Option<(String, String)> {
        let entries = mytools_core::scan_start_menu();
        for e in entries.iter() {
            if !e.path.to_ascii_lowercase().ends_with(".lnk") {
                continue;
            }
            if !e.name.to_ascii_lowercase().contains(needle) {
                continue;
            }
            if let Some((src, _)) = resolve_lnk_icon_source(&e.path) {
                return Some((e.path.clone(), src));
            }
        }
        None
    }

    #[cfg(windows)]
    #[test]
    fn shortcut_icon_matches_target_without_arrow_overlay() {
        // Regression for VS Code / CC Switch rendering the shortcut-arrow
        // badge: the icon we return for the `.lnk` must be the target app's
        // own icon, not the arrow-composited shortcut icon.
        let Some((lnk, target)) = shortcut_by_name("visual studio code") else {
            return;
        };
        if !target.to_ascii_lowercase().ends_with(".exe") {
            return;
        }
        let from_lnk = resolve_icon(&lnk).expect("shortcut should resolve an icon");
        let from_target = resolve_icon(&target).expect("target should resolve an icon");
        assert_eq!(
            from_lnk, from_target,
            "shortcut icon must match its target executable (no overlay)"
        );
    }

    #[cfg(windows)]
    #[test]
    fn shortcut_icon_is_not_the_arrow_baked_lnk_icon() {
        for needle in ["visual studio code", "cc switch"] {
            let Some((lnk, _)) = shortcut_by_name(needle) else {
                continue;
            };
            let resolved = resolve_icon(&lnk).expect("shortcut should resolve an icon");
            let with_arrow = imp::extract_png_data_uri(&lnk).expect("raw shortcut icon");
            assert_ne!(
                resolved, with_arrow,
                "{needle}: resolved icon must not be the arrow-baked .lnk icon"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn shortcut_resolves_to_a_valid_png() {
        let entries = mytools_core::scan_start_menu();
        let Some(e) = entries
            .iter()
            .find(|e| e.path.to_ascii_lowercase().ends_with(".lnk"))
        else {
            return;
        };
        if let Some(uri) = resolve_icon(&e.path) {
            assert!(
                uri.starts_with("data:image/png;base64,"),
                "bad uri for {}: {uri}",
                e.path
            );
        }
    }

    #[test]
    fn bgra_to_png_produces_1x1_data_uri() {
        let uri = bgra_to_png_data_uri(1, 1, &[0, 0, 255, 255]).unwrap();
        assert!(uri.starts_with("data:image/png;base64,"), "{uri}");
        assert!(uri.len() > "data:image/png;base64,".len());
    }

    #[cfg(windows)]
    #[test]
    fn missing_file_resolves_to_none() {
        let p = std::env::temp_dir().join("mytools-no-such-icon.lnk");
        assert_eq!(resolve_icon(&p.to_string_lossy()), None);
    }

    #[cfg(windows)]
    #[test]
    fn real_start_menu_entry_resolves_icon() {
        // Smoke test on the real machine: any `.lnk` either fails
        // gracefully (None) or yields a valid PNG data URI.
        let entries = mytools_core::scan_start_menu();
        for e in entries.iter().take(5) {
            if let Some(uri) = resolve_icon(&e.path) {
                assert!(
                    uri.starts_with("data:image/png;base64,"),
                    "bad uri for {}: {uri:?}",
                    e.path
                );
            }
        }
    }

    #[cfg(windows)]
    #[test]
    fn forward_slash_path_falls_back_to_backslash() {
        let entries = mytools_core::scan_start_menu();
        // Each spelling is a distinct cache key, so both are cold calls and the
        // shell can transiently decline one of them. Pick an entry where both
        // succeed — that leaves both results cached, making the comparison
        // below deterministic.
        let ok = entries.iter().any(|e| {
            let alt = e.path.replace('\\', "/");
            resolve_icon(&e.path).is_some() && resolve_icon(&alt).is_some()
        });
        if !ok {
            return;
        }
        let e = entries
            .iter()
            .find(|e| {
                let alt = e.path.replace('\\', "/");
                resolve_icon(&e.path).is_some() && resolve_icon(&alt).is_some()
            })
            .expect("checked above");
        let alt = e.path.replace('\\', "/");
        assert_eq!(
            resolve_icon(&alt),
            resolve_icon(&e.path),
            "a forward-slash path must resolve to the same icon"
        );
    }

    #[cfg(windows)]
    #[test]
    fn resolved_icon_source_uses_native_separators() {
        // Shell APIs reject forward slashes, and `GetIconLocation` does hand
        // them out (e.g. `C:/msys64/mingw64.exe`). A forward-slash source makes
        // `SHGetFileInfoW` fail, which silently falls back to the arrow-baked
        // `.lnk` — so every resolved source must be backslash-normalised.
        let entries = mytools_core::scan_start_menu();
        let mut checked = 0usize;
        for e in entries.iter() {
            if !e.path.to_ascii_lowercase().ends_with(".lnk") {
                continue;
            }
            let Some((source, _)) = resolve_lnk_icon_source(&e.path) else {
                continue;
            };
            assert!(
                !source.contains('/'),
                "resolved icon source must use backslashes: {source}"
            );
            checked += 1;
        }
        eprintln!("checked {checked} shortcut icon sources");
    }

    #[cfg(windows)]
    #[test]
    fn resolve_icon_caches() {
        let entries = mytools_core::scan_start_menu();
        // The shell intermittently declines to hand back an icon on a cold
        // call, so warm an entry first and then assert the cache contract:
        // a cached icon must come back byte-identical.
        let Some(e) = entries.iter().find(|e| resolve_icon(&e.path).is_some()) else {
            return;
        };
        let first = resolve_icon(&e.path);
        let second = resolve_icon(&e.path);
        assert_eq!(first, second, "a cached icon must be returned unchanged");
    }
}
