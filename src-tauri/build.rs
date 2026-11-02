use std::path::{Path, PathBuf};
use std::process::Command;

use tauri_build::WindowsAttributes;

fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("gnu")
    {
        copy_webview_loader_gnu();
    }

    tauri_build::try_build(
        tauri_build::Attributes::new()
            .windows_attributes(WindowsAttributes::new_without_app_manifest()),
    )
    .expect("failed to run tauri-build");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        match std::env::var("CARGO_CFG_TARGET_ENV").as_deref() {
            Ok("msvc") => embed_manifest_msvc(),
            Ok("gnu") => embed_manifest_gnu(),
            _ => {}
        }
    }
}

fn embed_manifest_msvc() {
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "\\windows-app-manifest.xml");
    println!("cargo:rerun-if-changed={manifest}");
    println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
    println!("cargo:rustc-link-arg=/MANIFESTINPUT:{manifest}");
}

fn embed_manifest_gnu() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let manifest = concat!(env!("CARGO_MANIFEST_DIR"), "/windows-app-manifest.xml")
        .replace('\\', "/");
    let rc = out_dir.join("app-manifest.rc");
    let obj = out_dir.join("app-manifest.o");
    std::fs::write(&rc, format!("1 24 \"{manifest}\"\n")).expect("write rc file");
    let status = Command::new("windres")
        .arg(rc.to_str().unwrap())
        .args(["-o", obj.to_str().unwrap()])
        .status()
        .expect("failed to run windres");
    assert!(
        status.success(),
        "windres failed to compile the app manifest"
    );
    println!("cargo:rerun-if-changed={manifest}");
    println!("cargo:rustc-link-arg={}", obj.display());
}

/// MinGW links `webview2-com` against `WebView2Loader.dll` at runtime, so
/// the DLL must sit next to the installed exe. Copy a valid loader DLL into
/// `CARGO_MANIFEST_DIR/resources/` (consumed by `bundle.resources`).
fn copy_webview_loader_gnu() {
    let dest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("resources");
    let dest = dest_dir.join("WebView2Loader.dll");
    if let Ok(md) = std::fs::metadata(&dest) {
        if md.len() >= 1_000 {
            return;
        }
    }
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let target_dir = out_dir.join("../../../..");
    let sources = [
        find_webview_loader(&target_dir),
        std::env::var_os("CARGO_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(PathBuf::from))
            .and_then(|home| find_webview_loader(&home.join("registry/src"))),
    ];
    for src in sources.into_iter().flatten() {
        if let Err(e) = std::fs::create_dir_all(&dest_dir) {
            panic!("failed to create resources dir: {e}");
        }
        match std::fs::copy(&src, &dest) {
            Ok(_) => {
                println!(
                    "cargo:warning=webview2 loader copied from {}",
                    src.display()
                );
                return;
            }
            Err(e) => {
                eprintln!("failed to copy {}: {e}", src.display());
            }
        }
    }
    panic!(
        "WebView2Loader.dll not found under {:?} or cargo registry; \
         the NSIS/MSI bundle would crash at startup without it",
        target_dir
    );
}

/// Depth-limited search for a real (non-stub) `WebView2Loader.dll` produced
/// by `webview2-com-sys` for the x64 target.
fn find_webview_loader(root: &Path) -> Option<PathBuf> {
    let mut best: Option<PathBuf> = None;
    let mut best_len = 0u64;
    let mut stack: Vec<(PathBuf, usize)> = vec![(root.to_path_buf(), 0)];
    while let Some((dir, depth)) = stack.pop() {
        if depth > 8 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                stack.push((path, depth + 1));
                continue;
            }
            if entry.file_name().to_string_lossy() != "WebView2Loader.dll" {
                continue;
            }
            let os = path.to_string_lossy();
            if !(os.contains("webview2-com-sys") && os.contains("x64")) {
                continue;
            }
            let Ok(md) = entry.metadata() else {
                continue;
            };
            if md.len() < 1_000 || md.len() <= best_len {
                continue;
            }
            best = Some(path);
            best_len = md.len();
        }
    }
    best
}
