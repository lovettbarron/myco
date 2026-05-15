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
}

impl Theme {
    /// Dark theme -- the default.
    pub fn dark() -> Self {
        Self {
            background: [0.1, 0.1, 0.12, 1.0],
            panel_background: [0.14, 0.14, 0.16, 1.0],
            title_bar_text: [0.78, 0.78, 0.80, 1.0],
            divider: [0.2, 0.2, 0.22, 1.0],
            divider_hover: [0.4, 0.4, 0.45, 1.0],
            panel_label_text: [0.5, 0.5, 0.52, 1.0],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
