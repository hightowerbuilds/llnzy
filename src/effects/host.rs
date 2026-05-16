//! Render an effect frame into a fresh NV12 CVPixelBuffer.
//!
//! GPUI 0.2.2's blade backend asserts that the `CVPixelBuffer` handed to
//! `Window::paint_surface` is `kCVPixelFormatType_420YpCbCr8BiPlanarFullRange`
//! (FourCC `'420f'`) -- NV12 biplanar YUV full-range. The `paint_surface`
//! hook was designed for video frames, so we generate the same format here:
//! plane 0 carries Y at full resolution, plane 1 carries Cb/Cr interleaved
//! at half resolution. Chroma subsampling halves color detail but the eye
//! is far more sensitive to luma; for ambient backgrounds this is invisible.
//!
//! ## M2 pipeline (wgpu)
//!
//! M1 filled both NV12 planes on the CPU with a gradient. M2 introduced a
//! wgpu render pipeline -- a fullscreen-triangle pass with built-in WGSL
//! fragment shaders writing into an RGBA8 texture. The texture is read back
//! to CPU bytes via a staging buffer, then we apply the same BT.601
//! full-range RGB->YUV conversion the M1 path used and write into the NV12
//! planes.
//!
//! The render texture and staging buffer are pooled by `(width, height)`
//! so steady-state rendering does no per-frame allocation. Resizes drop
//! the cached entry and rebuild it. Pipeline creation and frame rendering
//! run under wgpu error scopes; uncaptured errors and synchronous panics
//! disable further rendering on this host instead of taking down the app.
//!
//! Why readback instead of zero-copy IOSurface import: bridging
//! `metal-rs::Texture` to the `objc2-metal::MTLTexture` protocol expected by
//! `wgpu_hal::api::Metal::Device::texture_from_raw` would require a
//! cross-FFI transmute that we'd rather not introduce on the first wgpu
//! milestone. Unified memory on Apple Silicon keeps the readback cheap
//! enough to ship.

use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, OnceLock};
use std::time::Instant;

use core_foundation::{
    base::{CFType, TCFType},
    boolean::CFBoolean,
    dictionary::CFDictionary,
    string::CFString,
};
use core_video::pixel_buffer::{
    kCVPixelBufferIOSurfacePropertiesKey, kCVPixelBufferMetalCompatibilityKey,
    kCVPixelFormatType_420YpCbCr8BiPlanarFullRange, CVPixelBuffer,
};
use pollster::FutureExt as _;

/// wgpu requires that `bytes_per_row` for a buffer-image copy be a multiple
/// of 256. RGBA8 is 4 bytes/pixel, so the row stride we ask for is the
/// next multiple of 256 above `width * 4`.
const COPY_BYTES_PER_ROW_ALIGNMENT: u32 = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;

/// Render texture format. We chose `Rgba8Unorm` (linear) rather than the
/// sRGB variant because the BT.601 RGB->YUV conversion below treats inputs
/// as linear [0,1] floats; sampling an sRGB-encoded texture as if it were
/// linear would shift the Y/Cb/Cr values noticeably. When the real smoke
/// shader lands and decides on its own gamma model, this constant is the
/// single source of truth.
const RENDER_TEXTURE_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Size of the smoke shader's `Uniforms` block in bytes. 16 f32s laid out as
/// resolution + time + intensity + 3 vec4 colours.
const SMOKE_UNIFORM_BYTES: usize = 16 * 4;

/// Guardrails for the offscreen render target. The shader effect is a
/// full-window background, so unusually large backing-pixel dimensions can
/// allocate hundreds of megabytes across the render texture, staging buffer,
/// and destination CVPixelBuffer. Fail closed before asking Metal for a size
/// that is likely to trigger device loss.
const MAX_EFFECT_EDGE: u32 = 8192;
const MAX_EFFECT_PIXELS: u64 = 24_000_000;

