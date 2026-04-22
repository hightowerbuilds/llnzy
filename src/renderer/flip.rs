use bytemuck::{Pod, Zeroable};

use super::state::GpuState;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct FlipUniforms {
    angle: f32,      // rotation in radians (0 = front, PI = back)
    aspect: f32,     // width / height
    _padding: [f32; 2],
}

const FLIP_SHADER: &str = r#"
@group(0) @binding(0) var front_tex: texture_2d<f32>;
@group(0) @binding(1) var back_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct FlipUniforms {
    angle: f32,
    aspect: f32,
    _padding: vec2<f32>,
};
@group(1) @binding(0) var<uniform> flip: FlipUniforms;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) vi: u32) -> VertexOutput {
    var out: VertexOutput;
    // Fullscreen quad UV
    let uv = vec2<f32>(f32(vi & 1u) * 2.0, f32(vi >> 1u) * 2.0);
    out.uv = uv;

    // Map UV to [-1, 1] centered coordinates
    let centered = uv * 2.0 - 1.0;

    // Apply Y-axis rotation (perspective projection)
    let cos_a = cos(flip.angle);
    let sin_a = sin(flip.angle);

    // Rotate X coordinate around Y axis
    let rotated_x = centered.x * cos_a;
    let rotated_z = centered.x * sin_a;

    // Perspective: objects further away (larger z) appear smaller
    let perspective_dist = 2.5; // camera distance
    let scale = perspective_dist / (perspective_dist - rotated_z * 0.5);

    let final_x = rotated_x * scale;
    let final_y = centered.y * scale; // flip Y for wgpu

    out.position = vec4<f32>(final_x, -final_y, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Determine if we're showing front or back based on angle
    let cos_a = cos(flip.angle);

    if (cos_a > 0.0) {
        // Front face (terminal)
        return textureSample(front_tex, tex_sampler, in.uv);
    } else {
        // Back face (settings) — mirror the X to correct orientation
        let mirrored_uv = vec2<f32>(1.0 - in.uv.x, in.uv.y);
        return textureSample(back_tex, tex_sampler, mirrored_uv);
    }
}
"#;

pub struct FlipAnimation {
    pipeline: wgpu::RenderPipeline,
    tex_layout: wgpu::BindGroupLayout,
    uniform_buffer: wgpu::Buffer,
    #[allow(dead_code)]
    params_layout: wgpu::BindGroupLayout,
    params_bind_group: wgpu::BindGroup,
    sampler: wgpu::Sampler,
}

impl FlipAnimation {
    pub fn new(gpu: &GpuState) -> Self {
        let shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("flip_shader"),
            source: wgpu::ShaderSource::Wgsl(FLIP_SHADER.into()),
        });

        let tex_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flip_tex_layout"),
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
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let params_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("flip_params_layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("flip_uniforms"),
            size: std::mem::size_of::<FlipUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("flip_params_bg"),
            layout: &params_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("flip_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("flip_pipeline"),
            layout: Some(&gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&tex_layout, &params_layout],
                push_constant_ranges: &[],
            })),
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

        FlipAnimation {
            pipeline,
            tex_layout,
            uniform_buffer,
            params_layout,
            params_bind_group,
            sampler,
        }
    }

    /// Render the flip effect. `angle` is 0.0 (front) to PI (back).
    pub fn draw(
        &self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        front_view: &wgpu::TextureView,
        back_view: &wgpu::TextureView,
        target: &wgpu::TextureView,
        angle: f32,
    ) {
        let uniforms = FlipUniforms {
            angle,
            aspect: gpu.surface_config.width as f32 / gpu.surface_config.height as f32,
            _padding: [0.0; 2],
        };
        gpu.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let tex_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(front_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(back_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("flip_pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
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
