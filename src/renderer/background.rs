use bytemuck::{Pod, Zeroable};
use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};

use super::state::GpuState;
use crate::config::BackgroundImageFit;

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

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct ImageUniforms {
    screen_and_image_size: [f32; 4],
    mode: [f32; 4],
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

pub const BUILTIN_SHADER_NAMES: &[&str] = &["smoke", "aurora"];

const BUILTIN_SHADER_SOURCES: &[(&str, &str)] =
    &[("smoke", SMOKE_FRAGMENT), ("aurora", AURORA_FRAGMENT)];

fn builtin_shaders() -> Vec<(&'static str, String)> {
    BUILTIN_SHADER_SOURCES
        .iter()
        .map(|(name, frag)| (*name, format!("{}{}", SHARED_PREAMBLE, frag)))
        .collect()
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct CustomShaderSource {
    name: String,
    source: String,
}

pub fn custom_shader_names_from_dir(shader_dir: &Path) -> Vec<String> {
    custom_shader_files(shader_dir)
        .into_iter()
        .filter_map(|path| custom_shader_name(&path))
        .collect()
}

fn custom_shader_sources_from_dir(shader_dir: &Path) -> Vec<CustomShaderSource> {
    custom_shader_files(shader_dir)
        .into_iter()
        .filter_map(|path| {
            let name = custom_shader_name(&path)?;
            let fragment = std::fs::read_to_string(&path).ok()?;
            Some(CustomShaderSource {
                name,
                source: format!("{}{}", SHARED_PREAMBLE, fragment),
            })
        })
        .collect()
}

fn custom_shader_files(shader_dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(shader_dir) else {
        return Vec::new();
    };
    let mut paths: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("wgsl"))
        .collect();
    paths.sort();
    paths
}

fn custom_shader_name(path: &Path) -> Option<String> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(str::to_string)
}

// ── Image background blit shader ──

const IMAGE_SHADER: &str = r#"
@group(0) @binding(0) var img_tex: texture_2d<f32>;
@group(0) @binding(1) var img_sampler: sampler;

struct ImageUniforms {
    screen_and_image_size: vec4<f32>,
    mode: vec4<f32>,
};
@group(0) @binding(2) var<uniform> img_params: ImageUniforms;

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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let screen = max(img_params.screen_and_image_size.xy, vec2<f32>(1.0, 1.0));
    let img = max(img_params.screen_and_image_size.zw, vec2<f32>(1.0, 1.0));
    let frag_px = in.uv * screen;

    var sample_uv = in.uv;
    if (img_params.mode.x < 0.5) {
        let scale = max(screen.x / img.x, screen.y / img.y);
        let draw_size = img * scale;
        let origin = (screen - draw_size) * 0.5;
        sample_uv = (frag_px - origin) / draw_size;
    } else if (img_params.mode.x < 1.5) {
        let scale = min(screen.x / img.x, screen.y / img.y);
        let draw_size = img * scale;
        let origin = (screen - draw_size) * 0.5;
        sample_uv = (frag_px - origin) / draw_size;
    } else if (img_params.mode.x < 2.5) {
        sample_uv = fract(frag_px / img);
        return textureSample(img_tex, img_sampler, sample_uv);
    } else {
        let origin = (screen - img) * 0.5;
        sample_uv = (frag_px - origin) / img;
    }

    if (sample_uv.x < 0.0 || sample_uv.x > 1.0 || sample_uv.y < 0.0 || sample_uv.y > 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return textureSample(img_tex, img_sampler, sample_uv);
}
"#;

pub struct BackgroundRenderer {
    pipelines: FxHashMap<String, wgpu::RenderPipeline>,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    // Image background support
    image_pipeline: wgpu::RenderPipeline,
    image_tex_layout: wgpu::BindGroupLayout,
    image_sampler: wgpu::Sampler,
    image_uniform_buffer: wgpu::Buffer,
    image_bind_group: Option<wgpu::BindGroup>,
    loaded_image_path: Option<String>,
    rejected_image_path: Option<String>,
    loaded_image_size: [f32; 2],
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
        let mut pipelines = FxHashMap::default();
        for (name, source) in builtin_shaders() {
            if let Some(pipeline) = Self::compile_pipeline(gpu, &pipeline_layout, &source, name) {
                pipelines.insert(name.to_string(), pipeline);
            }
        }

