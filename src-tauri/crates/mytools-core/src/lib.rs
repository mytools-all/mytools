//! Pure-Rust core logic for MyTools — testable without a Tauri runtime.
//!
//! This crate deliberately depends on **no Tauri types** so that
//! `cargo test -p mytools-core` works on any host (CI, devcontainer,
//! bare GitHub Actions runner). The Tauri app shell re-exports these
//! modules under `src-tauri/src/{plugin,store,...}`.
//!
//! Modules grow as v0.1 features come online:
//!   - manifest:  plugin manifest parsing (native + Rubick schema)
//!   - db:        sqlite connection helpers + schema migrations
//!   - search:    fuzzy match scoring
//!   - panel:     launcher window geometry + global hotkey constants
//!   - calculator: math expression evaluator (`=` prefix)
//!   - builtin:   first-class command catalog (calculator / JSON / …)
//!   - capability: capability registry, permission policy (foundation layer)

pub mod builtin;
pub mod calculator;
pub mod capability;
pub mod clipboard;
pub mod color;
pub mod db;
pub mod manifest;
pub mod panel;
pub mod permission;
pub mod plugin_loader;
pub mod plugin_mgmt;
pub mod search;
pub mod startmenu;

pub use clipboard::{ClipboardEntry, ClipboardError, ClipboardStore, MAX_ENTRIES};
pub use plugin_loader::{find_plugins_root, scan_dir, scan_dir_enabled, LoadedPlugin, LoaderError};
pub use plugin_mgmt::{
    archive_install_name, first_safe_name, install_from_dir, install_from_tar_gz, install_from_zip,
    is_safe_plugin_name, read_manifest, resolve_install_name, safe_plugin_dir, uninstall_plugin,
    PluginMgrError, PluginState, PLUGIN_MANIFEST_FILE, PLUGIN_STATE_FILE,
};
pub use startmenu::{scan as scan_start_menu, StartMenuEntry, StartMenuSource};

pub use builtin::{try_inline_calculator, BuiltinCommand, BuiltinKind, BUILTIN_COMMANDS};
pub use capability::{
    validate_capability_id, CallContext, Capability, CapabilityCall, CapabilityError,
    CapabilityRegistry, Decision, DefaultPolicy, GrantPolicy, Handler, Origin, Permission, Policy,
    HOST_API_VERSION,
};
pub use color::{parse, parse_hex, parse_rgb, ColorError, Rgb};
pub use manifest::{ManifestError, PluginFeature, PluginManifest};
pub use panel::{PANEL_HEIGHT, PANEL_TOP_MARGIN, PANEL_WIDTH, TOGGLE_SHORTCUT_STR};
pub use permission::{Grant, GrantFile, GrantStore, PERMISSION_FILE};
pub use search::{fuzzy_match, FuzzyHit};

pub mod dir_grants;
pub mod plugin_session;