/// User-facing render parameters threaded into every smoke frame.
#[derive(Clone, Copy, Debug)]
pub struct EffectParams {
    /// 0.0..1.0; scales the shader's final colour output.
    pub intensity: f32,
    /// Three palette stops in linear sRGB [0, 1]. The shader maps the FBM
    /// output through these via smoothstep'd `mix` operations.
    pub palette: [[f32; 4]; 3],
}

impl Default for EffectParams {
    fn default() -> Self {
        // The "Mauve" preset from the Effects appearance tab, in linear [0,1].
        // Picked so the gradient remains atmospheric when no config is wired.
        Self {
            intensity: 0.45,
            palette: [
                [
                    0x10 as f32 / 255.0,
                    0x09 as f32 / 255.0,
                    0x14 as f32 / 255.0,
                    0.0,
                ],
                [
                    0x4d as f32 / 255.0,
                    0x1f as f32 / 255.0,
                    0x4f as f32 / 255.0,
                    0.0,
                ],
                [
                    0xc5 as f32 / 255.0,
                    0x7a as f32 / 255.0,
                    0xc8 as f32 / 255.0,
                    0.0,
                ],
            ],
        }
    }
}

/// Which built-in effect shader to render. All variants share the same
/// `Uniforms` layout (and therefore the same bind group + uniform buffer);
/// only the shader module + render pipeline differ per kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EffectKind {
    Smoke,
    Fire,
    Aurora,
    Trees,
    Rain,
}

impl EffectKind {
    fn pipeline_index(self) -> usize {
        match self {
            EffectKind::Smoke => 0,
            EffectKind::Fire => 1,
            EffectKind::Aurora => 2,
            EffectKind::Trees => 3,
            EffectKind::Rain => 4,
        }
    }

    /// Maps `Config.effects.background` (a string) to a shader kind. None
    /// for any mode that isn't a shader effect (image / none / unknown).
    pub fn from_background_mode(mode: &str) -> Option<Self> {
        match mode {
            "smoke" => Some(EffectKind::Smoke),
            "fire" => Some(EffectKind::Fire),
            "aurora" => Some(EffectKind::Aurora),
            "trees" => Some(EffectKind::Trees),
            "rain" => Some(EffectKind::Rain),
            _ => None,
        }
    }
}

/// wgpu render texture + staging buffer pair cached by (width, height) so
/// the per-frame path can skip the texture/buffer create+drop syscalls.
/// Allocations only happen on first frame and on resize.
struct CachedRenderResources {
    width: u32,
    height: u32,
    texture: wgpu::Texture,
    view: wgpu::TextureView,
    staging: wgpu::Buffer,
    /// True if the staging buffer is currently mapped. We must `unmap()`
    /// before the next `map_async`, even if a previous frame returned
    /// early after mapping but before reading.
    mapped: bool,
}

/// Owns options passed into every CVPixelBuffer alloc and the wgpu device
/// state for the render pipelines. Built once via `EffectsHost::try_new` so
/// the per-frame path only does the per-frame work (uniform write, encode,
/// submit, map, copy).
pub struct EffectsHost {
    pixel_buffer_attrs: CFDictionary<CFString, CFType>,
    _instance: wgpu::Instance,
    _adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    /// One render pipeline per `EffectKind`, indexed by `EffectKind::pipeline_index`.
    /// All pipelines share the same bind group layout + uniform buffer.
    pipelines: [wgpu::RenderPipeline; 5],
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    /// Pooled per-frame render resources. The cache is invalidated and
    /// rebuilt when the requested dimensions change.
    cache: RefCell<Option<CachedRenderResources>>,
    /// Set to true by the wgpu uncaptured-error handler. Once raised, every
    /// future `render_frame` call returns `None` instead of attempting more
    /// GPU work that would also fail. Lets the caller degrade gracefully
    /// instead of panicking.
    disabled: Arc<AtomicBool>,
}

