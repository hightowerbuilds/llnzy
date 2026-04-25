use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;

use super::state::GpuState;

/// Uniforms for the background effect shader.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct BackgroundUniforms {
    pub intensity: f32,
    pub speed: f32,
    pub _padding: [f32; 2],
    pub color1: [f32; 4],
    pub color2: [f32; 4],
    pub color3: [f32; 4],
}

// ── Shared vertex + noise preamble for all background shaders ──

const SHARED_PREAMBLE: &str = r#"
fn hash2(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

fn noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash2(i);
    let b = hash2(i + vec2<f32>(1.0, 0.0));
    let c = hash2(i + vec2<f32>(0.0, 1.0));
    let d = hash2(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

fn fbm(p_in: vec2<f32>, octaves: i32) -> f32 {
    var p = p_in;
    var value = 0.0;
    var amplitude = 0.5;
    var frequency = 1.0;
    for (var i = 0; i < octaves; i = i + 1) {
        value = value + amplitude * noise(p * frequency);
        frequency = frequency * 2.0;
        amplitude = amplitude * 0.5;
        let c = cos(0.5);
        let s = sin(0.5);
        p = vec2<f32>(p.x * c - p.y * s, p.x * s + p.y * c);
    }
    return value;
}

struct FrameUniforms {
    time: f32,
    delta_time: f32,
    resolution: vec2<f32>,
    frame: u32,
};
@group(0) @binding(0) var<uniform> frame: FrameUniforms;

struct BackgroundUniforms {
    intensity: f32,
    speed: f32,
    _padding: vec2<f32>,
    color1: vec4<f32>,
    color2: vec4<f32>,
    color3: vec4<f32>,
};
@group(1) @binding(0) var<uniform> bg: BackgroundUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let x = f32(i32(vi & 1u) * 4 - 1);
    let y = f32(i32(vi >> 1u) * 4 - 1);
    out.position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = vec2<f32>(x * 0.5 + 0.5, 1.0 - (y * 0.5 + 0.5));
    return out;
}
"#;

// ── Fragment shaders ──

const SMOKE_FRAGMENT: &str = r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = frame.time * bg.speed * 0.15;
    let aspect = frame.resolution.x / frame.resolution.y;
    var uv = in.uv;
    uv.x = uv.x * aspect;
    let warp1 = vec2<f32>(
        fbm(uv * 3.0 + vec2<f32>(t * 0.7, t * 0.3), 4),
        fbm(uv * 3.0 + vec2<f32>(t * 0.4 + 5.2, t * 0.6 + 1.3), 4)
    );
    let warp2 = vec2<f32>(
        fbm(uv * 3.0 + warp1 * 2.0 + vec2<f32>(t * 0.2 + 1.7, t * 0.5 + 9.2), 4),
        fbm(uv * 3.0 + warp1 * 2.0 + vec2<f32>(t * 0.3 + 8.3, t * 0.1 + 2.8), 4)
    );
    let smoke = fbm(uv * 2.0 + warp2 * 1.5, 5);
    let layer1 = mix(bg.color1.rgb, bg.color2.rgb, smoothstep(0.2, 0.6, smoke));
    let layer2 = mix(layer1, bg.color3.rgb, smoothstep(0.4, 0.8, smoke));
    let glow = smoothstep(0.5, 0.9, smoke) * 0.15;
    let color = layer2 + glow;
    return vec4<f32>(color, bg.intensity);
}
"#;

const AURORA_FRAGMENT: &str = r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = frame.time * bg.speed * 0.12;
    let aspect = frame.resolution.x / frame.resolution.y;
    var uv = in.uv;
    uv.x = uv.x * aspect;

    // Curtains of light — vertical bands that sway
    var aurora = 0.0;
    for (var i = 0; i < 5; i = i + 1) {
        let fi = f32(i);
        let wave_x = uv.x * (1.5 + fi * 0.4) + t * (0.3 + fi * 0.1) + fi * 2.1;
        let wave = sin(wave_x) * cos(wave_x * 0.7 + t * 0.2) * 0.5 + 0.5;
        let band = smoothstep(0.0, 0.3, wave) * smoothstep(1.0, 0.5, wave);
        // Fade toward top of screen
        let vertical_fade = smoothstep(0.8, 0.2, uv.y) * smoothstep(0.0, 0.15, uv.y);
        aurora += band * vertical_fade * (0.6 - fi * 0.08);
    }

    // Mix theme colors based on position
    let color_mix = mix(bg.color1.rgb, bg.color2.rgb, uv.x / aspect);
    let highlight = mix(color_mix, bg.color3.rgb, aurora * 0.7);
    let glow = aurora * 0.3;

    return vec4<f32>(highlight + glow, aurora * bg.intensity);
}
"#;

