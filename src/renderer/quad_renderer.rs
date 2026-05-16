use tracing::debug;

/// A single colored rectangle instance for GPU instanced rendering.
///
/// Layout matches the WGSL shader's QuadInstance struct.
/// Uses 16-byte alignment via padding field for GPU buffer compatibility.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct QuadInstance {
    /// Top-left corner position in pixels.
    pub position: [f32; 2],
    /// Width and height in pixels.
    pub size: [f32; 2],
    /// RGBA color (0.0..=1.0).
    pub color: [f32; 4],
    /// Rounded corner radius in pixels.
    pub corner_radius: f32,
    /// Padding for 16-byte alignment.
    pub _padding: f32,
}

/// Viewport uniform data sent to the GPU.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    viewport_size: [f32; 2],
}

/// Maximum number of quad instances per frame (T-01-03 mitigation).
const MAX_INSTANCES: usize = 1000;

/// Instanced colored rectangle renderer.
///
/// Uses a WGSL shader with instanced drawing: 6 vertices (unit quad)
/// drawn N times with per-instance position, size, color, and corner_radius.
pub struct QuadRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    uniform_bind_group: wgpu::BindGroup,
    instance_buffer: wgpu::Buffer,
    instance_count: u32,
}

impl QuadRenderer {
    /// Create a new QuadRenderer with the given device and surface format.
    pub fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        // Load shader
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Quad Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/quad.wgsl").into()),
        });

        // Uniform buffer for viewport size
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Uniforms"),
            size: std::mem::size_of::<Uniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Bind group layout and bind group for uniforms
        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Quad Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Quad Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        // Pipeline layout
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Quad Pipeline Layout"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        // Instance buffer layout: matches QuadInstance struct fields
        let instance_buffer_layout = wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<QuadInstance>() as u64,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                // @location(0) position: vec2<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 0,
                    shader_location: 0,
                },
                // @location(1) size: vec2<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x2,
                    offset: 8,
                    shader_location: 1,
                },
                // @location(2) color: vec4<f32>
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32x4,
                    offset: 16,
                    shader_location: 2,
                },
                // @location(3) corner_radius: f32
                wgpu::VertexAttribute {
                    format: wgpu::VertexFormat::Float32,
                    offset: 32,
                    shader_location: 3,
                },
            ],
        };

        // Render pipeline
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Quad Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[instance_buffer_layout],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview_mask: None,
            cache: None,
        });

        // Initial empty instance buffer
        let instance_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Quad Instance Buffer"),
            size: (std::mem::size_of::<QuadInstance>() * MAX_INSTANCES) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            uniform_buffer,
            uniform_bind_group,
            instance_buffer,
            instance_count: 0,
        }
    }

    /// Prepare quad data for rendering.
    ///
    /// Uploads viewport uniforms and instance data to the GPU.
    /// Caps instance count to MAX_INSTANCES (T-01-03 threat mitigation).
    pub fn prepare(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        quads: &[QuadInstance],
        viewport_width: f32,
        viewport_height: f32,
    ) {
        // Write viewport uniforms
        let uniforms = Uniforms {
            viewport_size: [viewport_width, viewport_height],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        // Cap instance count to prevent GPU buffer exhaustion (T-01-03)
        let count = quads.len().min(MAX_INSTANCES);
        if quads.len() > MAX_INSTANCES {
            debug!(
                requested = quads.len(),
                max = MAX_INSTANCES,
                "Quad instance count capped"
            );
        }

        // Write instance data
        if count > 0 {
            queue.write_buffer(
                &self.instance_buffer,
                0,
                bytemuck::cast_slice(&quads[..count]),
            );
        }
        self.instance_count = count as u32;
    }

    /// Render all prepared quads in the given render pass.
    ///
    /// Must be called after `prepare()` within the same frame.
    /// Uses instanced drawing: 6 vertices (unit quad) x N instances.
    pub fn render<'pass>(&'pass self, pass: &mut wgpu::RenderPass<'pass>) {
        if self.instance_count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        pass.set_vertex_buffer(0, self.instance_buffer.slice(..));
        pass.draw(0..6, 0..self.instance_count);
    }
}
