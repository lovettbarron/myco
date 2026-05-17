//! Color conversion utilities for theme system.
//!
//! Provides sRGB <-> linear-light conversion and hex string parsing.

/// Convert a linear-light f32 channel to sRGB u8 for glyphon text colors.
pub fn linear_to_srgb_u8(v: f32) -> u8 {
    let s = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

/// Convert an sRGB u8 channel to linear-light f32.
pub fn srgb_to_linear(v: u8) -> f32 {
    let s = v as f32 / 255.0;
    if s <= 0.04045 {
        s / 12.92
    } else {
        ((s + 0.055) / 1.055).powf(2.4)
    }
}

/// Parse a hex color string (e.g. "#FF5555") into linear-light RGBA [f32; 4].
///
/// Returns black on invalid input (uses `unwrap_or(0)` per threat T-04-02).
pub fn hex_to_linear(hex: &str) -> [f32; 4] {
    let hex = hex.trim_start_matches('#');
    let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
    let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
    let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
    [srgb_to_linear(r), srgb_to_linear(g), srgb_to_linear(b), 1.0]
}

/// Parse a hex color string to sRGB u8 triple (e.g. "#FF5555" -> [255, 85, 85]).
///
/// Returns [0, 0, 0] on invalid input.
pub fn hex_to_srgb_u8(hex: &str) -> [u8; 3] {
    let hex = hex.trim_start_matches('#');
    [
        u8::from_str_radix(&hex[0..2], 16).unwrap_or(0),
        u8::from_str_radix(&hex[2..4], 16).unwrap_or(0),
        u8::from_str_radix(&hex[4..6], 16).unwrap_or(0),
    ]
}
