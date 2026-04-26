use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

use super::state::GpuState;

const MAX_PARTICLES: u32 = 4096;

/// Per-particle data stored in a GPU storage buffer.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct Particle {
    pos: [f32; 2],
    vel: [f32; 2],
    color: [f32; 4],
    life: f32,
    max_life: f32,
    size: f32,
    _pad: f32,
}

/// Uniforms passed to the compute + render shaders.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ParticleUniforms {
    delta_time: f32,
    time: f32,
    resolution: [f32; 2],
    count: u32,
    speed: f32,
    _pad: [f32; 2],
}

const COMPUTE_SHADER: &str = r#"
struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color: vec4<f32>,
    life: f32,
    max_life: f32,
    size: f32,
    _pad: f32,
};

struct Params {
    delta_time: f32,
    time: f32,
    resolution: vec2<f32>,
    count: u32,
    speed: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<storage, read_write> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: Params;

// Simple hash for pseudo-random respawn
fn hash(n: f32) -> f32 {
    return fract(sin(n) * 43758.5453123);
}

@compute @workgroup_size(256)
fn cs_main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let idx = gid.x;
    if (idx >= params.count) { return; }

    var p = particles[idx];
    let dt = params.delta_time * params.speed;

    // Age the particle
    p.life = p.life - dt;

    if (p.life <= 0.0) {
        // Respawn with pseudo-random position and properties
        let seed = f32(idx) * 1.618 + params.time * 0.1;
        p.pos = vec2<f32>(
            hash(seed) * 2.0 - 0.5,        // x: spread across screen width
            hash(seed + 1.0) * 2.0 - 0.5    // y: spread across screen height
        );
        p.vel = vec2<f32>(
            (hash(seed + 2.0) - 0.5) * 0.03,  // slow horizontal drift
            (hash(seed + 3.0) - 0.5) * 0.02 - 0.005  // slight upward drift
        );
        p.max_life = 3.0 + hash(seed + 4.0) * 6.0;  // 3-9 seconds
        p.life = p.max_life;
        p.size = 1.0 + hash(seed + 5.0) * 1.5;  // 1.0-2.5 px

        // Soft blue-white color with variation
        let hue = hash(seed + 6.0);
        p.color = vec4<f32>(
            0.5 + hue * 0.3,
            0.6 + hue * 0.2,
            0.8 + hue * 0.2,
            1.0
        );
    } else {
        // Drift with gentle sine wobble
        let wobble = sin(params.time * 1.5 + f32(idx) * 0.7) * 0.005;
        p.pos = p.pos + (p.vel + vec2<f32>(wobble, 0.0)) * dt;

        // Wrap around edges
        if (p.pos.x < -0.1) { p.pos.x = 1.1; }
        if (p.pos.x > 1.1) { p.pos.x = -0.1; }
        if (p.pos.y < -0.1) { p.pos.y = 1.1; }
        if (p.pos.y > 1.1) { p.pos.y = -0.1; }
    }

    particles[idx] = p;
}
"#;

const RENDER_SHADER: &str = r#"
struct Particle {
    pos: vec2<f32>,
    vel: vec2<f32>,
    color: vec4<f32>,
    life: f32,
    max_life: f32,
    size: f32,
    _pad: f32,
};

struct Params {
    delta_time: f32,
    time: f32,
    resolution: vec2<f32>,
    count: u32,
    speed: f32,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<storage, read> particles: array<Particle>;
@group(0) @binding(1) var<uniform> params: Params;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) uv: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
    var out: VertexOutput;

    let p = particles[ii];

    // Life-based alpha: fade in at start, fade out at end
    let life_frac = p.life / p.max_life;
    let alpha = smoothstep(0.0, 0.1, life_frac) * smoothstep(1.0, 0.8, life_frac);

    // Pulsing brightness
    let pulse = 0.7 + 0.3 * sin(params.time * 2.0 + f32(ii) * 1.3);

    out.color = vec4<f32>(p.color.rgb * pulse, alpha * 0.7);

    // Quad vertices: two triangles forming a quad
    var local: vec2<f32>;
    switch vi {
        case 0u: { local = vec2<f32>(-1.0, -1.0); }
        case 1u: { local = vec2<f32>(1.0, -1.0); }
        case 2u: { local = vec2<f32>(-1.0, 1.0); }
        case 3u: { local = vec2<f32>(1.0, -1.0); }
        case 4u: { local = vec2<f32>(1.0, 1.0); }
        default: { local = vec2<f32>(-1.0, 1.0); }
    }
    out.uv = local;

    // Scale size by resolution to keep particles consistent
    let pixel_size = p.size / params.resolution;
    let world_pos = p.pos + local * pixel_size;

    // Map [0,1] position to clip space [-1,1], Y flipped
    out.position = vec4<f32>(world_pos.x * 2.0 - 1.0, 1.0 - world_pos.y * 2.0, 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Soft circular falloff
    let dist = length(in.uv);
    let softness = 1.0 - smoothstep(0.3, 1.0, dist);
    if (softness <= 0.0) { discard; }
    return vec4<f32>(in.color.rgb, in.color.a * softness);
}
"#;

