use glyphon::{
    Attrs, Buffer, Cache, Color as GlyphonColor, Family, FontSystem, Metrics, Resolution, Shaping,
    SwashCache, TextArea, TextAtlas, TextBounds, TextRenderer, Viewport,
};

/// A label to render with the text engine.
pub struct TextLabel {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub font_size: f32,
    pub color: GlyphonColor,
}

/// Metadata for a terminal text area, used to build TextArea references
/// from pre-built Buffers during the prepare step.
pub struct TerminalTextAreaMeta {
    pub left: f32,
    pub top: f32,
    pub bounds_left: i32,
    pub bounds_top: i32,
    pub bounds_right: i32,
    pub bounds_bottom: i32,
    pub default_color: GlyphonColor,
}

/// GPU text rendering engine wrapping glyphon.
///
/// Renders text labels into the wgpu render pass using cosmic-text for shaping
/// and glyphon for GPU atlas management.
pub struct TextEngine {
    font_system: FontSystem,
    swash_cache: SwashCache,
    /// Cache must be kept alive -- it owns shared pipelines and bind group layouts
    /// used by TextAtlas and Viewport.
    #[allow(dead_code)]
    cache: Cache,
    atlas: TextAtlas,
    text_renderer: TextRenderer,
    viewport: Viewport,
    /// Buffers must outlive the TextArea references during prepare/render.
    /// Cleared and rebuilt each frame.
    buffers: Vec<Buffer>,
    /// Pre-built terminal row buffers for per-cell true-color rendering (TERM-02).
    terminal_buffers: Vec<Buffer>,
    /// Metadata for each terminal text area.
    terminal_areas_meta: Vec<TerminalTextAreaMeta>,
}

impl TextEngine {
    /// Create a new TextEngine with system fonts loaded.
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(device);
        let mut atlas = TextAtlas::new(device, queue, &cache, format);
        let text_renderer = TextRenderer::new(
            &mut atlas,
            device,
            wgpu::MultisampleState::default(),
            None,
        );
        let viewport = Viewport::new(device, &cache);

        Self {
            font_system,
            swash_cache,
            cache,
            atlas,
            text_renderer,
            viewport,
            buffers: Vec::new(),
            terminal_buffers: Vec::new(),
            terminal_areas_meta: Vec::new(),
        }
    }

    /// Access the font system for terminal text rendering.
    pub fn font_system_mut(&mut self) -> &mut FontSystem {
        &mut self.font_system
    }

    /// Load a font from raw bytes into the font system.
    pub fn load_font_data(&mut self, data: Vec<u8>) {
        self.font_system.db_mut().load_font_data(data);
    }

    /// Set pre-built terminal row buffers for inclusion in the next prepare() call.
    ///
    /// This enables per-cell true-color rendering (TERM-02) via rich text Buffers.
    pub fn set_terminal_buffers(
        &mut self,
        buffers: Vec<Buffer>,
        areas_meta: Vec<TerminalTextAreaMeta>,
    ) {
        self.terminal_buffers = buffers;
        self.terminal_areas_meta = areas_meta;
    }

    /// Prepare text labels for rendering.
    ///
    /// Creates glyphon Buffers for each label, shapes text, and uploads to the GPU atlas.
    /// The buffers are stored on self to keep them alive for the render pass.
    /// Also includes any pre-built terminal buffers set via set_terminal_buffers().
    pub fn prepare(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        labels: &[TextLabel],
        width: u32,
        height: u32,
    ) {
        self.viewport
            .update(queue, Resolution { width, height });

        // Clear previous frame's buffers and rebuild
        self.buffers.clear();

        // Create buffers for each label
        for label in labels {
            let mut buffer = Buffer::new(
                &mut self.font_system,
                Metrics::new(label.font_size, label.font_size * 1.3),
            );
            buffer.set_size(&mut self.font_system, Some(label.width), Some(label.height));
            buffer.set_text(
                &mut self.font_system,
                &label.text,
                &Attrs::new().family(Family::SansSerif),
                Shaping::Advanced,
                None,
            );
            buffer.shape_until_scroll(&mut self.font_system, false);
            self.buffers.push(buffer);
        }

        // Build TextAreas referencing the stored buffers
        let mut text_areas: Vec<TextArea> = self
            .buffers
            .iter()
            .zip(labels.iter())
            .map(|(buffer, label)| TextArea {
                buffer,
                left: label.x,
                top: label.y,
                scale: 1.0,
                bounds: TextBounds {
                    left: label.x as i32,
                    top: label.y as i32,
                    right: (label.x + label.width) as i32,
                    bottom: (label.y + label.height) as i32,
                },
                default_color: label.color,
                custom_glyphs: &[],
            })
            .collect();

        // Add terminal text areas from pre-built buffers
        for (buf, meta) in self
            .terminal_buffers
            .iter()
            .zip(self.terminal_areas_meta.iter())
        {
            text_areas.push(TextArea {
                buffer: buf,
                left: meta.left,
                top: meta.top,
                scale: 1.0,
                bounds: TextBounds {
                    left: meta.bounds_left,
                    top: meta.bounds_top,
                    right: meta.bounds_right,
                    bottom: meta.bounds_bottom,
                },
                default_color: meta.default_color,
                custom_glyphs: &[],
            });
        }

        self.text_renderer
            .prepare(
                device,
                queue,
                &mut self.font_system,
                &mut self.atlas,
                &self.viewport,
                text_areas,
                &mut self.swash_cache,
            )
            .unwrap();
    }

    /// Render prepared text in the given render pass.
    ///
    /// Must be called after `prepare()` within the same frame.
    /// Text renders ON TOP of quads when called after quad_renderer.render().
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        self.text_renderer
            .render(&self.atlas, &self.viewport, pass)
            .unwrap();
    }
}
