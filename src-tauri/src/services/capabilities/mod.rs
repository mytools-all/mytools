mod app;
mod clipboard;
mod fs;
mod net;
mod screen;
mod shell;
mod storage;
mod system;
mod window;

use mytools_core::capability::{CapabilityError, CapabilityRegistry};
use mytools_core::dir_grants::DirGrantStore;
use mytools_core::permission::GrantStore;
use std::sync::Arc;
use tauri::{AppHandle, Runtime};

pub fn build_registry<R: Runtime>(
    app: &AppHandle<R>,
    grants: Arc<GrantStore>,
    dir_grants: Arc<DirGrantStore>,
) -> Result<CapabilityRegistry, CapabilityError> {
    let mut registry = CapabilityRegistry::with_grants(grants);
    clipboard::register(app, &mut registry)?;
    shell::register(app, &mut registry)?;
    system::register(app, &mut registry)?;
    app::register(app, &mut registry)?;
    fs::register(app, &mut registry, dir_grants)?;
    net::register(app, &mut registry)?;
    window::register(app, &mut registry)?;
    storage::register(app, &mut registry)?;
    screen::register(&mut registry)?;
    Ok(registry)
}
