/// Color palette for themed rendering.
///
/// All colors are RGBA as `[f32; 4]` with values in 0.0..=1.0.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Main window background
    pub background: [f32; 4],
    /// Panel body background (themed, not distinct colors per D-03)
    pub panel_background: [f32; 4],
    /// Title bar label color
    pub title_bar_text: [f32; 4],
    /// Divider line color (1px normally per D-04)
    pub divider: [f32; 4],
    /// Divider hover highlight
    pub divider_hover: [f32; 4],
    /// Centered type label in panel body
    pub panel_label_text: [f32; 4],
    // Markdown colors
    pub markdown_body_text: [f32; 4],
    pub markdown_heading_text: [f32; 4],
    pub markdown_code_text: [f32; 4],
    pub markdown_code_block_bg: [f32; 4],
    pub markdown_blockquote_border: [f32; 4],
    pub markdown_link_text: [f32; 4],
    pub markdown_hr: [f32; 4],
    // Sidebar colors
    pub sidebar_selected_bg: [f32; 4],
    pub sidebar_hover_bg: [f32; 4],
    pub sidebar_folder_text: [f32; 4],
    // Focus
    pub unfocused_overlay: [f32; 4],
}

impl Theme {
    /// Dracula theme (Warp variant). Values are linear-light for sRGB surface.
    pub fn dark() -> Self {
        Self {
            background: [0.0212, 0.0232, 0.0369, 1.0],              // #282a36
            panel_background: [0.0252, 0.0273, 0.0437, 1.0],        // #2c2e3b
            title_bar_text: [0.9387, 0.9387, 0.8879, 1.0],          // #f8f8f2
            divider: [0.0356, 0.0382, 0.0648, 1.0],                 // #353748
            divider_hover: [0.5089, 0.2918, 0.9473, 1.0],           // #bd93f9 purple accent
            panel_label_text: [0.1221, 0.1683, 0.3712, 1.0],        // #6272a4 comment
            markdown_body_text: [0.9387, 0.9387, 0.8879, 1.0],      // #f8f8f2
            markdown_heading_text: [0.5089, 0.2918, 0.9473, 1.0],   // #bd93f9
            markdown_code_text: [0.0802, 0.9560, 0.1981, 1.0],      // #50fa7b green
            markdown_code_block_bg: [0.0152, 0.0160, 0.0252, 1.0],  // #21222c
            markdown_blockquote_border: [0.1221, 0.1683, 0.3712, 1.0], // #6272a4
            markdown_link_text: [0.2582, 0.8148, 0.9823, 1.0],      // #8be9fd cyan
            markdown_hr: [0.0578, 0.0630, 0.1022, 1.0],             // #44475a current line
            sidebar_selected_bg: [0.0578, 0.0630, 0.1022, 1.0],     // #44475a
            sidebar_hover_bg: [0.0369, 0.0395, 0.0666, 1.0],        // #363849
            sidebar_folder_text: [0.1221, 0.1683, 0.3712, 1.0],     // #6272a4
            unfocused_overlay: [0.0, 0.0, 0.0, 0.25],
        }
    }
}

/// Convert a linear-light f32 channel to sRGB u8 for glyphon text colors.
pub fn linear_to_srgb_u8(v: f32) -> u8 {
    let s = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    (s * 255.0 + 0.5).clamp(0.0, 255.0) as u8
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
