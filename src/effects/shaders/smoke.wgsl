// Atmospheric smoke via domain-warped fractional Brownian motion (FBM).
// Technique: Inigo Quilez "domain warping" (https://iquilezles.org/articles/warp/)
// with value noise (https://iquilezles.org/articles/morenoise/, ShaderToy lsf3WH).
// Pipeline: warp = fbm(p + t*flow); base = fbm(p + W*warp); color = palette(base).

struct Uniforms {
    resolution: vec2<f32>,  // target texture size in pixels
    time:       f32,        // seconds since shader load
    intensity:  f32,        // 0..1 user slider, scales final brightness
    color1:     vec4<f32>,  // palette stop 0 (darkest), RGB used
    color2:     vec4<f32>,  // palette stop 1 (mid wisp)
    color3:     vec4<f32>,  // palette stop 2 (brightest wisp highlight)
};
@group(0) @binding(0) var<uniform> uni: Uniforms;

// Fullscreen-triangle vertex stage. Three vertices, no vertex buffer; the
// triangle covers the entire clip-space square so `@builtin(position)` in
// the fragment stage gives us a pixel coordinate over the whole target.
@vertex
fn vs_main(@builtin(vertex_index) vid: u32) -> @builtin(position) vec4<f32> {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0),
        vec2<f32>(-1.0,  3.0),
    );
    return vec4<f32>(positions[vid], 0.0, 1.0);
}

// --- Hash & value noise (Quilez lsf3WH) ----------------------------------
fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 78.233);
    return fract(r.x * r.y);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // quintic smoothstep (Perlin 2002) — C2 continuous, avoids derivative seams
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 2.0 - 1.0; // [-1, 1]
}

// --- 4-octave FBM with decorrelation rotation ----------------------------
const M2: mat2x2<f32> = mat2x2<f32>(0.80, 0.60, -0.60, 0.80);

fn fbm(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var a = 0.5;
    var sum = 0.0;
    for (var i: i32 = 0; i < 4; i = i + 1) {
        sum = sum + a * vnoise(p);
        p = M2 * p * 2.0;   // lacunarity 2.0
        a = a * 0.5;        // gain 0.5
    }
    return sum;             // ~[-1, 1]
}

// --- 3-stop palette (piecewise linear in sRGB) ---------------------------
fn palette(t_in: f32) -> vec3<f32> {
    let t = clamp(t_in, 0.0, 1.0);
    let lo = mix(uni.color1.rgb, uni.color2.rgb, smoothstep(0.0, 0.55, t));
    return mix(lo, uni.color3.rgb, smoothstep(0.55, 1.0, t));
}

@fragment
fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Normalize so 1 unit == ~256 px; smallest octave (octave 3, freq 8x)
    // therefore has a feature wavelength of 256/8 = 32 px >> 4 px bandlimit.
    let SCALE = 256.0;
    let p = coord.xy / SCALE;

    // Slow atmospheric time — full visual cycle ~20 s.
    let t = uni.time * 0.06;

    // Inner flow drives the warp itself (Quilez "flow noise" trick).
    let q = vec2<f32>(
        fbm(p + vec2<f32>(0.0, 0.0) + vec2<f32>(t,        0.5 * t)),
        fbm(p + vec2<f32>(5.2, 1.3) + vec2<f32>(-0.7 * t, 0.3 * t))
    );

    // Single domain-warp pass; W controls swirl strength.
    let W = 1.7;
    let n = fbm(p + W * q);

    // Remap [-1,1] -> [0,1] with a soft bias toward dark so most of the
    // frame sits at low luminance for readable text overlay.
    var v = 0.5 + 0.5 * n;
    v = pow(v, 1.6);                 // gamma-shape: darken mids
    v = mix(0.04, 0.95, v);          // floor + ceiling clamp

    var rgb = palette(v);

    // Intensity scales overall brightness; 0.4 default keeps it muted.
    rgb = rgb * uni.intensity;

    return vec4<f32>(rgb, 1.0);
}
