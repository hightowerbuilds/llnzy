// Procedural fire via upward-advected FBM with vertical falloff mask.
// Technique blend:
//   - Value noise + quintic Hermite (Quilez "morenoise", lsf3WH)
//   - 4-octave FBM with decorrelation rotation (Quilez "fbm")
//   - Upward y-scroll for advection, mild horizontal sway for wandering tongues
//     (cf. Shadertoy lsf3RH "Fire" by sschmiderer, Mtf3D7 "Fire" by 834144373)
//   - Vertical pow-falloff mask so top of frame stays near-black for text legibility
//   - 3-stop palette with peakier hot core via pow() before the upper smoothstep
// Uniform layout, vertex stage, and entry signatures match smoke.wgsl exactly.

struct Uniforms {
    resolution: vec2<f32>,
    time:       f32,
    intensity:  f32,
    color1:     vec4<f32>,  // darkest (ember black / deep red)
    color2:     vec4<f32>,  // mid     (orange / red body)
    color3:     vec4<f32>,  // hottest (yellow / white core)
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

// --- Hash & value noise (Quilez lsf3WH) ----------------------------------
fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    let r = q + dot(q, q + 78.233);
    return fract(r.x * r.y);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    // Quintic Hermite -- C2 continuous, kills derivative shimmer
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

// --- 3-stop palette with peakier hot core --------------------------------
fn palette(t_in: f32) -> vec3<f32> {
    let t = clamp(t_in, 0.0, 1.0);
    let lo = mix(uni.color1.rgb, uni.color2.rgb, smoothstep(0.0, 0.50, t));
    // Sharpen the upper transition so the bright core only appears at the
    // tips of flame tongues -- fire's "hot core" concept (unlike smoke).
    let hot = smoothstep(0.62, 0.92, t);
    return mix(lo, uni.color3.rgb, pow(hot, 1.4));
}

@fragment
fn fs_main(@builtin(position) coord: vec4<f32>) -> @location(0) vec4<f32> {
    // Anisotropic scaling: stretch vertically so flames are taller than wide.
    // SCALE_X = 220 px/unit -> octave-3 wavelength ~27 px (>> 4 px bandlimit).
    // SCALE_Y = 340 px/unit -> octave-3 wavelength ~42 px vertically.
    let SCALE = vec2<f32>(220.0, 340.0);
    var p = coord.xy / SCALE;

    // Upward advection: scroll the noise field DOWN over time so when we sample
    // it the features appear to rise. Period ~6.4 s for the dominant octave.
    let t = uni.time;
    p.y = p.y + t * 0.55;

    // Lateral wander: low-frequency lateral offset that drifts with time,
    // gives flames a "lick sideways" feel instead of pure vertical rise.
    let sway = 0.18 * vnoise(vec2<f32>(p.x * 0.6, t * 0.25));
    p.x = p.x + sway;

    // FBM body
    var n = fbm(p);
    n = 0.5 + 0.5 * n;          // -> [0,1]

    // Vertical falloff mask: v01 = 0 at top, 1 at bottom (coord.y grows down).
    let v01 = clamp(coord.y / uni.resolution.y, 0.0, 1.0);
    // bottomness: 1 at the bottom edge, 0 at the top
    let bottomness = v01;
    // Pow curve: k=2.2 concentrates fire in the lower ~40% of the frame.
    // Add a soft floor at the very bottom so the base is solid embers.
    let mask = smoothstep(0.0, 0.10, bottomness) * pow(bottomness, 2.2);

    // Carve the flame tongues. Subtracting (1 - mask) eats the noise away
    // toward the top; what survives is fire-shaped, not a wash.
    var fire = n * 1.25 - (1.0 - mask) * 0.95;
    fire = clamp(fire, 0.0, 1.0);

    // Peakier core: only the brightest noise lobes reach color3.
    fire = pow(fire, 1.15);

    var rgb = palette(fire);

    // Intensity scales overall brightness; keeps text readable at 0.4 default.
    rgb = rgb * uni.intensity;

    return vec4<f32>(rgb, 1.0);
}
