//! Color utilities — parse and format HEX / RGB / HSL strings.
//!
//! v0.1 keeps these on the Rust side (pure logic, easy to verify);
//! v0.2 adds a screen-sampling command that uses a native dependency.

use serde::Serialize;

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl Rgb {
    pub fn to_hex(self) -> String {
        format!("#{:02X}{:02X}{:02X}", self.r, self.g, self.b)
    }

    pub fn to_rgb_string(self) -> String {
        format!("rgb({}, {}, {})", self.r, self.g, self.b)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ColorError {
    #[error("invalid hex string: {0}")]
    BadHex(String),
    #[error("invalid rgb string: {0}")]
    BadRgb(String),
}

/// Parse a `#RRGGBB` or `RRGGBB` or `#RGB` short string.
pub fn parse_hex(input: &str) -> Result<Rgb, ColorError> {
    let trimmed = input.trim().trim_start_matches('#');
    let expanded = match trimmed.len() {
        3 => {
            let mut s = String::with_capacity(6);
            for c in trimmed.chars() {
                s.push(c);
                s.push(c);
            }
            s
        }
        6 => trimmed.to_string(),
        _ => return Err(ColorError::BadHex(input.to_string())),
    };
    let r =
        u8::from_str_radix(&expanded[0..2], 16).map_err(|_| ColorError::BadHex(input.into()))?;
    let g =
        u8::from_str_radix(&expanded[2..4], 16).map_err(|_| ColorError::BadHex(input.into()))?;
    let b =
        u8::from_str_radix(&expanded[4..6], 16).map_err(|_| ColorError::BadHex(input.into()))?;
    Ok(Rgb { r, g, b })
}

pub fn parse_rgb(input: &str) -> Result<Rgb, ColorError> {
    let trimmed = input.trim();
    let body = trimmed
        .strip_prefix("rgb(")
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| ColorError::BadRgb(input.to_string()))?;
    let parts: Vec<&str> = body.split(',').map(str::trim).collect();
    if parts.len() != 3 {
        return Err(ColorError::BadRgb(input.to_string()));
    }
    let r = parts[0]
        .parse::<u8>()
        .map_err(|_| ColorError::BadRgb(input.to_string()))?;
    let g = parts[1]
        .parse::<u8>()
        .map_err(|_| ColorError::BadRgb(input.to_string()))?;
    let b = parts[2]
        .parse::<u8>()
        .map_err(|_| ColorError::BadRgb(input.to_string()))?;
    Ok(Rgb { r, g, b })
}

/// Auto-detect format and parse.
pub fn parse(input: &str) -> Result<Rgb, ColorError> {
    let trimmed = input.trim();
    if (trimmed.starts_with('#') || trimmed.chars().all(|c| c.is_ascii_hexdigit()))
        && trimmed.chars().filter(|c| c.is_ascii_hexdigit()).count() == 6
    {
        return parse_hex(trimmed);
    }
    if trimmed.starts_with("rgb(") {
        return parse_rgb(trimmed);
    }
    Err(ColorError::BadHex(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_six_digit_hex() {
        let c = parse_hex("#FF8800").unwrap();
        assert_eq!(c.r, 0xFF);
        assert_eq!(c.g, 0x88);
        assert_eq!(c.b, 0x00);
    }

    #[test]
    fn parses_three_digit_hex_short_form() {
        let c = parse_hex("#f80").unwrap();
        assert_eq!(c.to_hex(), "#FF8800");
    }

    #[test]
    fn parses_hex_without_hash() {
        assert_eq!(parse_hex("aabbcc").unwrap().to_hex(), "#AABBCC");
    }

    #[test]
    fn rejects_bad_hex() {
        assert!(parse_hex("xyz").is_err());
        assert!(parse_hex("#12345").is_err());
    }

    #[test]
    fn parses_rgb_string() {
        let c = parse_rgb("rgb(10, 20, 30)").unwrap();
        assert_eq!(c.r, 10);
        assert_eq!(c.g, 20);
        assert_eq!(c.b, 30);
    }

    #[test]
    fn rejects_bad_rgb() {
        assert!(parse_rgb("rgb(1,2)").is_err());
        assert!(parse_rgb("not-rgb").is_err());
    }

    #[test]
    fn auto_detects_hex_and_rgb() {
        assert_eq!(parse("#ABCDEF").unwrap().to_hex(), "#ABCDEF");
        assert_eq!(parse("rgb(1, 2, 3)").unwrap().to_hex(), "#010203");
    }

    #[test]
    fn to_hex_is_uppercase() {
        assert_eq!(Rgb { r: 1, g: 2, b: 3 }.to_hex(), "#010203");
    }
}
