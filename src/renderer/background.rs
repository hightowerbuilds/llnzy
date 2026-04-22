use bytemuck::{Pod, Zeroable};

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

const SMOKE_SHADER: &str = r#"
// ── Noise utilities ──

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

// ── Uniforms ──

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

// ── Vertex ──

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

// ── Fragment: living smoke ──

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let t = frame.time * bg.speed * 0.15;
    let aspect = frame.resolution.x / frame.resolution.y;
    var uv = in.uv;
    uv.x = uv.x * aspect;

    // Domain warping — displace the UV before sampling noise
    // This creates the organic, flowing smoke feel
    let warp1 = vec2<f32>(
        fbm(uv * 3.0 + vec2<f32>(t * 0.7, t * 0.3), 4),
        fbm(uv * 3.0 + vec2<f32>(t * 0.4 + 5.2, t * 0.6 + 1.3), 4)
    );

    let warp2 = vec2<f32>(
        fbm(uv * 3.0 + warp1 * 2.0 + vec2<f32>(t * 0.2 + 1.7, t * 0.5 + 9.2), 4),
        fbm(uv * 3.0 + warp1 * 2.0 + vec2<f32>(t * 0.3 + 8.3, t * 0.1 + 2.8), 4)
    );

    // Final smoke density from double-warped noise
    let smoke = fbm(uv * 2.0 + warp2 * 1.5, 5);

    // Create depth layers by mixing theme colors based on smoke density
    let layer1 = mix(bg.color1.rgb, bg.color2.rgb, smoothstep(0.2, 0.6, smoke));
    let layer2 = mix(layer1, bg.color3.rgb, smoothstep(0.4, 0.8, smoke));

    // Subtle brightness variation for glow in dense smoke regions
    let glow = smoothstep(0.5, 0.9, smoke) * 0.15;
    let color = layer2 + glow;

    return vec4<f32>(color, bg.intensity);
}
"#;

pub struct BackgroundRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    #[allow(dead_code)] // needed for future shader hot-swap
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl BackgroundRenderer {
    pub fn new(gpu: &GpuState) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("background_shader"),
                source: wgpu::ShaderSource::Wgsl(SMOKE_SHADER.into()),
            });

        // Background-specific uniforms at @group(1)
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

        let uniforms = BackgroundUniforms {
            intensity: 0.3,
            speed: 1.0,
            _padding: [0.0; 2],
            color1: [0.05, 0.05, 0.12, 1.0],
            color2: [0.12, 0.08, 0.18, 1.0],
            color3: [0.18, 0.12, 0.22, 1.0],
        };

        let uniform_buffer =
            gpu.device
                .create_buffer(&wgpu::BufferDescriptor {
                    label: Some("bg_uniforms"),
                    size: std::mem::size_of::<BackgroundUniforms>() as u64,
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });

        gpu.queue.write_buffer(
            &uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_bind_group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("bg_pipeline_layout"),
                    bind_group_layouts: &[
                        &gpu.frame_bind_group_layout, // @group(0) = frame uniforms
                        &bind_group_layout,            // @group(1) = background uniforms
                    ],
                    push_constant_ranges: &[],
                });

        let pipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("bg_pipeline"),
                    layout: Some(&pipeline_layout),
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
                            format: gpu.surface_config.format,
                            // Alpha blending so smoke composites over the cleared background
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

        BackgroundRenderer {
            pipeline,
            uniform_buffer,
            bind_group_layout,
            bind_group,
        }
    }

    /// Update uniforms from config (called on config hot-reload).
    pub fn update_uniforms(
        &self,
        gpu: &GpuState,
        intensity: f32,
        speed: f32,
        bg_color: [f32; 4],
    ) {
        // Derive smoke colors from the terminal background color
        // color1 = base (darkest), color2 = mid smoke, color3 = highlights
        let r = bg_color[0];
        let g = bg_color[1];
        let b = bg_color[2];

        // Blue-grey smoke palette
        let uniforms = BackgroundUniforms {
            intensity,
            speed,
            _padding: [0.0; 2],
            color1: [r * 0.25 + 0.02, g * 0.28 + 0.03, b * 0.5 + 0.08, 1.0],
            color2: [r * 0.3 + 0.06, g * 0.35 + 0.08, b * 0.6 + 0.14, 1.0],
            color3: [r * 0.4 + 0.12, g * 0.5 + 0.14, b * 0.7 + 0.18, 1.0],
        };

        gpu.queue.write_buffer(
            &self.uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );
    }

    /// Draw the background effect into the given render target.
    pub fn draw(
        &self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("background_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load, // Load the cleared background
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &gpu.frame_bind_group, &[]);
        pass.set_bind_group(1, &self.bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
