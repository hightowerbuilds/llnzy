//! Custom shader / per-pixel effect pipeline.
//!
//! GPUI 0.2.2's `Scene::Primitive` enum is closed -- the only macOS-public
//! way to put arbitrary per-pixel content under GPUI's text is
//! `Window::paint_surface(bounds, image_buffer: CVPixelBuffer)`. This module
//! owns the bridge between our render pipeline and that hook.
//!
//! ## M1 status (2026-05-14)
//!
//! M1 lands the CVPixelBuffer + `paint_surface` end of the bridge as a
//! CPU-filled gradient test pattern, so the harness can be verified end-
//! to-end before introducing the wgpu device + WGSL pipeline (M2). The
//! `EffectsHost::render_frame` signature is what M2 will swap out for a
//! real GPU render pass.

#[cfg(target_os = "macos")]
mod element;
#[cfg(target_os = "macos")]
mod host;

#[cfg(target_os = "macos")]
pub use element::EffectsElement;
#[cfg(target_os = "macos")]
pub use host::{EffectKind, EffectParams, EffectsHost};
