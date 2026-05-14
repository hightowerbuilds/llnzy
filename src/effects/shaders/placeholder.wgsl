// M2 placeholder fragment shader: solid purple.
//
// This shader exists only to prove the wgpu render pipeline is alive
// end-to-end: a vertex stage emits a fullscreen triangle and the fragment
// stage paints every covered pixel the same colour. When the real M2 smoke
// shader lands it replaces just `fs_main` -- the vertex stage and uniform
// layout are intended to stay.

struct Uniforms {
    // x = time (seconds since EffectsHost::new), y = width, z = height, w = pad.
    time_and_resolution: vec4<f32>,
};

@group(0) @binding(0) var<uniform> uniforms: Uniforms;

struct VsOut {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

// Three vertices chosen so the triangle covers the entire clip-space square
// (-1,-1)..(1,1) without a vertex buffer. The UV maps the covered area to
// [0,1] on x and y.
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    var uvs = array<vec2<f32>, 3>(
        vec2<f32>(0.0, 1.0),
        vec2<f32>(2.0, 1.0),
        vec2<f32>(0.0, -1.0),
    );
    var out: VsOut;
    out.clip_pos = vec4<f32>(positions[vid], 0.0, 1.0);
    out.uv = uvs[vid];
    return out;
}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {
    // Solid purple proves the pipeline runs. `uniforms` is bound but unused
    // so the binding survives WGSL's unused-removal pass without warning.
    let _t = uniforms.time_and_resolution.x;
    return vec4<f32>(0.2, 0.05, 0.5, 1.0);
}
