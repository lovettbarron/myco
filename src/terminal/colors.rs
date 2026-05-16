use alacritty_terminal::vte::ansi::{Color, NamedColor};

/// Independent 16-color ANSI palette for terminal rendering.
///
/// Per D-06: Terminal has its own ANSI palette, separate from the app theme.
/// Theme integration is deferred to Phase 4.
pub struct AnsiPalette {
    /// Standard 16 ANSI colors: 8 normal + 8 bright.
    pub colors: [[u8; 3]; 16],
    /// Default foreground color.
    pub foreground: [u8; 3],
    /// Default background color.
    pub background: [u8; 3],
}

impl Default for AnsiPalette {
    fn default() -> Self {
        Self {
            colors: [
                // Dracula normal colors (0-7)
                [0, 0, 0],         // Black
                [255, 85, 85],     // Red       #ff5555
                [80, 250, 123],    // Green     #50fa7b
                [241, 250, 140],   // Yellow    #f1fa8c
                [189, 147, 249],   // Blue      #bd93f9
                [255, 121, 198],   // Magenta   #ff79c6
                [139, 233, 253],   // Cyan      #8be9fd
                [187, 187, 187],   // White     #bbbbbb
                // Dracula bright colors (8-15)
                [85, 85, 85],      // Bright Black   #555555
                [255, 85, 85],     // Bright Red     #ff5555
                [80, 250, 123],    // Bright Green   #50fa7b
                [241, 250, 140],   // Bright Yellow  #f1fa8c
                [202, 169, 250],   // Bright Blue    #caa9fa
                [255, 121, 198],   // Bright Magenta #ff79c6
                [139, 233, 253],   // Bright Cyan    #8be9fd
                [255, 255, 255],   // Bright White   #ffffff
            ],
            foreground: [248, 248, 242], // #f8f8f2
            background: [40, 42, 54],    // #282a36
        }
    }
}

/// Resolve a vte::ansi::Color to an RGB triple using the terminal palette.
///
/// Handles:
/// - Color::Spec(rgb): true 24-bit color passthrough
/// - Color::Indexed(idx): standard 16 colors, 6x6x6 color cube, 24-step grayscale
/// - Color::Named(named): maps to palette index
pub fn resolve_color(color: Color, palette: &AnsiPalette) -> [u8; 3] {
    match color {
        Color::Spec(rgb) => [rgb.r, rgb.g, rgb.b],
        Color::Indexed(idx) => {
            if idx < 16 {
                palette.colors[idx as usize]
            } else if idx < 232 {
                // 216-color cube (6x6x6): indices 16..=231
                let idx = idx - 16;
                let r = (idx / 36) * 51;
                let g = ((idx / 6) % 6) * 51;
                let b = (idx % 6) * 51;
                [r, g, b]
            } else {
                // 24-step grayscale ramp: indices 232..=255
                let v = (idx - 232) * 10 + 8;
                [v, v, v]
            }
        }
        Color::Named(named) => {
            let idx = named as usize;
            if idx < 16 {
                palette.colors[idx]
            } else {
                // Fallback for special named colors beyond the 16 palette
                palette.foreground
            }
        }
    }
}

/// Resolve a foreground color, returning the default foreground for Named::Foreground.
pub fn resolve_fg(color: Color, palette: &AnsiPalette) -> [u8; 3] {
    match color {
        Color::Named(NamedColor::Foreground) => palette.foreground,
        other => resolve_color(other, palette),
    }
}

/// Resolve a background color, returning the default background for Named::Background.
pub fn resolve_bg(color: Color, palette: &AnsiPalette) -> [u8; 3] {
    match color {
        Color::Named(NamedColor::Background) => palette.background,
        other => resolve_color(other, palette),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alacritty_terminal::vte::ansi::Rgb;

    #[test]
    fn test_resolve_color_spec_true_color() {
        let palette = AnsiPalette::default();
        let rgb = resolve_color(Color::Spec(Rgb { r: 255, g: 100, b: 0 }), &palette);
        assert_eq!(rgb, [255, 100, 0]);
    }

    #[test]
    fn test_resolve_color_indexed_standard() {
        let palette = AnsiPalette::default();
        // Index 1 = Red in standard ANSI palette
        let rgb = resolve_color(Color::Indexed(1), &palette);
        assert_eq!(rgb, palette.colors[1]);
    }

    #[test]
    fn test_resolve_color_indexed_216_cube() {
        let palette = AnsiPalette::default();
        // Index 16 = first color in the 6x6x6 cube (0,0,0)
        let rgb = resolve_color(Color::Indexed(16), &palette);
        assert_eq!(rgb, [0, 0, 0]);
        // Index 196: idx=180, r=180/36=5, so r = 5*51 = 255
        let rgb = resolve_color(Color::Indexed(196), &palette);
        assert_eq!(rgb[0], 255);
    }

    #[test]
    fn test_resolve_color_indexed_grayscale() {
        let palette = AnsiPalette::default();
        // Index 232 = first grayscale (8)
        let rgb = resolve_color(Color::Indexed(232), &palette);
        assert_eq!(rgb, [8, 8, 8]);
        // Index 255 = last grayscale (238)
        let rgb = resolve_color(Color::Indexed(255), &palette);
        assert_eq!(rgb, [238, 238, 238]);
    }

    #[test]
    fn test_resolve_color_named() {
        let palette = AnsiPalette::default();
        // NamedColor::Red = index 1
        let rgb = resolve_color(Color::Named(NamedColor::Red), &palette);
        assert_eq!(rgb, palette.colors[1]);
    }

    #[test]
    fn test_resolve_fg_default() {
        let palette = AnsiPalette::default();
        let rgb = resolve_fg(Color::Named(NamedColor::Foreground), &palette);
        assert_eq!(rgb, palette.foreground);
    }

    #[test]
    fn test_resolve_bg_default() {
        let palette = AnsiPalette::default();
        let rgb = resolve_bg(Color::Named(NamedColor::Background), &palette);
        assert_eq!(rgb, palette.background);
    }
}
