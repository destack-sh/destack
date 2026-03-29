use std::ptr;

use objc2::AnyThread;
use objc2::rc::Retained;
use objc2_app_kit::{NSBitmapFormat, NSBitmapImageRep, NSDeviceRGBColorSpace, NSImage};
use objc2_foundation::NSSize;

use crate::diagnostic::RuntimeResult;
use crate::platform;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::display::{WindowIconPixelFormat, WindowIconSet};

/// Preferred AppKit window icon target dimension.
const WINDOW_ICON_TARGET_DIMENSION: u32 = 64;

/// One decoded AppKit icon image in RGBA8 order.
#[derive(Debug, Clone)]
pub(crate) struct DecodedWindowIconImage {
    /// Icon width in pixels.
    width: u32,
    /// Icon height in pixels.
    height: u32,
    /// Packed RGBA8 bytes in row-major order.
    pixels_rgba: Vec<u8>,
}

/// Return the index that best matches the preferred AppKit icon target size.
fn best_icon_index(images: &[DecodedWindowIconImage]) -> usize {
    let mut best_index = 0usize;
    let mut best_score = u32::MAX;

    // choose the image closest to the standard task-switcher size
    for (index, image) in images.iter().enumerate() {
        let score = image.width.abs_diff(WINDOW_ICON_TARGET_DIMENSION)
            + image.height.abs_diff(WINDOW_ICON_TARGET_DIMENSION);
        if score < best_score {
            best_score = score;
            best_index = index;
        }
    }

    best_index
}

/// Decode one icon-set payload into validated RGBA image payloads.
fn decode_window_icons(icons: WindowIconSet) -> RuntimeResult<Vec<DecodedWindowIconImage>> {
    let images = unsafe { icons.images.as_slice()? };
    // reject empty icon sets
    if images.is_empty() {
        return Err(platform::core::invalid_argument(
            "icons",
            "icon set must contain at least one image",
        ));
    }

    let mut decoded = Vec::with_capacity(images.len());

    // decode every image payload into RGBA8 bytes
    for image in images {
        if image.width == 0 || image.height == 0 {
            return Err(platform::core::invalid_argument(
                "icons",
                "icon image width and height must be greater than zero",
            ));
        }

        let width = platform::core::u32_to_usize(image.width);
        let height = platform::core::u32_to_usize(image.height);
        let pixel_count = width.checked_mul(height).ok_or_else(|| {
            platform::core::invalid_argument("icons", "icon dimensions are too large")
        })?;
        let expected_length = pixel_count.checked_mul(4).ok_or_else(|| {
            platform::core::invalid_argument("icons", "icon pixel payload is too large")
        })?;
        let pixels = unsafe { image.pixels.as_slice()? };

        if pixels.len() != expected_length {
            return Err(platform::core::invalid_argument(
                "icons",
                format!(
                    "icon pixel length {} does not match expected {expected_length}",
                    pixels.len()
                ),
            ));
        }

        let mut pixels_rgba = Vec::with_capacity(expected_length);

        // normalize all icon pixels into RGBA8
        for pixel in pixels.chunks_exact(4) {
            match image.pixel_format {
                WindowIconPixelFormat::Rgba8 => pixels_rgba.extend_from_slice(pixel),
                WindowIconPixelFormat::Bgra8 => {
                    pixels_rgba.push(pixel[2]);
                    pixels_rgba.push(pixel[1]);
                    pixels_rgba.push(pixel[0]);
                    pixels_rgba.push(pixel[3]);
                }
            }
        }

        decoded.push(DecodedWindowIconImage {
            width: image.width,
            height: image.height,
            pixels_rgba,
        });
    }

    Ok(decoded)
}