        if let Some(paths) = crate::platform::paths::current_paths() {
            for shader in custom_shader_sources_from_dir(&paths.shaders_dir()) {
                if let Some(pipeline) =
                    Self::compile_pipeline(gpu, &pipeline_layout, &shader.source, &shader.name)
                {
                    log::info!("Loaded custom shader: {}", shader.name);
                    pipelines.insert(shader.name, pipeline);
                } else {
                    log::warn!("Failed to compile custom shader: {}", shader.name);
                }
            }
        }

        // Image background pipeline (separate bind group layout: texture + sampler)
        let image_tex_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("bg_image_tex_layout"),
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
                        wgpu::BindGroupLayoutEntry {
                            binding: 2,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let image_uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bg_image_uniforms"),
            size: std::mem::size_of::<ImageUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let image_sampler = gpu.device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("bg_image_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let image_shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("bg_image_shader"),
                source: wgpu::ShaderSource::Wgsl(IMAGE_SHADER.into()),
            });

        let image_pipeline_layout =
            gpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label: Some("bg_image_pipeline_layout"),
                    bind_group_layouts: &[&image_tex_layout],
                    push_constant_ranges: &[],
                });

        let image_pipeline = gpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some("bg_image_pipeline"),
                layout: Some(&image_pipeline_layout),
                vertex: wgpu::VertexState {
                    module: &image_shader,
                    entry_point: "vs_main",
                    buffers: &[],
                    compilation_options: Default::default(),
                },
                fragment: Some(wgpu::FragmentState {
                    module: &image_shader,
                    entry_point: "fs_main",
                    targets: &[Some(wgpu::ColorTargetState {
                        format: gpu.surface_config.format,
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
            pipelines,
            uniform_buffer,
            bind_group,
            image_pipeline,
            image_tex_layout,
            image_sampler,
            image_uniform_buffer,
            image_bind_group: None,
            loaded_image_path: None,
            rejected_image_path: None,
            loaded_image_size: [1.0, 1.0],
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
    /// If custom colors are set, use them as the three shader layers;
    /// otherwise fall back to layers derived from the terminal background color.
    pub fn update_uniforms(
        &self,
        gpu: &GpuState,
        intensity: f32,
        speed: f32,
        bg_color: [f32; 4],
        custom_colors: [Option<[u8; 3]>; 3],
    ) {
        let rgb = |color: [u8; 3]| {
            [
                color[0] as f32 / 255.0,
                color[1] as f32 / 255.0,
                color[2] as f32 / 255.0,
                1.0,
            ]
        };
        let derived = |mult: [f32; 3], add: [f32; 3]| {
            [
                bg_color[0] * mult[0] + add[0],
                bg_color[1] * mult[1] + add[1],
                bg_color[2] * mult[2] + add[2],
                1.0,
            ]
        };
        let primary = custom_colors[0];
        let color1 = primary
            .map(rgb)
            .unwrap_or_else(|| derived([0.15, 0.15, 0.25], [0.0, 0.0, 0.02]));
        let color2 = custom_colors[1]
            .or(primary.map(|c| brighten_rgb(c, [18, 32, 18])))
            .map(rgb)
            .unwrap_or_else(|| derived([0.2, 0.18, 0.3], [0.01, 0.01, 0.03]));
        let color3 = custom_colors[2]
            .or(primary.map(|c| brighten_rgb(c, [42, 28, 48])))
            .map(rgb)
            .unwrap_or_else(|| derived([0.25, 0.22, 0.35], [0.02, 0.02, 0.05]));

        let uniforms = BackgroundUniforms {
            intensity,
            speed,
            _padding: [0.0; 2],
            color1,
            color2,
            color3,
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

    /// Load an image from disk and upload it as a GPU texture.
    /// Skips if the path matches the already-loaded image.
    pub fn load_image(&mut self, gpu: &GpuState, path: &str) {
        if self.loaded_image_path.as_deref() == Some(path) {
            return;
        }
        if self.rejected_image_path.as_deref() == Some(path) {
            return;
        }

        if let Err(err) = crate::theme_store::validate_background_image(Path::new(path)) {
            log::warn!("Skipping background image: {err}");
            self.image_bind_group = None;
            self.loaded_image_path = None;
            self.rejected_image_path = Some(path.to_string());
            return;
        }

        let img = match image::open(path) {
            Ok(img) => img.to_rgba8(),
            Err(e) => {
                log::warn!("Failed to load background image: {e}");
                self.image_bind_group = None;
                self.loaded_image_path = None;
                self.rejected_image_path = Some(path.to_string());
                return;
            }
        };

        let (width, height) = (img.width(), img.height());
        let rgba = img.into_raw();

        let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("bg_image_texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        gpu.queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &rgba,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        self.image_bind_group = Some(gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bg_image_bind_group"),
            layout: &self.image_tex_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.image_sampler),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: self.image_uniform_buffer.as_entire_binding(),
                },
            ],
        }));

        self.loaded_image_path = Some(path.to_string());
        self.rejected_image_path = None;
        self.loaded_image_size = [width as f32, height as f32];
    }

    /// Draw the background image to the target view.
    pub fn draw_image(
        &self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        image_fit: BackgroundImageFit,
    ) {
        let Some(bind_group) = &self.image_bind_group else {
            return;
        };
        let uniforms = ImageUniforms {
            screen_and_image_size: [
                gpu.surface_config.width as f32,
                gpu.surface_config.height as f32,
                self.loaded_image_size[0],
                self.loaded_image_size[1],
            ],
            mode: [image_fit.shader_mode(), 0.0, 0.0, 0.0],
        };
        gpu.queue.write_buffer(
            &self.image_uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("background_image_pass"),
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

        pass.set_pipeline(&self.image_pipeline);
        pass.set_bind_group(0, bind_group, &[]);
        pass.draw(0..3, 0..1);
    }
}

fn brighten_rgb(color: [u8; 3], add: [u8; 3]) -> [u8; 3] {
    [
        color[0].saturating_add(add[0]),
        color[1].saturating_add(add[1]),
        color[2].saturating_add(add[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::{
        custom_shader_names_from_dir, custom_shader_sources_from_dir, BUILTIN_SHADER_NAMES,
        BUILTIN_SHADER_SOURCES,
    };
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn builtin_shader_names_match_registered_sources() {
        let source_names: Vec<&str> = BUILTIN_SHADER_SOURCES
            .iter()
            .map(|(name, _)| *name)
            .collect();

        assert_eq!(source_names, BUILTIN_SHADER_NAMES);
    }

    #[test]
    fn custom_shader_names_are_sorted_wgsl_file_stems() {
        let dir = temp_shader_dir("names");
        std::fs::write(dir.join("zebra.wgsl"), "z").unwrap();
        std::fs::write(dir.join("alpha.wgsl"), "a").unwrap();
        std::fs::write(dir.join("notes.txt"), "ignore").unwrap();

        let names = custom_shader_names_from_dir(&dir);

        assert_eq!(names, ["alpha", "zebra"]);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn custom_shader_sources_include_shared_preamble() {
        let dir = temp_shader_dir("sources");
        let fragment = r#"
@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.uv, 0.0, bg.intensity);
}
"#;
        std::fs::write(dir.join("custom.wgsl"), fragment).unwrap();

        let shaders = custom_shader_sources_from_dir(&dir);

        assert_eq!(shaders.len(), 1);
        assert_eq!(shaders[0].name, "custom");
        assert!(shaders[0].source.contains("struct FrameUniforms"));
        assert!(shaders[0].source.contains(fragment));

        let _ = std::fs::remove_dir_all(dir);
    }

    fn temp_shader_dir(label: &str) -> std::path::PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("llnzy-custom-shader-{label}-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }
}
