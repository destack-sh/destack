use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowIconPixelFormat, WindowIconSet};

/// Preferred icon dimension for compositor task-switcher surfaces.
const WAYLAND_ICON_TARGET_DIMENSION: u32 = 64;

/// Prepared icon buffer payload for wl_shm icon uploads.
pub(crate) struct WaylandIconBuffer {
    /// Icon width in pixels.
    pub(crate) width: i32,
    /// Icon height in pixels.
    pub(crate) height: i32,
    /// Icon row stride in bytes.
    pub(crate) stride: i32,
    /// Packed ARGB8888 pixel bytes.
    pub(crate) pixels_argb8888: Vec<u8>,
}

/// Validate one optional icon-set payload.
pub(crate) fn validate_icon_set(icons: Option<WindowIconSet>) -> RuntimeResult<()> {
    let Some(icons) = icons else {
        return Ok(());
    };

    // decode icon image payloads
    let images = unsafe { icons.images.as_slice()? };
    if images.is_empty() {
        return Err(core_platform::invalid_argument(
            "icons",
            "icon set must contain at least one image",
        ));
    }

    // validate each icon image payload
    for image in images {
        if image.width == 0 || image.height == 0 {
            return Err(core_platform::invalid_argument(
                "icons",
                "icon image width and height must be greater than zero",
            ));
        }

        let width = core_platform::u32_to_usize(image.width);
        let height = core_platform::u32_to_usize(image.height);
        let pixel_count = width.checked_mul(height).ok_or_else(|| {
            core_platform::invalid_argument("icons", "icon dimensions are too large")
        })?;
        let expected = pixel_count.checked_mul(4).ok_or_else(|| {
            core_platform::invalid_argument("icons", "icon payload length is too large")
        })?;
        let pixels = unsafe { image.pixels.as_slice()? };

        if pixels.len() != expected {
            return Err(core_platform::invalid_argument(
                "icons",
                format!(
                    "icon pixel length {} does not match expected {expected}",
                    pixels.len()
                ),
            ));
        }
    }

    Ok(())
}

/// Decode one optional icon payload into one upload-ready ARGB8888 buffer.
pub(crate) fn decode_icon_buffer(
    icons: Option<WindowIconSet>,
) -> RuntimeResult<Option<WaylandIconBuffer>> {
    let Some(icons) = icons else {
        return Ok(None);
    };

    // decode icon image payloads after validation
    let images = unsafe { icons.images.as_slice()? };
    let selected = images.iter().min_by_key(|image| {
        let width_delta = image.width.abs_diff(WAYLAND_ICON_TARGET_DIMENSION);
        let height_delta = image.height.abs_diff(WAYLAND_ICON_TARGET_DIMENSION);
        let target_score = (width_delta as u64).saturating_add(height_delta as u64);
        let area = (image.width as u64).saturating_mul(image.height as u64);
        (target_score, u64::MAX.saturating_sub(area))
    });
    let selected = selected.ok_or_else(|| {
        core_platform::invalid_argument("icons", "icon set must contain at least one image")
    })?;

    // normalize selected dimensions for wl_shm upload
    let width = selected.width.min(i32::MAX as u32) as i32;
    let height = selected.height.min(i32::MAX as u32) as i32;
    let stride = width
        .checked_mul(4)
        .ok_or_else(|| core_platform::invalid_argument("icons", "icon stride is too large"))?;
    let pixels = unsafe { selected.pixels.as_slice()? };
    let mut pixels_argb8888 = Vec::with_capacity(pixels.len());

    // convert source pixel payload to wl_shm ARGB8888 bytes
    for chunk in pixels.chunks_exact(4) {
        let (red, green, blue, alpha) = match selected.pixel_format {
            WindowIconPixelFormat::Rgba8 => (chunk[0], chunk[1], chunk[2], chunk[3]),
            WindowIconPixelFormat::Bgra8 => (chunk[2], chunk[1], chunk[0], chunk[3]),
        };

        pixels_argb8888.push(blue);
        pixels_argb8888.push(green);
        pixels_argb8888.push(red);
        pixels_argb8888.push(alpha);
    }

    Ok(Some(WaylandIconBuffer {
        width,
        height,
        stride,
        pixels_argb8888,
    }))
}
