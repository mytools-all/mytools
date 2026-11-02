//! Re-exports from the `mytools-core` workspace member.
//!
//! Plugins installed in the Tauri shell ultimately call these helpers
//! (via Tauri commands in `commands/`). Keeping the re-exports thin
//! means new core modules become available to Tauri commands by just
//! adding them to `mytools-core/src/lib.rs`.

pub use mytools_core::{
    fuzzy_match, try_inline_calculator, BuiltinCommand, BuiltinKind, FuzzyHit, ManifestError,
    PluginFeature, PluginManifest, BUILTIN_COMMANDS,
};
