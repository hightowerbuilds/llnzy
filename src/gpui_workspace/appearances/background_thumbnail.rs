//! Small, asynchronously decoded previews for the saved background library.
use std::{path::PathBuf, sync::Arc, time::SystemTime};

use gpui::{
    div, img, prelude::*, px, rgb, App, Asset, ImageCacheError, ObjectFit, RenderImage,
    StyledImage, Window,
};

use crate::ui_theme::{Typography, UiTheme};

const WIDTH: u32 = 112;
const HEIGHT: u32 = 72;

#[derive(Clone, Hash)]
struct ThumbnailSource {
    path: PathBuf,
    modified: Option<SystemTime>,
    bytes: u64,
}

struct BackgroundThumbnail;

impl Asset for BackgroundThumbnail {
    type Source = ThumbnailSource;
    type Output = Result<Arc<RenderImage>, ImageCacheError>;

    fn load(
        source: Self::Source,
        _cx: &mut App,
    ) -> impl std::future::Future<Output = Self::Output> + Send + 'static {
        // The background task owns only the path, never the UI's App reference.
        let path = source.path;
        async move {
            crate::theme_store::validate_background_image(&path)
                .map_err(|error| ImageCacheError::Asset(error.into()))?;
            let decoded = image::open(&path)?;
            Ok(Arc::new(thumbnail_image(decoded)))
        }
    }
}

fn thumbnail_image(decoded: image::DynamicImage) -> RenderImage {
    // Retain just a static preview at 2x display resolution, never a full-size
    // image or all the animation frames. GPUI expects BGRA pixels.
    let mut pixels = decoded.thumbnail(WIDTH * 2, HEIGHT * 2).into_rgba8();
    for pixel in pixels.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    RenderImage::new(vec![image::Frame::new(pixels)])
}

pub(super) fn background_thumbnail(
    path: PathBuf,
    active: bool,
    palette: UiTheme,
) -> impl IntoElement {
    let metadata = std::fs::metadata(&path).ok();
    let source = ThumbnailSource {
        path,
        modified: metadata
            .as_ref()
            .and_then(|metadata| metadata.modified().ok()),
        bytes: metadata.map_or(0, |metadata| metadata.len()),
    };
    div()
        .w(px(WIDTH as f32))
        .h(px(HEIGHT as f32))
        .flex_none()
        .rounded_sm()
        .overflow_hidden()
        .border_1()
        .border_color(rgb(if active {
            palette.accent
        } else {
            palette.border
        }))
        .bg(rgb(palette.panel_bg))
        .child(
            img(move |window: &mut Window, cx: &mut App| {
                window.use_asset::<BackgroundThumbnail>(&source, cx)
            })
            .size_full()
            .object_fit(ObjectFit::Contain)
            .with_loading(move || placeholder("Loading…", palette))
            .with_fallback(move || placeholder("No preview", palette)),
        )
}

fn placeholder(label: &'static str, palette: UiTheme) -> gpui::AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .text_size(px(Typography::CAPTION))
        .text_color(rgb(palette.muted_text))
        .child(label)
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preview_preserves_aspect_ratio_and_bounds_large_images() {
        for (width, height, expected) in [(1600, 800, (224, 112)), (800, 1600, (72, 144))] {
            let preview = thumbnail_image(image::DynamicImage::new_rgb8(width, height));
            let size = preview.size(0);
            assert_eq!((size.width.0, size.height.0), expected);
            assert_eq!(preview.frame_count(), 1);
        }
    }

    #[test]
    fn preview_converts_red_and_blue_channels_for_gpui_and_preserves_alpha() {
        let pixels = image::RgbaImage::from_pixel(1, 1, image::Rgba([240, 100, 20, 128]));
        let preview = thumbnail_image(image::DynamicImage::ImageRgba8(pixels));
        assert_eq!(&preview.as_bytes(0).unwrap()[..4], &[20, 100, 240, 128]);
    }
}
