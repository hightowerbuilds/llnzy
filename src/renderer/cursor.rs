use bytemuck::{Pod, Zeroable};

use super::state::GpuState;
use crate::config::CursorStyle;

const MAX_TRAIL: usize = 12;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CursorUniforms {
    pos: [f32; 2],   // cursor pixel position
    size: [f32; 2],  // cursor width, height in pixels
    color: [f32; 4], // cursor color
    resolution: [f32; 2],
    time: f32,
    glow_radius: f32, // glow spread in pixels
    pulse_speed: f32,
    trail_count: f32,
    _padding: [f32; 2],
}

// Trail positions packed as vec4s (xy = position, zw = size) for the shader
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct TrailData {
    entries: [[f32; 4]; MAX_TRAIL], // [x, y, alpha, _pad] per entry
}

const CURSOR_SHADER: &str = r#"
struct CursorUniforms {
    pos: vec2<f32>,
    size: vec2<f32>,
    color: vec4<f32>,
    resolution: vec2<f32>,
    time: f32,
    glow_radius: f32,
    pulse_speed: f32,
    trail_count: f32,
    _padding: vec2<f32>,
};

struct TrailData {
    entries: array<vec4<f32>, 12>,  // x, y, alpha, _pad
};

@group(0) @binding(0) var<uniform> cursor: CursorUniforms;
@group(0) @binding(1) var<uniform> trail: TrailData;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) instance: f32,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vi: u32,
    @builtin(instance_index) ii: u32,
) -> VertexOutput {
    var out: VertexOutput;
    out.instance = f32(ii);

    // Quad vertex position
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

    // Instance 0 = main cursor, instances 1+ = trail
    var center: vec2<f32>;
    var expand: f32;

    if (ii == 0u) {
        center = cursor.pos + cursor.size * 0.5;
        // Expand quad to include glow radius
        expand = max(cursor.size.x, cursor.size.y) * 0.5 + cursor.glow_radius;
    } else {
        let trail_idx = ii - 1u;
        if (trail_idx >= u32(cursor.trail_count)) {
            // Hide unused trail instances off-screen
            out.position = vec4<f32>(-10.0, -10.0, 0.0, 1.0);
            return out;
        }
        let t = trail.entries[trail_idx];
        center = vec2<f32>(t.x, t.y) + cursor.size * 0.5;
        expand = max(cursor.size.x, cursor.size.y) * 0.5 + cursor.glow_radius * 0.5;
    }

    let pixel_pos = center + local * expand;
    let ndc = vec2<f32>(
        pixel_pos.x / cursor.resolution.x * 2.0 - 1.0,
        1.0 - pixel_pos.y / cursor.resolution.y * 2.0,
    );
    out.position = vec4<f32>(ndc, 0.0, 1.0);

    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let ii = u32(in.instance);

    // Reconstruct pixel position from UV
    var center: vec2<f32>;
    var expand: f32;
    var base_alpha: f32;

    if (ii == 0u) {
        center = cursor.pos + cursor.size * 0.5;
        expand = max(cursor.size.x, cursor.size.y) * 0.5 + cursor.glow_radius;
        base_alpha = 1.0;
    } else {
        let trail_idx = ii - 1u;
        let t = trail.entries[trail_idx];
        center = vec2<f32>(t.x, t.y) + cursor.size * 0.5;
        expand = max(cursor.size.x, cursor.size.y) * 0.5 + cursor.glow_radius * 0.5;
        base_alpha = t.z; // trail alpha
    }

    let frag_pos = center + in.uv * expand;

    // Distance from fragment to cursor rect edges
    let half_size = cursor.size * 0.5;
    let d = abs(frag_pos - center) - half_size;
    let outside_dist = length(max(d, vec2<f32>(0.0)));
    let inside_dist = min(max(d.x, d.y), 0.0);
    let sdf = outside_dist + inside_dist;

    // Pulse: oscillate brightness
    let pulse = 0.85 + 0.15 * sin(cursor.time * cursor.pulse_speed);

    // Core cursor (solid inside the rect)
    let core_alpha = 1.0 - smoothstep(-0.5, 0.5, sdf);

    // Glow (soft falloff outside the rect)
    let glow_alpha = 1.0 - smoothstep(0.0, cursor.glow_radius, sdf);
    let glow_strength = glow_alpha * glow_alpha * 0.4; // quadratic falloff

    let alpha = (core_alpha + glow_strength) * pulse * base_alpha;

    if (alpha < 0.005) { discard; }

    return vec4<f32>(cursor.color.rgb * pulse, alpha);
}
"#;