impl EffectsHost {
    pub fn try_new() -> Result<Self, String> {
        let mut instance_desc = wgpu::InstanceDescriptor::new_without_display_handle();
        instance_desc.backends = wgpu::Backends::METAL;
        let instance = wgpu::Instance::new(instance_desc);

        // We don't need a surface -- we render to an offscreen texture and
        // read back. `request_adapter` with `compatible_surface: None` picks
        // the default Metal adapter on macOS.
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .block_on()
            .map_err(|error| format!("wgpu: no Metal adapter available: {error}"))?;

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("effects-host-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                experimental_features: wgpu::ExperimentalFeatures::disabled(),
                memory_hints: wgpu::MemoryHints::Performance,
                trace: wgpu::Trace::Off,
            })
            .block_on()
            .map_err(|error| format!("wgpu: failed to acquire Metal device: {error}"))?;

        // Catch wgpu errors instead of letting wgpu's default handler panic
        // the whole app. OutOfMemory disables the effect; validation /
        // internal errors are logged but treated as non-fatal too (the host
        // stays alive, the user sees no shader output, the rest of the
        // editor keeps working).
        let disabled = Arc::new(AtomicBool::new(false));
        let disabled_for_handler = disabled.clone();
        device.on_uncaptured_error(Arc::new(move |error: wgpu::Error| {
            disabled_for_handler.store(true, Ordering::Relaxed);
            log::error!("wgpu effects disabled after error: {error}");
        }));