pub struct ParticleSystem {
    compute_pipeline: wgpu::ComputePipeline,
    render_pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    compute_bind_group: wgpu::BindGroup,
    render_bind_group: wgpu::BindGroup,
    count: u32,
}

impl ParticleSystem {
    pub fn new(gpu: &GpuState, count: u32) -> Self {
        let count = count.min(MAX_PARTICLES);

        // Initialize particles with zero life so they respawn immediately
        let particles: Vec<Particle> = (0..MAX_PARTICLES)
            .map(|_| Particle {
                pos: [0.0; 2],
                vel: [0.0; 2],
                color: [0.0; 4],
                life: 0.0,
                max_life: 1.0,
                size: 1.5,
                _pad: 0.0,
            })
            .collect();

        let particle_buffer = gpu
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("particle_buffer"),
                contents: bytemuck::cast_slice(&particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::VERTEX,
            });

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("particle_uniforms"),
            size: std::mem::size_of::<ParticleUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Compute bind group layout (read-write storage, compute only)
        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("particle_compute_bgl"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: false },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::COMPUTE,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        // We need a separate layout for render (read-only storage)
        let render_bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("particle_render_bgl"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Storage { read_only: true },
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let compute_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_compute_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        let render_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("particle_render_bg"),
            layout: &render_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: particle_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: uniform_buffer.as_entire_binding(),
                },
            ],
        });

        // Compute pipeline
        let compute_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("particle_compute"),
                source: wgpu::ShaderSource::Wgsl(COMPUTE_SHADER.into()),
            });

        let compute_pipeline =
            gpu.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label: Some("particle_compute_pipeline"),
                    layout: Some(&gpu.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: &[&bind_group_layout],
                            push_constant_ranges: &[],
                        },
                    )),
                    module: &compute_shader,
                    entry_point: "cs_main",
                    compilation_options: Default::default(),
                    cache: None,
                });

        // Render pipeline
        let render_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("particle_render"),
                source: wgpu::ShaderSource::Wgsl(RENDER_SHADER.into()),
            });

        let render_pipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("particle_render_pipeline"),
                    layout: Some(&gpu.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: &[&render_bind_group_layout],
                            push_constant_ranges: &[],
                        },
                    )),
                    vertex: wgpu::VertexState {
                        module: &render_shader,
                        entry_point: "vs_main",
                        buffers: &[],
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: &render_shader,
                        entry_point: "fs_main",
                        targets: &[Some(wgpu::ColorTargetState {
                            format: gpu.surface_config.format,
                            // Additive blending for glowing particles
                            blend: Some(wgpu::BlendState {
                                color: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::SrcAlpha,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                                alpha: wgpu::BlendComponent {
                                    src_factor: wgpu::BlendFactor::One,
                                    dst_factor: wgpu::BlendFactor::One,
                                    operation: wgpu::BlendOperation::Add,
                                },
                            }),
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

        ParticleSystem {
            compute_pipeline,
            render_pipeline,
            uniform_buffer,
            compute_bind_group,
            render_bind_group,
            count,
        }
    }

    pub fn set_count(&mut self, count: u32) {
        self.count = count.min(MAX_PARTICLES);
    }

    /// Update particles via compute shader, then render them.
    pub fn update_and_draw(
        &self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        time: f32,
        delta_time: f32,
        speed: f32,
    ) {
        let uniforms = ParticleUniforms {
            delta_time,
            time,
            resolution: [
                gpu.surface_config.width as f32,
                gpu.surface_config.height as f32,
            ],
            count: self.count,
            speed,
            _pad: [0.0; 2],
        };
        gpu.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        // Compute pass: update particle positions
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("particle_compute_pass"),
                ..Default::default()
            });
            pass.set_pipeline(&self.compute_pipeline);
            pass.set_bind_group(0, &self.compute_bind_group, &[]);
            pass.dispatch_workgroups(self.count.div_ceil(256), 1, 1);
        }

        // Render pass: draw particles as instanced quads
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("particle_render_pass"),
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
            pass.set_pipeline(&self.render_pipeline);
            pass.set_bind_group(0, &self.render_bind_group, &[]);
            pass.draw(0..6, 0..self.count); // 6 vertices per quad, N instances
        }
    }
}
