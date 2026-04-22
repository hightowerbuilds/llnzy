use bytemuck::{Pod, Zeroable};

use super::state::GpuState;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    radius: f32,
    direction: f32, // 0.0 = horizontal, 1.0 = vertical
}

const BLOOM_THRESHOLD_SHADER: &str = r#"
@group(0) @binding(0) var src_tex: texture_2d<f32>;
@group(0) @binding(1) var src_sampler: sampler;

struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    radius: f32,
    direction: f32,
};
@group(1) @binding(0) var<uniform> params: BloomUniforms;

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

@fragment
fn fs_threshold(in: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(src_tex, src_sampler, in.uv);
    let brightness = dot(color.rgb, vec3<f32>(0.2126, 0.7152, 0.0722));
    let contribution = smoothstep(params.threshold, params.threshold + 0.15, brightness);
    return vec4<f32>(color.rgb * contribution, 1.0);
}

@fragment
fn fs_blur(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(src_tex));
    let pixel = 1.0 / tex_size;

    // Direction: horizontal (1,0) or vertical (0,1)
    let dir = select(vec2<f32>(1.0, 0.0), vec2<f32>(0.0, 1.0), params.direction > 0.5);
    let step = dir * pixel * params.radius;

    // 9-tap Gaussian kernel (sigma ≈ 3)
    var result = textureSample(src_tex, src_sampler, in.uv).rgb * 0.2270;
    result += textureSample(src_tex, src_sampler, in.uv + step * 1.0).rgb * 0.1946;
    result += textureSample(src_tex, src_sampler, in.uv - step * 1.0).rgb * 0.1946;
    result += textureSample(src_tex, src_sampler, in.uv + step * 2.0).rgb * 0.1216;
    result += textureSample(src_tex, src_sampler, in.uv - step * 2.0).rgb * 0.1216;
    result += textureSample(src_tex, src_sampler, in.uv + step * 3.0).rgb * 0.0541;
    result += textureSample(src_tex, src_sampler, in.uv - step * 3.0).rgb * 0.0541;
    result += textureSample(src_tex, src_sampler, in.uv + step * 4.0).rgb * 0.0162;
    result += textureSample(src_tex, src_sampler, in.uv - step * 4.0).rgb * 0.0162;

    return vec4<f32>(result, 1.0);
}
"#;

const BLOOM_COMPOSITE_SHADER: &str = r#"
@group(0) @binding(0) var scene_tex: texture_2d<f32>;
@group(0) @binding(1) var bloom_tex: texture_2d<f32>;
@group(0) @binding(2) var tex_sampler: sampler;

struct BloomUniforms {
    threshold: f32,
    intensity: f32,
    radius: f32,
    direction: f32,
};
@group(1) @binding(0) var<uniform> params: BloomUniforms;

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

@fragment
fn fs_composite(in: VertexOutput) -> @location(0) vec4<f32> {
    let scene = textureSample(scene_tex, tex_sampler, in.uv);
    let bloom = textureSample(bloom_tex, tex_sampler, in.uv);
    return vec4<f32>(scene.rgb + bloom.rgb * params.intensity, scene.a);
}
"#;

pub struct BloomEffect {
    // Half-res textures for blur (ping-pong)
    bloom_a: wgpu::Texture,
    bloom_a_view: wgpu::TextureView,
    bloom_b: wgpu::Texture,
    bloom_b_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    // Pipelines
    threshold_pipeline: wgpu::RenderPipeline,
    blur_pipeline: wgpu::RenderPipeline,
    composite_pipeline: wgpu::RenderPipeline,
    // Bind group layouts
    tex_layout: wgpu::BindGroupLayout,
    composite_layout: wgpu::BindGroupLayout,
    #[allow(dead_code)]
    params_layout: wgpu::BindGroupLayout,
    // Uniform buffer
    params_buffer: wgpu::Buffer,
    params_bind_group: wgpu::BindGroup,
    // Current dimensions (half-res)
    half_width: u32,
    half_height: u32,
}