        // Uniform buffer: 64 bytes matching smoke.wgsl's `Uniforms`:
        //   [0..2]  resolution (vec2<f32>)
        //   [2]     time
        //   [3]     intensity
        //   [4..8]  color1 (vec4<f32>)
        //   [8..12] color2
        //   [12..16] color3
        // Persistent so we just rewrite contents per frame.
        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("effects-host-uniforms"),
            size: SMOKE_UNIFORM_BYTES as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("effects-host-bgl"),
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

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("effects-host-bg"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("effects-host-pl"),
            bind_group_layouts: &[Some(&bind_group_layout)],
            immediate_size: 0,
        });

        let shader_sources = builtin_shader_sources();
        let build_pipeline = |index: usize| {
            let (label, wgsl) = shader_sources[index];
            build_effect_pipeline(&device, &pipeline_layout, label, wgsl)
        };
        let smoke_pipeline = build_pipeline(0)?;
        let fire_pipeline = build_pipeline(1)?;
        let aurora_pipeline = build_pipeline(2)?;
        let canopy_pipeline = build_pipeline(3)?;
        let rain_pipeline = build_pipeline(4)?;

        Ok(Self {
            pixel_buffer_attrs: build_pixel_buffer_attrs(),
            _instance: instance,
            _adapter: adapter,
            device,
            queue,
            pipelines: [
                smoke_pipeline,
                fire_pipeline,
                aurora_pipeline,
                canopy_pipeline,
                rain_pipeline,
            ],
            uniform_buffer,
            bind_group,
            cache: RefCell::new(None),
            disabled,
        })
    }

    /// Render a single frame at logical dimensions `width × height` (which
    /// will be rounded up to even because NV12 requires 2-divisible
    /// dimensions for chroma subsampling), returning a CVPixelBuffer ready
    /// to hand to `Window::paint_surface`.
    ///
    /// Failures (allocator out-of-memory, lock failures, GPU readback
    /// errors) report as `None`; caller should skip the frame rather than
    /// crash.
    pub fn render_frame(
        &self,
        kind: EffectKind,
        time: f32,
        width: u32,
        height: u32,
        params: EffectParams,
    ) -> Option<CVPixelBuffer> {
        if self.disabled.load(Ordering::Relaxed) {
            return None;
        }

        match catch_unwind(AssertUnwindSafe(|| {
            self.render_frame_inner(kind, time, width, height, params)
        })) {
            Ok(frame) => frame,
            Err(payload) => {
                self.disable_after_error(format!(
                    "shader render panicked: {}",
                    panic_payload_to_string(payload)
                ));
                None
            }
        }
    }

    pub fn is_disabled(&self) -> bool {
        self.disabled.load(Ordering::Relaxed)
    }

    fn render_frame_inner(
        &self,
        kind: EffectKind,
        time: f32,
        width: u32,
        height: u32,
        params: EffectParams,
    ) -> Option<CVPixelBuffer> {
        let width_u = (width.max(2) + 1) & !1;
        let height_u = (height.max(2) + 1) & !1;
        let pixels = (width_u as u64) * (height_u as u64);
        if width_u > MAX_EFFECT_EDGE || height_u > MAX_EFFECT_EDGE || pixels > MAX_EFFECT_PIXELS {
            self.disable_after_error(format!(
                "shader frame size is too large: {width_u}x{height_u} ({pixels} px)"
            ));
            return None;
        }

        let width = width_u as usize;
        let height = height_u as usize;

        let scopes = WgpuErrorScopes::push(&self.device);

        // 1. Pack the 16 f32 uniforms (resolution + time + intensity + 3
        //    colour vec4s) and write in a single upload.
        let mut uniforms = [0.0f32; 16];
        uniforms[0] = width_u as f32;
        uniforms[1] = height_u as f32;
        uniforms[2] = time;
        uniforms[3] = params.intensity.clamp(0.0, 1.0);
        uniforms[4..8].copy_from_slice(&params.palette[0]);
        uniforms[8..12].copy_from_slice(&params.palette[1]);
        uniforms[12..16].copy_from_slice(&params.palette[2]);
        self.queue
            .write_buffer(&self.uniform_buffer, 0, bytemuck_cast(&uniforms));

        // wgpu requires the row stride in buffer copies be aligned to 256.
        let unpadded_bytes_per_row = width_u * 4; // RGBA8 = 4 bytes/pixel
        let padded_bytes_per_row = align_up(unpadded_bytes_per_row, COPY_BYTES_PER_ROW_ALIGNMENT);
        let staging_size = (padded_bytes_per_row as u64) * (height_u as u64);

        // 2. Reuse cached render target + staging buffer when dimensions
        //    match. wgpu types are Arc-backed and cheap to clone. The cache
        //    is rebuilt on resize; in steady state nothing allocates per
        //    frame here.
        let (render_texture, render_view, staging_buffer) = {
            let mut cache = self.cache.borrow_mut();
            let dimensions_match = cache
                .as_ref()
                .is_some_and(|c| c.width == width_u && c.height == height_u);
            if !dimensions_match {
                // Drop the previous entry before creating new ones so we
                // don't hold double the memory mid-replace.
                *cache = None;
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("effects-host-render"),
                    size: wgpu::Extent3d {
                        width: width_u,
                        height: height_u,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: RENDER_TEXTURE_FORMAT,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
                let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("effects-host-staging"),
                    size: staging_size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                *cache = Some(CachedRenderResources {
                    width: width_u,
                    height: height_u,
                    texture,
                    view,
                    staging,
                    mapped: false,
                });
            }
            // Defensive unmap in case a previous frame mapped but returned
            // early before reading. Without this, the next map_async would
            // fail or stall.
            if let Some(entry) = cache.as_mut() {
                if entry.mapped {
                    entry.staging.unmap();
                    entry.mapped = false;
                }
            }
            let entry = cache.as_ref().expect("cache initialized above");
            (
                entry.texture.clone(),
                entry.view.clone(),
                entry.staging.clone(),
            )
        };

        // 3. Encode the render pass + the texture->buffer copy in one
        //    submission so the GPU does them back-to-back without a CPU
        //    round-trip.
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("effects-host-encoder"),
            });
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("effects-host-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &render_view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                    depth_slice: None,
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            pass.set_pipeline(&self.pipelines[kind.pipeline_index()]);
            pass.set_bind_group(0, &self.bind_group, &[]);
            // 3 vertices, 1 instance: a single fullscreen triangle, vertex
            // positions come from `@builtin(vertex_index)` in the shader.
            pass.draw(0..3, 0..1);
        }
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: &render_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &staging_buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded_bytes_per_row),
                    rows_per_image: Some(height_u),
                },
            },
            wgpu::Extent3d {
                width: width_u,
                height: height_u,
                depth_or_array_layers: 1,
            },
        );
        self.queue.submit(Some(encoder.finish()));

        // 4. Map the staging buffer and wait for the GPU to flush. We use a
        //    blocking channel + `Device::poll` so we don't need an async
        //    runtime. On Apple Silicon the unified memory means this wait
        //    is short; we still treat a map error as a frame skip rather
        //    than a panic.
        let slice = staging_buffer.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            // Channel send failures here just mean the receiver was dropped
            // (frame skipped); nothing to do.
            let _ = tx.send(result);
        });
        // PollType::Wait makes the call block until the submitted work is
        // done, at which point our map_async callback fires.
        let map_result = match self.device.poll(wgpu::PollType::wait_indefinitely()) {
            Ok(_) => rx
                .recv()
                .map_err(|_| "shader readback callback channel closed".to_string())
                .and_then(|result| {
                    result.map_err(|error| format!("shader readback map failed: {error:?}"))
                }),
            Err(error) => Err(format!("shader GPU poll failed: {error}")),
        };
        let scoped_error = scopes.pop();
        if let Some(error) = scoped_error {
            if map_result.is_ok() {
                staging_buffer.unmap();
                self.mark_cached_mapped(false);
            }
            self.disable_after_error(format!(
                "wgpu error while rendering {kind:?} shader: {error}"
            ));
            return None;
        }
        if let Err(error) = map_result {
            self.disable_after_error(error);
            return None;
        }

        // From here on the buffer IS mapped. Any early return must unmap
        // before exit, or the pooled buffer will fail the next map_async.
        self.mark_cached_mapped(true);

        // 5. Allocate the destination CVPixelBuffer. Done after the GPU
        //    work succeeds so we don't burn an allocation if the readback
        //    fails.
        let buffer = match CVPixelBuffer::new(
            kCVPixelFormatType_420YpCbCr8BiPlanarFullRange,
            width,
            height,
            Some(&self.pixel_buffer_attrs),
        ) {
            Ok(buffer) => buffer,
            Err(error) => {
                staging_buffer.unmap();
                self.mark_cached_mapped(false);
                self.disable_after_error(format!(
                    "shader CVPixelBuffer allocation failed: {error:?}"
                ));
                return None;
            }
        };
        let lock_result = buffer.lock_base_address(0);
        if lock_result != 0 {
            staging_buffer.unmap();
            self.mark_cached_mapped(false);
            self.disable_after_error(format!(
                "shader CVPixelBuffer lock failed: CVReturn {lock_result}"
            ));
            return None;
        }

        // 6. Read the mapped staging buffer and write into the NV12 planes.
        //    Drop the view + unmap before returning so subsequent frames
        //    can reuse the buffer (we recreate per-frame for M2, but this
        //    is the contract wgpu wants).
        {
            let view = slice.get_mapped_range();
            // SAFETY: we just locked the buffer above. The two planes are
            // accessed through `get_base_address_of_plane(0)` (Y, full res)
            // and `get_base_address_of_plane(1)` (CbCr interleaved, half
            // res). Every write stays inside the strides reported by
            // `get_bytes_per_row_of_plane`.
            unsafe {
                let y_base = buffer.get_base_address_of_plane(0) as *mut u8;
                let y_stride = buffer.get_bytes_per_row_of_plane(0);
                let cbcr_base = buffer.get_base_address_of_plane(1) as *mut u8;
                let cbcr_stride = buffer.get_bytes_per_row_of_plane(1);
                rgba_to_nv12(
                    &view,
                    padded_bytes_per_row as usize,
                    Nv12Planes {
                        y_base,
                        y_stride,
                        cbcr_base,
                        cbcr_stride,
                        width,
                        height,
                    },
                );
            }
        }
        staging_buffer.unmap();
        self.mark_cached_mapped(false);
        let unlock_result = buffer.unlock_base_address(0);
        if unlock_result != 0 {
            self.disable_after_error(format!(
                "shader CVPixelBuffer unlock failed: CVReturn {unlock_result}"
            ));
            return None;
        }

        Some(buffer)
    }

    /// Update the pooled cache's `mapped` flag so the next frame's defensive
    /// unmap-on-entry knows whether the staging buffer needs to be cleared.
    fn mark_cached_mapped(&self, mapped: bool) {
        if let Some(entry) = self.cache.borrow_mut().as_mut() {
            entry.mapped = mapped;
        }
    }

    fn disable_after_error(&self, reason: impl AsRef<str>) {
        if !self.disabled.swap(true, Ordering::Relaxed) {
            log::error!("Disabling shader effects: {}", reason.as_ref());
        }
    }
}

