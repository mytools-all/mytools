//! Color parsing helpers exposed to the frontend. The actual screen
//! sampler lives in v0.2; v0.1 only accepts user input.

use mytools_core::color::{parse as core_parse, parse_hex, parse_rgb, Rgb};

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorView {
    pub hex: String,
    pub rgb: String,
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl From<Rgb> for ColorView {
    fn from(c: Rgb) -> Self {
        ColorView {
            hex: c.to_hex(),
            rgb: c.to_rgb_string(),
            r: c.r,
            g: c.g,
            b: c.b,
        }
    }
}

#[tauri::command]
pub async fn color_parse(input: String) -> Result<ColorView, String> {
    core_parse(&input)
        .map(ColorView::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn color_parse_hex(input: String) -> Result<ColorView, String> {
    parse_hex(&input)
        .map(ColorView::from)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn color_parse_rgb(input: String) -> Result<ColorView, String> {
    parse_rgb(&input)
        .map(ColorView::from)
        .map_err(|e| e.to_string())
}
