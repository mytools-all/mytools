use serde_json::{json, Value};

pub fn foreground() -> isize {
    #[cfg(windows)]
    unsafe {
        windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() as isize
    }
    #[cfg(not(windows))]
    {
        0
    }
}

pub fn capture(app: &tauri::AppHandle, foreground: isize) -> Value {
    use tauri_plugin_clipboard_manager::ClipboardExt;
    let files = selected_files(foreground);
    let mut payload = json!({"version":1,"source":"clipboard"});
    if !files.is_empty() {
        payload["files"] = json!(files);
        payload["source"] = json!("explorer");
    }
    if let Ok(image) = app.clipboard().read_image() {
        if image.width().saturating_mul(image.height()) <= 16_000_000 {
            if let Some(pixels) =
                image::RgbaImage::from_raw(image.width(), image.height(), image.rgba().to_vec())
            {
                let mut cursor = std::io::Cursor::new(Vec::new());
                if pixels
                    .write_to(&mut cursor, image::ImageFormat::Png)
                    .is_ok()
                    && cursor.get_ref().len() <= 3 * 1024 * 1024
                {
                    payload["image"] = json!({"mime":"image/png","dataUrl":format!("data:image/png;base64,{}",super::app_icons::base64_encode(cursor.get_ref()))});
                    return payload;
                }
            }
        }
    }
    if let Ok(text) = app.clipboard().read_text() {
        if !text.is_empty() && text.len() <= 1024 * 1024 {
            payload["text"] = json!(text);
        }
    }
    if payload.get("files").is_some() || payload.get("text").is_some() {
        payload
    } else {
        Value::Null
    }
}

#[cfg(not(windows))]
fn selected_files(_: isize) -> Vec<String> {
    Vec::new()
}

#[cfg(windows)]
fn selected_files(foreground: isize) -> Vec<String> {
    use std::{
        os::windows::process::CommandExt,
        process::{Command, Stdio},
        time::{Duration, Instant},
    };
    unsafe {
        let mut class = [0u16; 64];
        let count = windows_sys::Win32::UI::WindowsAndMessaging::GetClassNameW(
            foreground as _,
            class.as_mut_ptr(),
            class.len() as i32,
        );
        let class = String::from_utf16_lossy(&class[..count.max(0) as usize]);
        if class != "CabinetWClass" && class != "ExploreWClass" {
            return Vec::new();
        }
    }
    let script = format!(
        r#"[Console]::OutputEncoding=[System.Text.UTF8Encoding]::new($false);$ErrorActionPreference='Stop';$shell=New-Object -ComObject Shell.Application;$items=@();foreach($window in $shell.Windows()){{if([long]$window.HWND -eq {foreground}){{foreach($item in $window.Document.SelectedItems()){{if($item.IsFileSystem){{$items+= $item.Path;if($items.Count -ge 1000){{break}}}}}};break}}}};ConvertTo-Json -InputObject $items -Compress"#
    );
    let Ok(mut child) = Command::new("powershell.exe")
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .creation_flags(0x08000000)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    else {
        return Vec::new();
    };
    let stdout = child.stdout.take();
    let reader = std::thread::spawn(move || {
        use std::io::Read;
        let mut bytes = Vec::new();
        if let Some(stdout) = stdout {
            let _ = stdout.take(1024 * 1024).read_to_end(&mut bytes);
        }
        bytes
    });
    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started.elapsed() < Duration::from_secs(3) => {
                std::thread::sleep(Duration::from_millis(20))
            }
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return Vec::new();
            }
        }
    }
    let success = child.wait().is_ok_and(|status| status.success());
    let bytes = reader.join().unwrap_or_default();
    if !success {
        return Vec::new();
    }
    serde_json::from_slice(&bytes).unwrap_or_default()
}