// Thread-local singleton. `CFDictionary` inside `EffectsHost` is `!Sync`,
// so we can't use `static OnceLock`. GPUI's render path lives on the main
// thread, so a thread-local is the right shape: one EffectsHost per render
// thread, reused across every paint of every EffectsElement.
thread_local! {
    static SHARED_HOST: OnceCell<Result<EffectsHost, String>> = const { OnceCell::new() };
}
/// Process-wide animation clock. Read by `EffectsElement::paint` so every
/// instance of the element sees the same wall-clock time, which means a new
/// element constructed mid-session doesn't start its animation at t=0.
/// `Instant` is `Sync` so this can stay in a real static.
static APP_START: OnceLock<Instant> = OnceLock::new();

impl EffectsHost {
    /// Run a closure with a shared `EffectsHost` for the current thread,
    /// lazily creating it on first call. Closure-based instead of returning
    /// a reference because `OnceCell` inside `thread_local!` doesn't give us
    /// a `'static` borrow.
    pub fn with_shared<R>(f: impl FnOnce(&EffectsHost) -> R) -> Option<R> {
        SHARED_HOST.with(|cell| {
            cell.get_or_init(|| {
                let result = catch_unwind(AssertUnwindSafe(EffectsHost::try_new)).unwrap_or_else(
                    |payload| {
                        Err(format!(
                            "wgpu effects host init panicked: {}",
                            panic_payload_to_string(payload)
                        ))
                    },
                );
                if let Err(error) = &result {
                    log::warn!("Disabling shader effects: {error}");
                }
                result
            })
            .as_ref()
            .ok()
            .map(f)
        })
    }
}

