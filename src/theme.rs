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
    /// Dark theme -- the default.
    pub fn dark() -> Self {
        Self {
            background: [0.1, 0.1, 0.12, 1.0],
            panel_background: [0.14, 0.14, 0.16, 1.0],
            title_bar_text: [0.78, 0.78, 0.80, 1.0],
            divider: [0.2, 0.2, 0.22, 1.0],
            divider_hover: [0.4, 0.4, 0.45, 1.0],
            panel_label_text: [0.5, 0.5, 0.52, 1.0],
            markdown_body_text: [0.86, 0.84, 0.81, 1.0],
            markdown_heading_text: [0.93, 0.91, 0.88, 1.0],
            markdown_code_text: [0.78, 0.84, 0.78, 1.0],
            markdown_code_block_bg: [0.11, 0.11, 0.14, 1.0],
            markdown_blockquote_border: [0.30, 0.30, 0.35, 1.0],
            markdown_link_text: [0.45, 0.60, 0.85, 1.0],
            markdown_hr: [0.25, 0.25, 0.28, 1.0],
            sidebar_selected_bg: [0.18, 0.18, 0.22, 1.0],
            sidebar_hover_bg: [0.16, 0.16, 0.19, 1.0],
            sidebar_folder_text: [0.60, 0.60, 0.62, 1.0],
            unfocused_overlay: [0.0, 0.0, 0.0, 0.25],
        }
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}
