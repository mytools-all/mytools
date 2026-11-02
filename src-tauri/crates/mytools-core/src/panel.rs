//! Launcher panel geometry constants and global hotkey definition.
//!
//! Lived in the Tauri shell originally, but pulling them into core
//! lets the unit tests run on any host (CI runners without WebView2
//! runtime, devcontainers) and gives plugin authors a single source
//! of truth for the panel size they should target when rendering
//! their own UI.
//!
//! See docs/02-architecture.md §4.1 for the workspace split rationale.

/// Panel width in DIPs (device-independent pixels).
pub const PANEL_WIDTH: u32 = 720;

/// Panel height in DIPs.
pub const PANEL_HEIGHT: u32 = 480;

/// Distance from the top of the focused monitor to the panel, in DIPs.
pub const PANEL_TOP_MARGIN: i32 = 80;

/// Default global hotkey string. Tauri parses this on the host via
/// `Shortcut::from_str`, but the parser is not const, so the
/// canonical Alt+Space instance lives in `hotkey::toggle_shortcut`.
pub const TOGGLE_SHORTCUT_STR: &str = "Alt+Space";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panel_dimensions_match_design() {
        assert_eq!(PANEL_WIDTH, 720);
        assert_eq!(PANEL_HEIGHT, 480);
        assert_eq!(PANEL_TOP_MARGIN, 80);
    }

    #[test]
    fn shortcut_string_parses() {
        let parts: Vec<&str> = TOGGLE_SHORTCUT_STR.split('+').collect();
        assert_eq!(parts.len(), 2, "expected single modifier + single key");
        assert!(!parts[0].is_empty(), "modifier must not be empty");
        assert!(!parts[1].is_empty(), "key must not be empty");
    }
}
