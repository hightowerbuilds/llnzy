// Procedural aurora borealis -- vertical sheets of light over a dark sky.
// Technique: per-ribbon wavy centerline + smooth distance falloff + per-ribbon
// FBM intensity envelope, blended additively, hue-graded along ribbon height.
// References (technique only, not code):
//   Shadertoy XtGGRt  "Aurora"  by nimitz
//   Shadertoy wlsXz4  "Auroras" by GLtracy
//   Inigo Quilez:  articles/fbm,  articles/gradientnoise,  articles/warp

struct Uniforms {
    resolution: vec2<f32>,
    time:       f32,
    intensity:  f32,
    color1:     vec4<f32>,  // deep night-sky / underglow
    color2:     vec4<f32>,  // main green-teal sheet
    color3:     vec4<f32>,  // magenta-violet tip
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

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Quintic Hermite (Perlin 2002) -- C2, no derivative seams, no shimmer.
    let u = f * f * f * (f * (f * 6.0 - 15.0) + 10.0);
    let a = hash21(i);
    let b = hash21(i + vec2<f32>(1.0, 0.0));
    let c = hash21(i + vec2<f32>(0.0, 1.0));
    let d = hash21(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y) * 2.0 - 1.0; // [-1,1]
}

// --- 1D-ish FBM driving centerlines & envelopes (2 octaves) -------------
fn fbm2(p_in: vec2<f32>) -> f32 {
    var p = p_in;
    var a = 0.5;
    var s = 0.0;
    for (var i: i32 = 0; i < 2; i = i + 1) {
        s = s + a * vnoise(p);
        p = p * 2.03 + vec2<f32>(11.7, 3.1);
        a = a * 0.5;
    }
    return s; // ~[-1,1]
}

// Smooth distance falloff. Lorentzian -- wider soft skirt than a gaussian.
fn falloff(d: f32, sigma: f32) -> f32 {
    let r = d / sigma;
    return 1.0 / (1.0 + r * r);
}

// One ribbon. Returns vec4(rgb_contribution, mass).
fn ribbon(
    uvp: vec2<f32>,
    base_y: f32,
    amp: f32,
    x_scale: f32,
    speed: f32,
    sigma: f32,
    seed: vec2<f32>,
) -> vec4<f32> {
    let t = uni.time;

    let wave_in = vec2<f32>(uvp.x * x_scale + speed * t, 0.37 * t) + seed;
    let centerline = base_y + amp * fbm2(wave_in);

    let dy = uvp.y - centerline;
    let core = falloff(dy, sigma);

    let env_in = vec2<f32>(uvp.x * (x_scale * 0.5) - 0.13 * t, 0.11 * t) + seed * 1.7;
    var env = 0.5 + 0.5 * fbm2(env_in);
    env = smoothstep(0.15, 0.95, env);

    let mass = core * env;

    let h = clamp(0.5 + 0.6 * (dy / sigma), 0.0, 1.0);
    let hue = mix(uni.color2.rgb, uni.color3.rgb, smoothstep(0.35, 1.0, h));

    return vec4<f32>(hue * mass, mass);
}

@fragment
fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    let res = uni.resolution;
    let uvp = vec2<f32>(coord.x / res.y, 1.0 - coord.y / res.y);

    let r1 = ribbon(uvp, 0.58, 0.10, 1.4,  0.020, 0.085,
                    vec2<f32>(0.0,   0.0));
    let r2 = ribbon(uvp, 0.66, 0.07, 1.9,  0.034, 0.055,
                    vec2<f32>(17.3,  4.1));
    let r3 = ribbon(uvp, 0.74, 0.05, 2.6,  0.013, 0.040,
                    vec2<f32>(43.7, 22.9));

    var rgb = r1.rgb + r2.rgb + r3.rgb;
    let mass = r1.a + r2.a + r3.a;

    // Horizon underglow keeps the bottom from going pure black.
    let horizon = smoothstep(0.55, 0.0, uvp.y);
    rgb = rgb + uni.color1.rgb * (0.18 * horizon + 0.04);

    // Soft tone-shape: prevent additive blowout where ribbons overlap.
    rgb = rgb / (1.0 + 0.55 * mass);

    rgb = rgb * uni.intensity;

    return vec4<f32>(rgb, 1.0);
}