impl BloomEffect {
    pub fn new(gpu: &GpuState) -> Self {
        let half_width = (gpu.surface_config.width / 2).max(1);
        let half_height = (gpu.surface_config.height / 2).max(1);
        let format = gpu.surface_config.format;

        let sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bloom_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let (bloom_a, bloom_a_view) = create_bloom_texture(&gpu.device, half_width, half_height, format, "bloom_a");
        let (bloom_b, bloom_b_view) = create_bloom_texture(&gpu.device, half_width, half_height, format, "bloom_b");

        // Single-texture bind group layout (for threshold + blur passes)
        let tex_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_tex_layout"),
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

        // Composite bind group layout (scene + bloom + sampler)
        let composite_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_composite_layout"),
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

        // Params uniform layout
        let params_layout = gpu.device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("bloom_params_layout"),
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

        let params_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bloom_params"),
            size: std::mem::size_of::<BloomUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let params_bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_params_bg"),
            layout: &params_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: params_buffer.as_entire_binding(),
            }],
        });

        // Threshold + blur shader module
        let threshold_blur_shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_threshold_blur"),
            source: wgpu::ShaderSource::Wgsl(BLOOM_THRESHOLD_SHADER.into()),
        });

        // Composite shader module
        let composite_shader = gpu.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bloom_composite"),
            source: wgpu::ShaderSource::Wgsl(BLOOM_COMPOSITE_SHADER.into()),
        });

        // Threshold pipeline (full-res scene -> half-res bloom_a)
        let threshold_pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_threshold"),
            layout: Some(&gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&tex_layout, &params_layout],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &threshold_blur_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &threshold_blur_shader,
                entry_point: "fs_threshold",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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

        // Blur pipeline (ping-pong between bloom_a and bloom_b)
        let blur_pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_blur"),
            layout: Some(&gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&tex_layout, &params_layout],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &threshold_blur_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &threshold_blur_shader,
                entry_point: "fs_blur",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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

        // Composite pipeline (scene + bloom -> output)
        let composite_pipeline = gpu.device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("bloom_composite"),
            layout: Some(&gpu.device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: &[&composite_layout, &params_layout],
                push_constant_ranges: &[],
            })),
            vertex: wgpu::VertexState {
                module: &composite_shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &composite_shader,
                entry_point: "fs_composite",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
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

        BloomEffect {
            bloom_a, bloom_a_view,
            bloom_b, bloom_b_view,
            sampler,
            threshold_pipeline,
            blur_pipeline,
            composite_pipeline,
            tex_layout,
            composite_layout,
            params_layout,
            params_buffer,
            params_bind_group,
            half_width,
            half_height,
        }
    }

    pub fn resize(&mut self, gpu: &GpuState) {
        let hw = (gpu.surface_config.width / 2).max(1);
        let hh = (gpu.surface_config.height / 2).max(1);
        if hw != self.half_width || hh != self.half_height {
            self.half_width = hw;
            self.half_height = hh;
            let format = gpu.surface_config.format;
            let (a, av) = create_bloom_texture(&gpu.device, hw, hh, format, "bloom_a");
            let (b, bv) = create_bloom_texture(&gpu.device, hw, hh, format, "bloom_b");
            self.bloom_a = a;
            self.bloom_a_view = av;
            self.bloom_b = b;
            self.bloom_b_view = bv;
        }
    }

    /// Run the full bloom pipeline:
    /// 1. Threshold (scene -> bloom_a at half-res)
    /// 2. Blur horizontal (bloom_a -> bloom_b)
    /// 3. Blur vertical (bloom_b -> bloom_a)
    /// 4. Composite (scene + bloom_a -> swapchain or target)
    pub fn apply(
        &self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        scene_view: &wgpu::TextureView,
        output_view: &wgpu::TextureView,
        threshold: f32,
        intensity: f32,
        radius: f32,
    ) {
        // -- Step 1: Threshold (extract bright pixels, downsample to half-res)
        self.write_params(gpu, threshold, intensity, radius, 0.0);
        let scene_bg = self.make_tex_bind_group(gpu, scene_view);
        self.fullscreen_pass(encoder, &self.threshold_pipeline, &scene_bg, &self.bloom_a_view);

        // -- Step 2: Horizontal blur (bloom_a -> bloom_b)
        self.write_params(gpu, threshold, intensity, radius, 0.0); // direction = 0 (horizontal)
        let blur_h_bg = self.make_tex_bind_group(gpu, &self.bloom_a_view);
        self.fullscreen_pass(encoder, &self.blur_pipeline, &blur_h_bg, &self.bloom_b_view);

        // -- Step 3: Vertical blur (bloom_b -> bloom_a)
        self.write_params(gpu, threshold, intensity, radius, 1.0); // direction = 1 (vertical)
        let blur_v_bg = self.make_tex_bind_group(gpu, &self.bloom_b_view);
        self.fullscreen_pass(encoder, &self.blur_pipeline, &blur_v_bg, &self.bloom_a_view);

        // -- Step 4: Composite (scene + bloom_a -> output)
        self.write_params(gpu, threshold, intensity, radius, 0.0);
        let composite_bg = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bloom_composite_bg"),
            layout: &self.composite_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(scene_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.bloom_a_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });
        self.fullscreen_pass(encoder, &self.composite_pipeline, &composite_bg, output_view);
    }

    fn write_params(&self, gpu: &GpuState, threshold: f32, intensity: f32, radius: f32, direction: f32) {
        let params = BloomUniforms { threshold, intensity, radius, direction };
        gpu.queue.write_buffer(&self.params_buffer, 0, bytemuck::cast_slice(&[params]));
    }

    fn make_tex_bind_group(&self, gpu: &GpuState, view: &wgpu::TextureView) -> wgpu::BindGroup {
        gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: &self.tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        })
    }

    fn fullscreen_pass(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        pipeline: &wgpu::RenderPipeline,
        tex_bind_group: &wgpu::BindGroup,
        target: &wgpu::TextureView,
    ) {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, tex_bind_group, &[]);
        pass.set_bind_group(1, &self.params_bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn create_bloom_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    label: &str,
) -> (wgpu::Texture, wgpu::TextureView) {
    let tex = device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let view = tex.create_view(&wgpu::TextureViewDescriptor::default());
    (tex, view)
}
