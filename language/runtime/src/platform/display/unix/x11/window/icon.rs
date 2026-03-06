use crate::diagnostic::RuntimeResult;
use crate::platform::core as core_platform;
use crate::platform::display::{WindowIconPixelFormat, WindowIconSet};

/// Decode one icon set into `_NET_WM_ICON` cardinals.
pub(crate) fn net_wm_icon_payload(icons: WindowIconSet) -> RuntimeResult<Vec<u32>> {
    // decode the image slice and require at least one icon image
    let images = unsafe { icons.images.as_slice()? };
    // evaluate this condition
    if images.is_empty() {
        return Err(core_platform::invalid_argument(
            "icons",
            "icon set must contain at least one image",
        ));
    }

    // estimate output size and reserve once
    let mut total_words = 0usize;
    // iterate this sequence
    for image in images {
        // evaluate this condition
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
        total_words = total_words
            .checked_add(pixel_count + 2)
            .ok_or_else(|| core_platform::invalid_argument("icons", "icon payload is too large"))?;
    }
    let mut payload = Vec::with_capacity(total_words);

    // encode each icon image to width, height, and packed argb words
    for image in images {
        let width = core_platform::u32_to_usize(image.width);
        let height = core_platform::u32_to_usize(image.height);
        let pixel_count = width.checked_mul(height).ok_or_else(|| {
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

        payload.push(image.width);
        payload.push(image.height);
        // iterate this sequence
        for pixel in pixels.chunks_exact(4) {
            let (red, green, blue, alpha) = match image.pixel_format {
                WindowIconPixelFormat::Rgba8 => (pixel[0], pixel[1], pixel[2], pixel[3]),
                WindowIconPixelFormat::Bgra8 => (pixel[2], pixel[1], pixel[0], pixel[3]),
            };
            let argb = ((alpha as u32) << 24)
                | ((red as u32) << 16)
                | ((green as u32) << 8)
                | (blue as u32);
            payload.push(argb);
        }
    }

    Ok(payload)
}
