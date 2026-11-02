use std::path::{Path, PathBuf};

fn rust_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for entry in std::fs::read_dir(root).expect("source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            files.extend(rust_files(&path));
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files
}

#[test]
fn services_runtime_and_infrastructure_do_not_depend_on_ipc() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    for module in ["services", "runtime", "infrastructure"] {
        for path in rust_files(&root.join(module)) {
            let source = std::fs::read_to_string(&path).expect("Rust source");
            assert!(
                !source.contains("crate::commands") && !source.contains("super::commands"),
                "{} must not depend on the IPC adapter",
                path.display()
            );
            assert!(
                !source.contains("#[tauri::command]"),
                "{} must not declare IPC commands",
                path.display()
            );
        }
    }
}