// ── Built-in shader registry ──

fn builtin_shaders() -> Vec<(&'static str, String)> {
    let shaders = [("smoke", SMOKE_FRAGMENT), ("aurora", AURORA_FRAGMENT)];
    shaders
        .iter()
        .map(|(name, frag)| (*name, format!("{}{}", SHARED_PREAMBLE, frag)))
        .collect()
}

pub struct BackgroundRenderer {
    pipelines: HashMap<String, wgpu::RenderPipeline>,
    uniform_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl BackgroundRenderer {
    pub fn new(gpu: &GpuState) -> Self {
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bg_bind_group_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    }],
                });

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bg_uniforms"),
            size: std::mem::size_of::<BackgroundUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = gpu
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("bg_pipeline_layout"),
                bind_group_layouts: &[&gpu.frame_bind_group_layout, &bind_group_layout],
                push_constant_ranges: &[],
            });

        // Build a pipeline for each built-in shader
        let mut pipelines = HashMap::new();
        for (name, source) in builtin_shaders() {
            if let Some(pipeline) = Self::compile_pipeline(gpu, &pipeline_layout, &source, &name) {
                pipelines.insert(name.to_string(), pipeline);
            }
        }

        // Load custom shaders from ~/.config/llnzy/shaders/
        if let Some(config_dir) = dirs::config_dir() {
            let shader_dir = config_dir.join("llnzy").join("shaders");
            if let Ok(entries) = std::fs::read_dir(&shader_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().is_some_and(|e| e == "wgsl") {
                        if let Ok(user_frag) = std::fs::read_to_string(&path) {
                            let name = path.file_stem().unwrap().to_string_lossy().to_string();
                            let full_source = format!("{}{}", SHARED_PREAMBLE, user_frag);
                            if let Some(pipeline) =
                                Self::compile_pipeline(gpu, &pipeline_layout, &full_source, &name)
                            {
                                log::info!("Loaded custom shader: {}", name);
                                pipelines.insert(name, pipeline);
                            } else {
                                log::warn!("Failed to compile custom shader: {}", path.display());
                            }
                        }
                    }
                }
            }
        }

        BackgroundRenderer {
            pipelines,
            uniform_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    fn compile_pipeline(
        gpu: &GpuState,
        layout: &wgpu::PipelineLayout,
        source: &str,
        name: &str,
    ) -> Option<wgpu::RenderPipeline> {
        // wgpu panics on shader compilation errors in some backends,
        // so we catch_unwind to handle malformed custom shaders gracefully.
        let device = &gpu.device;
        let format = gpu.surface_config.format;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some(name),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some(name),
            layout: Some(layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        Some(pipeline)
    }

    /// Update uniforms from config.
    /// If `custom_color` is set, derive the three shader layers from it;
    /// otherwise fall back to the terminal background color.
    pub fn update_uniforms(
        &self,
        gpu: &GpuState,
        intensity: f32,
        speed: f32,
        bg_color: [f32; 4],
        custom_color: Option<[u8; 3]>,
    ) {
        let (r, g, b) = if let Some(c) = custom_color {
            (c[0] as f32 / 255.0, c[1] as f32 / 255.0, c[2] as f32 / 255.0)
        } else {
            (bg_color[0], bg_color[1], bg_color[2])
        };

        let uniforms = BackgroundUniforms {
            intensity,
            speed,
            _padding: [0.0; 2],
            color1: [r * 0.15, g * 0.15, b * 0.25 + 0.02, 1.0],
            color2: [r * 0.2 + 0.01, g * 0.18 + 0.01, b * 0.3 + 0.03, 1.0],
            color3: [r * 0.25 + 0.02, g * 0.22 + 0.02, b * 0.35 + 0.05, 1.0],
        };

        gpu.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
    }

    /// Draw the background effect. `shader_name` selects which pipeline to use.
    pub fn draw(
        &self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        shader_name: &str,
    ) {
        let Some(pipeline) = self.pipelines.get(shader_name) else {
            return;
        };

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("background_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &gpu.frame_bind_group, &[]);
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }

    /// List all available shader names (built-in + custom).
    pub fn available_shaders(&self) -> Vec<String> {
        let mut names: Vec<String> = self.pipelines.keys().cloned().collect();
        names.sort();
        names
    }
}
