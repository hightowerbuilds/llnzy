// Rain on a window: frosted-glass backdrop + procedural lensing droplets.
// Technique: 2-octave value-noise FBM backdrop sampled twice -- once at the
// fragment, once with a radial offset inside each drop to fake refraction
// (no scene texture available; the FBM is the "what's behind the glass").
// Drops live in a hashed grid: per-cell hash decides offset, radius, speed,
// and phase so each drop is decorrelated. Drops trickle downward and wrap.
// References:
//   Shadertoy ldfyzl ("Heartfelt", Martijn Steinrucken) -- drop geometry.
//   Shadertoy 4sBfWh ("Rain drops in glass", chronos)   -- cell scheme.
//   Inigo Quilez articles/distfunctions2d                -- SDF disc.
//   Inigo Quilez articles/morenoise (lsf3WH)             -- value noise.

struct Uniforms {
    resolution: vec2<f32>,
    time:       f32,
    intensity:  f32,
    color1:     vec4<f32>,  // darkest -- "outside" depth color
    color2:     vec4<f32>,  // mid -- frosted glass tint
    color3:     vec4<f32>,  // brightest -- drop rim highlight
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

// --- Hash & value noise (Quilez lsf3WH) ---------------------------------
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

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 2.0 - 1.0;
}

const M2: mat2x2<f32> = mat2x2<f32>(0.80, 0.60, -0.60, 0.80);
fn fbm2(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var a = 0.5;
    var s = 0.0;
    for (var i: i32 = 0; i < 2; i = i + 1) {
        s = s + a * vnoise(p);
        p = M2 * p * 2.0;
        a = a * 0.5;
    }
    return s;
}

fn palette(t_in: f32) -> vec3<f32> {
    let t = clamp(t_in, 0.0, 1.0);
    return mix(uni.color1.rgb, uni.color2.rgb, smoothstep(0.0, 1.0, t));
}

fn backdrop(pix: vec2<f32>) -> vec3<f32> {
    let p = pix / 320.0;
    var v = 0.5 + 0.5 * fbm2(p + vec2<f32>(0.0, uni.time * 0.015));
    v = pow(v, 1.5);
    v = mix(0.05, 0.65, v);
    return palette(v);
}

@fragment
fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    let pix = coord.xy;

    var rgb = backdrop(pix);

    let CELL = 96.0;
    let cell_uv = pix / CELL;
    let cell_id = floor(cell_uv);
    let cell_f  = fract(cell_uv);

    let h1 = hash22(cell_id);
    let h2 = hash22(cell_id + vec2<f32>(5.7, 9.3));

    let cx = mix(0.30, 0.70, h1.x);
    let r  = mix(0.14, 0.22, h1.y);
    let speed = mix(0.05, 0.16, h2.x);
    let phase = h2.y;
    let wob   = 0.85 + 0.30 * sin(uni.time * 0.7 + phase * 31.4);
    let yflow = phase + uni.time * speed * wob;
    let cy    = fract(yflow);

    let drop_center = vec2<f32>(cx, cy);
    var d = cell_f - drop_center;
    d.y = d.y * 1.15;
    let dist = length(d);
    let nr   = dist / r;

    let mask = 1.0 - smoothstep(0.70, 1.00, nr);

    if (mask > 0.001) {
        let dir   = select(vec2<f32>(0.0, 0.0), d / max(dist, 1e-5),
                           dist > 1e-5);
        let bend  = smoothstep(0.0, 1.0, nr) * (r * CELL * 0.45);
        let sample_px = pix - dir * bend;
        let lensed    = backdrop(sample_px);

        let drop_body = lensed * 1.35 + uni.color2.rgb * 0.10;

        let rim = smoothstep(0.82, 1.00, nr) * (1.0 - smoothstep(1.00, 1.06, nr));
        let rim_col = uni.color3.rgb * rim * 1.4;

        let hi_offset = (cell_f - (drop_center + vec2<f32>(-0.35, -0.35) * r)) / r;
        let hi = exp(-dot(hi_offset, hi_offset) * 14.0) * 0.55;
        let hi_col = uni.color3.rgb * hi;

        let drop_rgb = drop_body + rim_col + hi_col;
        rgb = mix(rgb, drop_rgb, mask);
    }

    rgb = rgb * uni.intensity;
    return vec4<f32>(rgb, 1.0);
}