/// Build one bitmap representation from one decoded RGBA8 icon payload.
fn bitmap_representation_from_icon(
    image: &DecodedWindowIconImage,
) -> RuntimeResult<Retained<NSBitmapImageRep>> {
    let width = platform::core::u32_to_isize("icons", image.width)?;
    let height = platform::core::u32_to_isize("icons", image.height)?;
    let bytes_per_row = image
        .width
        .checked_mul(4)
        .ok_or_else(|| platform::core::invalid_argument("icons", "icon row stride is too large"))?;
    let bytes_per_row = platform::core::u32_to_isize("icons", bytes_per_row)?;

    let bitmap_rep = unsafe {
        NSBitmapImageRep::initWithBitmapDataPlanes_pixelsWide_pixelsHigh_bitsPerSample_samplesPerPixel_hasAlpha_isPlanar_colorSpaceName_bitmapFormat_bytesPerRow_bitsPerPixel(
            NSBitmapImageRep::alloc(),
            ptr::null_mut(),
            width,
            height,
            8,
            4,
            true,
            false,
            NSDeviceRGBColorSpace,
            NSBitmapFormat::AlphaNonpremultiplied,
            bytes_per_row,
            32,
        )
    }
    .ok_or_else(|| platform::core::not_supported("destack.display.window.setIcons"))?;

    // require a writable bitmap buffer from AppKit
    let bitmap_data = bitmap_rep.bitmapData();
    if bitmap_data.is_null() {
        return Err(platform::core::io_operation_error(
            "destack.display.window.setIcons",
            Some(PlatformErrorCode::IoInvalidData),
            "AppKit returned one null icon bitmap buffer",
        ));
    }

    // copy the normalized RGBA8 bytes into the bitmap backing storage
    unsafe {
        ptr::copy_nonoverlapping(
            image.pixels_rgba.as_ptr(),
            bitmap_data,
            image.pixels_rgba.len(),
        );
    }

    Ok(bitmap_rep)
}

/// Decode one optional icon-set payload into normalized AppKit icon images.
pub(crate) fn decode_window_icon_images(
    icons: Option<WindowIconSet>,
) -> RuntimeResult<Option<Vec<DecodedWindowIconImage>>> {
    let Some(icons) = icons else {
        return Ok(None);
    };

    decode_window_icons(icons).map(Some)
}

/// Build one retained AppKit image from normalized icon payloads.
pub(crate) fn window_icon_image(
    decoded_images: Option<&[DecodedWindowIconImage]>,
) -> RuntimeResult<Option<Retained<NSImage>>> {
    let Some(decoded_images) = decoded_images else {
        return Ok(None);
    };

    let best_index = best_icon_index(decoded_images);
    let best_image = &decoded_images[best_index];
    let window_image = NSImage::initWithSize(
        NSImage::alloc(),
        NSSize::new(best_image.width as f64, best_image.height as f64),
    );

    // attach all decoded image representations so AppKit can choose the best one
    for image in decoded_images {
        let bitmap_representation = bitmap_representation_from_icon(image)?;
        window_image.addRepresentation(&bitmap_representation);
    }

    Ok(Some(window_image))
}

#[cfg(test)]
mod tests {
    use crate::platform::abi::NativeSlice;
    use crate::platform::display::{WindowIconImage, WindowIconPixelFormat, WindowIconSet};

    use super::{best_icon_index, decode_window_icons};

    /// Decode BGRA icon payloads into RGBA bytes.
    #[test]
    fn test_decode_window_icons_normalizes_bgra_pixels() {
        let mut pixels = vec![0x33, 0x22, 0x11, 0x44];
        let image = WindowIconImage {
            width: 1,
            height: 1,
            pixel_format: WindowIconPixelFormat::Bgra8,
            pixels: NativeSlice {
                data: pixels.as_mut_ptr(),
                len: pixels.len() as u32,
            },
        };
        let mut images = [image];
        let icon_set = WindowIconSet {
            images: NativeSlice {
                data: images.as_mut_ptr(),
                len: images.len() as u32,
            },
        };

        let decoded = decode_window_icons(icon_set).expect("icon payload should decode");
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].pixels_rgba, vec![0x11, 0x22, 0x33, 0x44]);
    }

    /// Prefer icon images closest to the standard AppKit target size.
    #[test]
    fn test_best_icon_index_prefers_64px_representation() {
        let images = vec![
            super::DecodedWindowIconImage {
                width: 16,
                height: 16,
                pixels_rgba: Vec::new(),
            },
            super::DecodedWindowIconImage {
                width: 64,
                height: 64,
                pixels_rgba: Vec::new(),
            },
            super::DecodedWindowIconImage {
                width: 128,
                height: 128,
                pixels_rgba: Vec::new(),
            },
        ];

        assert_eq!(best_icon_index(images.as_slice()), 1);
    }
}
