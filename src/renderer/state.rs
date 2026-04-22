use std::sync::Arc;
use std::time::Instant;

use bytemuck::{Pod, Zeroable};
use winit::window::Window;

/// Per-frame data shared with all GPU shaders via @group(1) @binding(0).
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FrameUniforms {
    pub time: f32,
    pub delta_time: f32,
    pub resolution: [f32; 2],
    pub frame: u32,
    pub _padding: [u32; 3],
}

pub struct GpuState {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub surface: wgpu::Surface<'static>,
    pub surface_config: wgpu::SurfaceConfiguration,
    // Frame uniforms
    pub frame_uniform_buffer: wgpu::Buffer,
    pub frame_bind_group_layout: wgpu::BindGroupLayout,
    pub frame_bind_group: wgpu::BindGroup,
    // Offscreen scene textures (content renders to A, post-processing ping-pongs A<->B)
    pub scene_texture: wgpu::Texture,
    pub scene_view: wgpu::TextureView,
    pub scene_texture_b: wgpu::Texture,
    pub scene_view_b: wgpu::TextureView,
    pub scene_sampler: wgpu::Sampler,
    start_time: Instant,
    last_frame_time: Instant,
    frame_count: u32,
    // Exposed for particle system and other per-frame consumers
    pub current_time: f32,
    pub current_delta: f32,
}

impl GpuState {
    pub async fn new(window: Arc<Window>) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("llnzy"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    ..Default::default()
                },
                None,
            )
            .await
            .expect("Failed to create GPU device");

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Frame uniforms — shared across all effect shaders
        let frame_uniform_buffer =
            device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("frame_uniforms"),
                size: std::mem::size_of::<FrameUniforms>() as u64,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

        let frame_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("frame_bind_group_layout"),
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

        let frame_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("frame_bind_group"),
            layout: &frame_bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: frame_uniform_buffer.as_entire_binding(),
            }],
        });

        // Offscreen scene textures (A for content, B for post-process ping-pong)
        let (scene_texture, scene_view) =
            Self::create_scene_texture(&device, &surface_config, "scene_texture_a");
        let (scene_texture_b, scene_view_b) =
            Self::create_scene_texture(&device, &surface_config, "scene_texture_b");
        let scene_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("scene_sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let now = Instant::now();

        GpuState {
            device,
            queue,
            surface,
            surface_config,
            frame_uniform_buffer,
            frame_bind_group_layout,
            frame_bind_group,
            scene_texture,
            scene_view,
            scene_texture_b,
            scene_view_b,
            scene_sampler,
            start_time: now,
            last_frame_time: now,
            frame_count: 0,
            current_time: 0.0,
            current_delta: 0.0,
        }
    }

    /// Update frame uniforms — call once at the start of each render().
    pub fn update_frame_uniforms(&mut self) {
        let now = Instant::now();
        let time = now.duration_since(self.start_time).as_secs_f32();
        let delta_time = now.duration_since(self.last_frame_time).as_secs_f32();
        self.last_frame_time = now;
        self.frame_count = self.frame_count.wrapping_add(1);
        self.current_time = time;
        self.current_delta = delta_time;

        let uniforms = FrameUniforms {
            time,
            delta_time,
            resolution: [
                self.surface_config.width as f32,
                self.surface_config.height as f32,
            ],
            frame: self.frame_count,
            _padding: [0; 3],
        };

        self.queue.write_buffer(
            &self.frame_uniform_buffer,
            0,
            bytemuck::cast_slice(&[uniforms]),
        );
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width > 0 && height > 0 {
            self.surface_config.width = width;
            self.surface_config.height = height;
            self.surface.configure(&self.device, &self.surface_config);
            // Recreate scene textures at new resolution
            let (tex, view) =
                Self::create_scene_texture(&self.device, &self.surface_config, "scene_texture_a");
            self.scene_texture = tex;
            self.scene_view = view;
            let (tex_b, view_b) =
                Self::create_scene_texture(&self.device, &self.surface_config, "scene_texture_b");
            self.scene_texture_b = tex_b;
            self.scene_view_b = view_b;
        }
    }

    fn create_scene_texture(
        device: &wgpu::Device,
        config: &wgpu::SurfaceConfiguration,
        label: &str,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: wgpu::Extent3d {
                width: config.width.max(1),
                height: config.height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: config.format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        (texture, view)
    }
}