pub struct CursorRenderer {
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    trail_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    trail_positions: Vec<(f32, f32)>, // pixel positions
    last_cursor_pos: Option<(f32, f32)>,
}

impl CursorRenderer {
    pub fn new(gpu: &GpuState) -> Self {
        let shader = gpu
            .device
            .create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("cursor_shader"),
                source: wgpu::ShaderSource::Wgsl(CURSOR_SHADER.into()),
            });

        let bind_group_layout =
            gpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label: Some("cursor_bgl"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding: 0,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding: 1,
                            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                            ty: wgpu::BindingType::Buffer {
                                ty: wgpu::BufferBindingType::Uniform,
                                has_dynamic_offset: false,
                                min_binding_size: None,
                            },
                            count: None,
                        },
                    ],
                });

        let uniform_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cursor_uniforms"),
            size: std::mem::size_of::<CursorUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let trail_buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("cursor_trail"),
            size: std::mem::size_of::<TrailData>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = gpu.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("cursor_bg"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: trail_buffer.as_entire_binding(),
                },
            ],
        });

        let pipeline =
            gpu.device
                .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some("cursor_pipeline"),
                    layout: Some(&gpu.device.create_pipeline_layout(
                        &wgpu::PipelineLayoutDescriptor {
                            label: None,
                            bind_group_layouts: &[&bind_group_layout],
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

        CursorRenderer {
            pipeline,
            uniform_buffer,
            trail_buffer,
            bind_group,
            trail_positions: Vec::new(),
            last_cursor_pos: None,
        }
    }

    /// Call each frame with the cursor's pixel position. Updates the trail.
    pub fn draw(
        &mut self,
        gpu: &GpuState,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::TextureView,
        cursor_row: usize,
        cursor_col: usize,
        cell_w: f32,
        cell_h: f32,
        offset_x: f32,
        offset_y: f32,
        cursor_style: CursorStyle,
        cursor_color: [u8; 3],
        time: f32,
        trail_enabled: bool,
    ) {
        let px = cursor_col as f32 * cell_w + offset_x;
        let py = cursor_row as f32 * cell_h + offset_y;

        let (cw, ch) = match cursor_style {
            CursorStyle::Block => (cell_w, cell_h),
            CursorStyle::Beam => (2.0, cell_h),
            CursorStyle::Underline => (cell_w, 2.0),
        };

        let cursor_px = match cursor_style {
            CursorStyle::Underline => (px, py + cell_h - 2.0),
            _ => (px, py),
        };

        // Update trail
        if trail_enabled {
            let new_pos = (cursor_px.0, cursor_px.1);
            let moved = self.last_cursor_pos.map_or(true, |last| {
                (last.0 - new_pos.0).abs() > 0.5 || (last.1 - new_pos.1).abs() > 0.5
            });
            if moved {
                if let Some(last) = self.last_cursor_pos {
                    self.trail_positions.insert(0, last);
                    if self.trail_positions.len() > MAX_TRAIL {
                        self.trail_positions.truncate(MAX_TRAIL);
                    }
                }
                self.last_cursor_pos = Some(new_pos);
            }
        } else {
            self.trail_positions.clear();
            self.last_cursor_pos = Some((cursor_px.0, cursor_px.1));
        }

        // Build trail uniform data
        let mut trail_data = TrailData {
            entries: [[0.0; 4]; MAX_TRAIL],
        };
        for (i, &(tx, ty)) in self.trail_positions.iter().enumerate() {
            if i >= MAX_TRAIL {
                break;
            }
            let alpha = 1.0 - (i as f32 + 1.0) / (MAX_TRAIL as f32 + 1.0);
            trail_data.entries[i] = [tx, ty, alpha * alpha * 0.5, 0.0]; // quadratic falloff
        }

        let color = [
            cursor_color[0] as f32 / 255.0,
            cursor_color[1] as f32 / 255.0,
            cursor_color[2] as f32 / 255.0,
            1.0,
        ];

        let uniforms = CursorUniforms {
            pos: [cursor_px.0, cursor_px.1],
            size: [cw, ch],
            color,
            resolution: [
                gpu.surface_config.width as f32,
                gpu.surface_config.height as f32,
            ],
            time,
            glow_radius: 12.0,
            pulse_speed: 3.0,
            trail_count: self.trail_positions.len() as f32,
            _padding: [0.0; 2],
        };

        gpu.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));
        gpu.queue
            .write_buffer(&self.trail_buffer, 0, bytemuck::cast_slice(&[trail_data]));

        let instance_count = 1 + self.trail_positions.len() as u32;

        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("cursor_pass"),
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

        pass.set_pipeline(&self.pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(0..6, 0..instance_count);
    }
}