/// Elapsed time since the process's animation clock started, in seconds.
/// Used as the `time` uniform for every shader frame.
pub fn app_time_seconds() -> f32 {
    APP_START.get_or_init(Instant::now).elapsed().as_secs_f32()
}

/// Build a render pipeline for one effect shader. All effects share the
/// same `pipeline_layout` (and therefore the same bind group + uniforms);
/// only the shader module + the final `RenderPipeline` are per-effect.
fn build_effect_pipeline(
    device: &wgpu::Device,
    pipeline_layout: &wgpu::PipelineLayout,
    label: &str,
    wgsl: &'static str,
) -> Result<wgpu::RenderPipeline, String> {
    let scopes = WgpuErrorScopes::push(device);
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(&format!("effects-host-shader-{label}")),
        source: wgpu::ShaderSource::Wgsl(Cow::Borrowed(wgsl)),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(&format!("effects-host-pipeline-{label}")),
        layout: Some(pipeline_layout),
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            buffers: &[],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        },
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: None,
            unclipped_depth: false,
            polygon_mode: wgpu::PolygonMode::Fill,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            targets: &[Some(wgpu::ColorTargetState {
                format: RENDER_TEXTURE_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: wgpu::PipelineCompilationOptions::default(),
        }),
        multiview_mask: None,
        cache: None,
    });
    if let Some(error) = scopes.pop() {
        return Err(format!("{label} shader pipeline failed: {error}"));
    }
    Ok(pipeline)
}

