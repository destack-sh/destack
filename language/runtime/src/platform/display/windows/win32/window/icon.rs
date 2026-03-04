use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIcon, DestroyIcon, GetSystemMetrics, SM_CXICON, SM_CXSMICON, SM_CYICON, SM_CYSMICON,
};

use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowIconPixelFormat, WindowIconSet};

use super::{WINDOW_ICON_BITS_PER_PIXEL, WINDOW_ICON_COLOR_PLANES, core, dimension_to_i32};

/// Decoded icon-image payload normalized to BGRA8 bytes.
#[derive(Debug, Clone)]
pub(super) struct DecodedWindowIconImage {
    /// Icon width in pixels.
    pub(super) width: u32,
    /// Icon height in pixels.
    pub(super) height: u32,
    /// Packed BGRA8 icon pixel bytes.
    pub(super) pixels_bgra: Vec<u8>,
}

/// Resolve one icon-system metric with one stable fallback.
fn icon_metric(metric: i32, fallback: u32) -> u32 {
    let value = unsafe { GetSystemMetrics(metric) };
    // evaluate this condition
    if value <= 0 {
        return fallback;
    }

    value as u32
}

/// Return target icon dimensions for small and big icon lanes.
pub(super) fn icon_target_dimensions(small_default: u32, big_default: u32) -> (u32, u32, u32, u32) {
    let small_width = icon_metric(SM_CXSMICON, small_default);
    let small_height = icon_metric(SM_CYSMICON, small_default);
    let big_width = icon_metric(SM_CXICON, big_default);
    let big_height = icon_metric(SM_CYICON, big_default);
    (small_width, small_height, big_width, big_height)
}

/// Decode one window-icon set into validated BGRA payloads.
pub(super) fn decode_window_icons(
    icons: WindowIconSet,
) -> RuntimeResult<Vec<DecodedWindowIconImage>> {
    let images = unsafe { icons.images.as_slice()? };
    // evaluate this condition
    if images.is_empty() {
        return Err(core_platform::invalid_argument(
            "icons",
            "icon set must contain at least one image",
        ));
    }

    let mut decoded = Vec::with_capacity(images.len());
    // iterate this sequence
    for image in images {
        // evaluate this condition
        if image.width == 0 || image.height == 0 {
            return Err(core_platform::invalid_argument(
                "icons",
                "icon image width and height must be greater than zero",
            ));
        }

        let pixel_count = (image.width as usize)
            .checked_mul(image.height as usize)
            .ok_or_else(|| {
                core_platform::invalid_argument("icons", "icon dimensions are too large")
            })?;
        let expected_length = pixel_count.checked_mul(4).ok_or_else(|| {
            core_platform::invalid_argument("icons", "icon pixel payload is too large")
        })?;

        let pixels = unsafe { image.pixels.as_slice()? };
        // evaluate this condition
        if pixels.len() != expected_length {
            return Err(core_platform::invalid_argument(
                "icons",
                format!(
                    "icon pixel length {} does not match expected {expected_length}",
                    pixels.len()
                ),
            ));
        }

        let row_bytes = (image.width as usize) * 4;
        let height = image.height as usize;
        let mut pixels_bgra = vec![0u8; expected_length];
        // iterate this sequence
        for row in 0..height {
            let source_row = (height - 1 - row) * row_bytes;
            let target_row = row * row_bytes;

            let source = &pixels[source_row..source_row + row_bytes];
            let target = &mut pixels_bgra[target_row..target_row + row_bytes];

            // resolve this variant
            match image.pixel_format {
                WindowIconPixelFormat::Bgra8 => target.copy_from_slice(source),
                WindowIconPixelFormat::Rgba8 => {
                    // iterate this sequence
                    for (source_pixel, target_pixel) in
                        source.chunks_exact(4).zip(target.chunks_exact_mut(4))
                    {
                        target_pixel[0] = source_pixel[2];
                        target_pixel[1] = source_pixel[1];
                        target_pixel[2] = source_pixel[0];
                        target_pixel[3] = source_pixel[3];
                    }
                }
            }
        }

        decoded.push(DecodedWindowIconImage {
            width: image.width,
            height: image.height,
            pixels_bgra,
        });
    }

    Ok(decoded)
}

/// Select one icon image that best matches one target dimension.
pub(super) fn best_icon_index(
    images: &[DecodedWindowIconImage],
    target_width: u32,
    target_height: u32,
) -> usize {
    let mut best_index = 0usize;
    let mut best_score = u32::MAX;

    // iterate this sequence
    for (index, image) in images.iter().enumerate() {
        let score = image.width.abs_diff(target_width) + image.height.abs_diff(target_height);
        // evaluate this condition
        if score < best_score {
            best_score = score;
            best_index = index;
        }
    }

    best_index
}

/// Create one Win32 `HICON` handle from one decoded BGRA icon payload.
pub(super) fn create_hicon(
    image: &DecodedWindowIconImage,
    operation: &'static str,
) -> RuntimeResult<isize> {
    let width = dimension_to_i32(image.width, "icons.width")?;
    let height = dimension_to_i32(image.height, "icons.height")?;

    let and_row_bytes = (image.width as usize).div_ceil(32) * 4;
    let and_bytes_length = and_row_bytes
        .checked_mul(image.height as usize)
        .ok_or_else(|| {
            core_platform::invalid_argument("icons", "icon mask payload is too large")
        })?;
    let and_mask = vec![0u8; and_bytes_length];

    let icon = unsafe {
        CreateIcon(
            0,
            width,
            height,
            WINDOW_ICON_COLOR_PLANES,
            WINDOW_ICON_BITS_PER_PIXEL,
            and_mask.as_ptr(),
            image.pixels_bgra.as_ptr(),
        )
    };
    // evaluate this condition
    if icon == 0 {
        return Err(core::io_error(
            operation,
            "CreateIcon",
            "failed to create window icon",
        ));
    }

    Ok(icon)
}

/// Destroy one owned icon handle when present.
fn destroy_owned_icon(icon: isize) {
    // evaluate this condition
    if icon == 0 {
        return;
    }

    unsafe {
        let _ = DestroyIcon(icon);
    }
}

/// Destroy one pair of icon handles without double free.
pub(super) fn destroy_owned_icons(small_icon: isize, big_icon: isize) {
    // evaluate this condition
    if small_icon != 0 {
        destroy_owned_icon(small_icon);
    }

    // evaluate this condition
    if big_icon != 0 && big_icon != small_icon {
        destroy_owned_icon(big_icon);
    }
}
