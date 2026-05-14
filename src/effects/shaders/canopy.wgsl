// Windy canopy: forest seen from beneath, leaves bobbing on independent phases.
// Technique blend:
//   - Per-fragment grid hashing (one leaf-cell per layer; no neighbor iteration)
//   - Per-cell phase decorrelation via hash22 -> sin/cos bob with global wind drift
//   - Rotated ellipse SDF for leaf shape (Quilez "distfunctions2d",
//     https://iquilezles.org/articles/distfunctions2d/) -- simple norm/aspect form,
//     not the exact iterative ellipse (we only need a smooth coverage falloff)
//   - 3 back-to-front depth layers (size up, speed up, brightness up)
//   - 3-stop palette: color1 = canopy shadow, color2 = leaf body, color3 = sunfleck
// References: Shadertoy XllGRH (foliage), MtcGRl (procedural foliage with wind),
//   smoke.wgsl / fire.wgsl in this repo for hash + palette conventions.
// Uniform layout, vertex stage and fragment signature match the other shaders.

struct Uniforms {
    resolution: vec2<f32>,
    time:       f32,
    intensity:  f32,
    color1:     vec4<f32>,  // deep canopy shadow / background
    color2:     vec4<f32>,  // principal leaf green
    color3:     vec4<f32>,  // sun through the leaves (highlight)
};
@group(0) @binding(0) var<uniform> uni: Uniforms;

@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

// --- Hashes (Quilez lsf3WH style) ---------------------------------------
fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 78.233);
    return fract(r.x * r.y);
}

fn hash22(p: vec2<f32>) -> vec2<f32> {
    let a = hash21(p);
    let b = hash21(p + vec2<f32>(37.19, 11.71));
    return vec2<f32>(a, b);
}

// Soft-edged rotated ellipse coverage. Returns 1 at center, 0 outside.
fn leaf_coverage(d: vec2<f32>, angle: f32, aspect: f32, radius: f32) -> f32 {
    let c = cos(angle);
    let s = sin(angle);
    let dl = vec2<f32>( c * d.x + s * d.y,
                       -s * d.x + c * d.y);
    let q = vec2<f32>(dl.x / aspect, dl.y);
    let r = length(q);
    return 1.0 - smoothstep(radius * 0.55, radius, r);
}

// One canopy layer. Returns (coverage, leaf_hash) so the caller can gate the
// sunfleck highlight on the per-leaf hash.
fn canopy_layer(pix: vec2<f32>, cell_size: f32, layer_seed: vec2<f32>,
                wind: vec2<f32>, bob_amp: f32, t: f32) -> vec2<f32> {
    let p = pix / cell_size;
    let cell = floor(p) + layer_seed;
    let f    = fract(p);

    let h1 = hash22(cell);
    let h2 = hash22(cell + vec2<f32>(5.7, 9.3));

    let center = vec2<f32>(0.5) + (h1 - 0.5) * 0.55;

    let phase = h2.x * 6.2831853;
    let freq  = 1.6 + h2.y * 0.9;

    let bob = vec2<f32>(sin(t * freq + phase),
                        cos(t * freq * 0.7 + phase * 1.3)) * bob_amp;
    let global = wind * (0.6 + 0.4 * sin(t * 0.25));

    let d = f - (center + bob + global);

    let angle  = h1.y * 6.2831853;
    let aspect = 1.5 + h2.x * 0.9;
    let radius = 0.38 + h2.y * 0.12;

    let cov = leaf_coverage(d, angle, aspect, radius);
    let leaf_t = h1.x;
    return vec2<f32>(cov, leaf_t);
}

@fragment
fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    let pix = coord.xy;
    let t   = uni.time;

    var rgb = uni.color1.rgb;

    // Layer 0 (back): small, dim, slow.
    {
        let layer = canopy_layer(pix, 18.0, vec2<f32>( 0.0, 0.0),
                                  vec2<f32>(0.020, 0.010), 0.018, t);
        let cov   = layer.x;
        let lt    = layer.y;
        var lcol  = mix(uni.color1.rgb, uni.color2.rgb, 0.55);
        let hl    = smoothstep(0.88, 0.99, lt) * cov;
        lcol      = mix(lcol, uni.color3.rgb, hl * 0.35);
        rgb       = mix(rgb, lcol, cov * 0.75);
    }

    // Layer 1 (mid).
    {
        let layer = canopy_layer(pix, 30.0, vec2<f32>(17.0, 4.0),
                                  vec2<f32>(0.030, 0.015), 0.030, t);
        let cov   = layer.x;
        let lt    = layer.y;
        var lcol  = mix(uni.color1.rgb, uni.color2.rgb, 0.82);
        let hl    = smoothstep(0.78, 0.97, lt) * cov;
        lcol      = mix(lcol, uni.color3.rgb, hl * 0.55);
        rgb       = mix(rgb, lcol, cov * 0.85);
    }

    // Layer 2 (front): large, bright, fastest.
    {
        let layer = canopy_layer(pix, 52.0, vec2<f32>(3.0, 29.0),
                                  vec2<f32>(0.045, 0.022), 0.045, t);
        let cov   = layer.x;
        let lt    = layer.y;
        var lcol  = mix(uni.color2.rgb, uni.color3.rgb,
                        smoothstep(0.70, 1.0, lt) * 0.55);
        let hl    = smoothstep(0.85, 1.00, lt) * cov;
        lcol      = mix(lcol, uni.color3.rgb, hl * 0.6);
        rgb       = mix(rgb, lcol, cov);
    }

    rgb = rgb * uni.intensity;

    return vec4<f32>(rgb, 1.0);
}