/// Pushes all three wgpu error-scope classes so validation, internal, and
/// out-of-memory failures are reported to our log path rather than surfacing
/// through the device's default panic handler.
struct WgpuErrorScopes {
    out_of_memory: wgpu::ErrorScopeGuard,
    internal: wgpu::ErrorScopeGuard,
    validation: wgpu::ErrorScopeGuard,
}

impl WgpuErrorScopes {
    fn push(device: &wgpu::Device) -> Self {
        Self {
            out_of_memory: device.push_error_scope(wgpu::ErrorFilter::OutOfMemory),
            internal: device.push_error_scope(wgpu::ErrorFilter::Internal),
            validation: device.push_error_scope(wgpu::ErrorFilter::Validation),
        }
    }

    fn pop(self) -> Option<wgpu::Error> {
        let Self {
            out_of_memory,
            internal,
            validation,
        } = self;
        let validation_error = validation.pop().block_on();
        let internal_error = internal.pop().block_on();
        let out_of_memory_error = out_of_memory.pop().block_on();
        validation_error.or(internal_error).or(out_of_memory_error)
    }
}

fn panic_payload_to_string(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_string()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "unknown panic payload".to_string()
    }
}

fn builtin_shader_sources() -> [(&'static str, &'static str); 5] {
    [
        ("smoke", include_str!("shaders/smoke.wgsl")),
        ("fire", include_str!("shaders/fire.wgsl")),
        ("aurora", include_str!("shaders/aurora.wgsl")),
        ("canopy", include_str!("shaders/canopy.wgsl")),
        ("rain", include_str!("shaders/rain.wgsl")),
    ]
}

/// Build the CFDictionary of pixel-buffer creation attributes: IOSurface
/// backing (required for `CVMetalTextureCacheCreateTextureFromImage` in
/// GPUI's blade renderer) and Metal-compatibility (so the IOSurface can be
/// wrapped as an MTLTexture without extra copies).
fn build_pixel_buffer_attrs() -> CFDictionary<CFString, CFType> {
    let io_surface_props = CFDictionary::<CFString, CFType>::from_CFType_pairs(&[]);
    // SAFETY: these CoreVideo constants are process-lifetime CFString
    // singletons. `wrap_under_get_rule` is correct because ownership remains
    // with CoreVideo and the wrapper must not release the constants.
    let io_surface_key =
        unsafe { CFString::wrap_under_get_rule(kCVPixelBufferIOSurfacePropertiesKey) };
    let metal_key = unsafe { CFString::wrap_under_get_rule(kCVPixelBufferMetalCompatibilityKey) };
    CFDictionary::from_CFType_pairs(&[
        (io_surface_key, io_surface_props.as_CFType()),
        (metal_key, CFBoolean::true_value().as_CFType()),
    ])
}

/// Convert the wgpu staging-buffer RGBA8 readback into NV12 YCbCr full-range
/// using BT.601 coefficients (the same matrix the M1 CPU path used).
///
/// `padded_row_stride` is the staging buffer's row pitch (rounded up to
/// `COPY_BYTES_PER_ROW_ALIGNMENT`); the natural row is `width * 4`.
///
/// SAFETY: caller must hold the CVPixelBuffer's base-address lock and pass
/// the buffer's actual `*_stride` (which can exceed the natural row width
/// due to plane alignment).
struct Nv12Planes {
    y_base: *mut u8,
    y_stride: usize,
    cbcr_base: *mut u8,
    cbcr_stride: usize,
    width: usize,
    height: usize,
}

