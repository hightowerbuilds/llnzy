use bytemuck::{Pod, Zeroable};

use super::state::GpuState;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct CrtUniforms {
    pub scanline_intensity: f32,
    pub curvature: f32,
    pub vignette_strength: f32,
    pub chromatic_aberration: f32,
    pub grain_intensity: f32,
    pub time: f32,
    pub _padding: [f32; 2],
}

#[derive(Clone, Copy)]
pub struct CrtParams {
    pub scanline_intensity: f32,
    pub curvature: f32,
    pub vignette_strength: f32,
    pub chromatic_aberration: f32,
    pub grain_intensity: f32,
    pub time: f32,
}

const CRT_SHADER: &str = r#"
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct CrtUniforms {
    scanline_intensity: f32,
    curvature: f32,
    vignette_strength: f32,
    chromatic_aberration: f32,
    grain_intensity: f32,
    time: f32,
    _padding: vec2<f32>,
};
@group(1) @binding(0) var<uniform> crt: CrtUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    let uv = vec2<f32>(f32(vi & 1u) * 2.0, f32(vi >> 1u) * 2.0);
    out.position = vec4<f32>(uv.x * 2.0 - 1.0, 1.0 - uv.y * 2.0, 0.0, 1.0);
    out.uv = uv;
    return out;
}

// Hash for film grain
fn grain_hash(p: vec2<f32>) -> f32 {
    var p3 = fract(vec3<f32>(p.x, p.y, p.x) * 0.1031);
    p3 = p3 + dot(p3, vec3<f32>(p3.y + 33.33, p3.z + 33.33, p3.x + 33.33));
    return fract((p3.x + p3.y) * p3.z);
}

// Barrel distortion for CRT curvature
fn barrel_distort(uv: vec2<f32>, amount: f32) -> vec2<f32> {
    let centered = uv * 2.0 - 1.0;
    let r2 = dot(centered, centered);
    let distorted = centered * (1.0 + amount * r2);
    return distorted * 0.5 + 0.5;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(src_tex));
    var uv = in.uv;

    // ── Barrel distortion (CRT curvature) ──
    if (crt.curvature > 0.0) {
        uv = barrel_distort(uv, crt.curvature);
        // Black outside the curved screen edge
        if (uv.x < 0.0 || uv.x > 1.0 || uv.y < 0.0 || uv.y > 1.0) {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
    }

    // ── Chromatic aberration ──
    var color: vec3<f32>;
    if (crt.chromatic_aberration > 0.0) {
        let offset = (uv - 0.5) * crt.chromatic_aberration * 0.01;
        let r = textureSample(src_tex, src_sampler, uv + offset).r;
        let g = textureSample(src_tex, src_sampler, uv).g;
        let b = textureSample(src_tex, src_sampler, uv - offset).b;
        color = vec3<f32>(r, g, b);
    } else {
        color = textureSample(src_tex, src_sampler, uv).rgb;
    }

    // ── Scanlines ──
    if (crt.scanline_intensity > 0.0) {
        let scan_y = uv.y * tex_size.y;
        let scanline = sin(scan_y * 3.14159) * 0.5 + 0.5;
        let darken = 1.0 - crt.scanline_intensity * (1.0 - scanline);
        color = color * darken;
    }

    // ── Vignette ──
    if (crt.vignette_strength > 0.0) {
        let centered = uv - 0.5;
        let dist = dot(centered, centered);
        let vignette = 1.0 - dist * crt.vignette_strength * 3.0;
        color = color * clamp(vignette, 0.0, 1.0);
    }

    // ── Film grain ──
    if (crt.grain_intensity > 0.0) {
        let grain_uv = uv * tex_size + vec2<f32>(crt.time * 100.0, crt.time * 73.0);
        let grain = (grain_hash(grain_uv) - 0.5) * crt.grain_intensity;
        color = color + grain;
    }

    return vec4<f32>(color, 1.0);
}
"#;

pub struct CrtEffect {
    pipeline: wgpu::RenderPipeline,
    tex_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    params_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
}

impl CrtEffect {
    pub fn new(gpu: &GpuState) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("crt_shader"),
                source: wgpu::ShaderSource::Wgsl(CRT_SHADER.into()),
            });

        let tex_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("crt_tex_layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                            view_dimension: wgpu::TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        let params_layout = gpu
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("crt_params_layout"),
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
            label: Some("crt_uniforms"),
            size: std::mem::size_of::<CrtUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("crt_params_bg"),
            layout: &params_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("crt_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("crt_pipeline"),
                    layout: Some(&gpu.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: &[&tex_layout, &params_layout],
                            push_constant_ranges: &[],
                        },
                    )),
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
                            blend: None,
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState::default(),
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });

        CrtEffect {
            pipeline,
            tex_layout,
            uniform_buffer,
            params_bind_group,
            sampler,
        }
    }

    /// Apply CRT effect: reads from src_view, writes to target_view.
    pub fn apply(
        &self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        src_view: &wgpu::TextureView,
        target_view: &wgpu::TextureView,
        params: CrtParams,
    ) {
        let uniforms = CrtUniforms {
            scanline_intensity: params.scanline_intensity,
            curvature: params.curvature,
            vignette_strength: params.vignette_strength,
            chromatic_aberration: params.chromatic_aberration,
            grain_intensity: params.grain_intensity,
            time: params.time,
            _padding: [0.0; 2],
        };
        gpu.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let tex_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(src_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("crt_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &tex_bg, &[]);
        pass.set_bind_group(1, &self.params_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}
