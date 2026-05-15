// Instanced quad rendering shader.
// Each instance is a colored rectangle defined in pixel coordinates.
// The vertex shader converts pixel coordinates to clip space using a viewport uniform.

struct QuadInstance {
    @location(0) position: vec2<f32>,  // top-left corner in pixels
    @location(1) size: vec2<f32>,      // width, height in pixels
    @location(2) color: vec4<f32>,     // RGBA color
    @location(3) corner_radius: f32,   // rounded corner radius
};

struct Uniforms {
    viewport_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local_pos: vec2<f32>,       // position within quad (0..size)
    @location(2) size: vec2<f32>,
    @location(3) corner_radius: f32,
};

// Unit quad vertices: 0,0 -> 1,1
var<private> QUAD_VERTICES: array<vec2<f32>, 6> = array(
    vec2(0.0, 0.0), vec2(1.0, 0.0), vec2(0.0, 1.0),
    vec2(1.0, 0.0), vec2(1.0, 1.0), vec2(0.0, 1.0),
);

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    instance: QuadInstance,
) -> VertexOutput {
    let vertex = QUAD_VERTICES[vertex_index];
    let pixel_pos = instance.position + vertex * instance.size;

    // Convert pixel coordinates to clip space (-1..1)
    let clip_pos = vec2(
        (pixel_pos.x / uniforms.viewport_size.x) * 2.0 - 1.0,
        1.0 - (pixel_pos.y / uniforms.viewport_size.y) * 2.0,
    );

    var out: VertexOutput;
    out.clip_position = vec4(clip_pos, 0.0, 1.0);
    out.color = instance.color;
    out.local_pos = vertex * instance.size;
    out.size = instance.size;
    out.corner_radius = instance.corner_radius;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Simple solid fill (rounded corners can be added later)
    return in.color;
}