unsafe fn rgba_to_nv12(rgba: &[u8], padded_row_stride: usize, planes: Nv12Planes) {
    // Y plane: one byte per source pixel.
    for y in 0..planes.height {
        let src_row = &rgba[y * padded_row_stride..y * padded_row_stride + planes.width * 4];
        let dst_row = planes.y_base.add(y * planes.y_stride);
        for x in 0..planes.width {
            let r = src_row[x * 4] as f32 / 255.0;
            let g = src_row[x * 4 + 1] as f32 / 255.0;
            let b = src_row[x * 4 + 2] as f32 / 255.0;
            *dst_row.add(x) = rgb_to_y(r, g, b);
        }
    }

    // CbCr plane: half resolution in both axes. For each chroma macropixel,
    // sample RGB at the top-left of the corresponding 2x2 source block (a
    // box filter is fine -- the test pattern is constant colour anyway).
    let cb_w = planes.width / 2;
    let cb_h = planes.height / 2;
    for cy in 0..cb_h {
        let sy = cy * 2;
        let src_row = &rgba[sy * padded_row_stride..sy * padded_row_stride + planes.width * 4];
        let dst_row = planes.cbcr_base.add(cy * planes.cbcr_stride);
        for cx in 0..cb_w {
            let sx = cx * 2;
            let r = src_row[sx * 4] as f32 / 255.0;
            let g = src_row[sx * 4 + 1] as f32 / 255.0;
            let b = src_row[sx * 4 + 2] as f32 / 255.0;
            let (cb, cr) = rgb_to_cbcr(r, g, b);
            *dst_row.add(cx * 2) = cb;
            *dst_row.add(cx * 2 + 1) = cr;
        }
    }
}

/// BT.601 full-range RGB->Y. Inputs/outputs in [0, 1] / [0, 255].
fn rgb_to_y(r: f32, g: f32, b: f32) -> u8 {
    let y = 0.299 * r + 0.587 * g + 0.114 * b;
    (y * 255.0).round().clamp(0.0, 255.0) as u8
}

/// BT.601 full-range RGB->(Cb, Cr). Inputs/outputs in [0, 1] / [0, 255].
/// Chroma is biased by +128 so neutral grey reads as (128, 128).
fn rgb_to_cbcr(r: f32, g: f32, b: f32) -> (u8, u8) {
    let cb = -0.168736 * r - 0.331264 * g + 0.5 * b;
    let cr = 0.5 * r - 0.418688 * g - 0.081312 * b;
    let cb = ((cb + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
    let cr = ((cr + 0.5) * 255.0).round().clamp(0.0, 255.0) as u8;
    (cb, cr)
}

fn align_up(value: u32, alignment: u32) -> u32 {
    value.div_ceil(alignment) * alignment
}

/// Cast a `&[f32; 16]` to the `&[u8]` that `Queue::write_buffer` wants. We
/// hand-roll this rather than pull in the `bytemuck` crate just for one
/// 64-byte upload.
fn bytemuck_cast(uniforms: &[f32; 16]) -> &[u8] {
    // SAFETY: f32 and u8 have no padding and the resulting slice is the
    // same length in bytes (16 floats * 4 bytes = 64 bytes).
    unsafe { std::slice::from_raw_parts(uniforms.as_ptr() as *const u8, SMOKE_UNIFORM_BYTES) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_effect_shaders_parse_validate_and_expose_entries() {
        for (label, wgsl) in builtin_shader_sources() {
            let module = naga::front::wgsl::parse_str(wgsl)
                .unwrap_or_else(|error| panic!("{label} WGSL parse failed: {error:?}"));

            let mut validator = naga::valid::Validator::new(
                naga::valid::ValidationFlags::all(),
                naga::valid::Capabilities::default(),
            );
            validator
                .validate(&module)
                .unwrap_or_else(|error| panic!("{label} WGSL validation failed: {error:?}"));

            let mut has_vertex_entry = false;
            let mut has_fragment_entry = false;
            for entry in &module.entry_points {
                has_vertex_entry |= entry.name == "vs_main";
                has_fragment_entry |= entry.name == "fs_main";
            }

            assert!(has_vertex_entry, "{label} shader is missing vs_main");
            assert!(has_fragment_entry, "{label} shader is missing fs_main");
        }
    }
}
