use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use super::abi::{
    AsioChannelInfo, AsioDriverInfo, asio_activate_driver, asio_driver_can_sample_rate,
    asio_driver_get_buffer_size, asio_driver_get_channel_info, asio_driver_get_channels,
    asio_driver_get_error_message, asio_driver_get_name, asio_driver_get_sample_rate,
    asio_driver_init,
};
use super::constants::{
    ASE_OK, ASIO_FALSE, ASIO_MAX_DRIVER_NAME_BYTES, ASIO_MAX_ERROR_MESSAGE_BYTES,
    ASIO_MAX_PROBED_CHANNELS, ASIO_PROBED_SAMPLE_RATES, ASIO_REGISTRY_PATH, ASIO_ST_FLOAT32_LSB,
    ASIO_ST_FLOAT32_MSB, ASIO_ST_FLOAT64_LSB, ASIO_ST_FLOAT64_MSB, ASIO_ST_INT16_LSB,
    ASIO_ST_INT16_MSB, ASIO_ST_INT24_LSB, ASIO_ST_INT24_MSB, ASIO_ST_INT32_LSB,
    ASIO_ST_INT32_LSB24, ASIO_ST_INT32_MSB, ASIO_ST_INT32_MSB24, ASIO_TRUE,
};
use super::core::{
    AsioChannelDescriptor, AsioDeviceProfile, AsioDriverInterface, AsioDriverRow,
    AsioSampleEncoding, AsioSession, ComApartment, asio_error, initialize_com_apartment, succeeded,
};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};

use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::System::Com::CLSIDFromString;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WOW64_32KEY, KEY_WOW64_64KEY, REG_SZ, RRF_RT_REG_SZ,
    RegCloseKey, RegEnumKeyExW, RegGetValueW, RegOpenKeyExW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
use windows_sys::core::GUID;

/// Enumerate registered ASIO drivers from both 32-bit and 64-bit registry views.
pub(super) fn enumerate_registered_drivers() -> Vec<AsioDriverRow> {
    let mut rows = BTreeMap::new();

    // scan one 64-bit registry view when available
    append_registry_view_rows(KEY_READ | KEY_WOW64_64KEY, &mut rows);

    // scan one 32-bit registry view for legacy drivers on 64-bit hosts
    append_registry_view_rows(KEY_READ | KEY_WOW64_32KEY, &mut rows);

    rows.into_values().collect()
}

/// Append ASIO registry rows from one specific view.
fn append_registry_view_rows(view_flags: u32, rows: &mut BTreeMap<String, AsioDriverRow>) {
    let mut root = 0 as HKEY;

    // open one ASIO root key for this registry view
    let path = core_platform::wide_with_nul(ASIO_REGISTRY_PATH);
    let open_status =
        unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, path.as_ptr(), 0, view_flags, &mut root) };
    if open_status != 0 || root == 0 {
        return;
    }

    let mut index = 0u32;
    loop {
        let mut name = [0u16; 512];
        let mut name_len = (name.len().saturating_sub(1)) as u32;

        // enumerate one subkey name by numeric index
        let status = unsafe {
            RegEnumKeyExW(
                root,
                index,
                name.as_mut_ptr(),
                &mut name_len,
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                std::ptr::null_mut(),
            )
        };
        if status != 0 {
            break;
        }

        let key_name = String::from_utf16_lossy(&name[..name_len as usize]);
        if key_name.is_empty() {
            index = index.saturating_add(1);
            continue;
        }

        // resolve one CLSID string payload for this driver row
        let class_id_text = read_registry_string(root, &key_name, "CLSID");
        let Some(class_id_text) = class_id_text else {
            index = index.saturating_add(1);
            continue;
        };

        let class_id = parse_guid(&class_id_text);
        let Some(class_id) = class_id else {
            index = index.saturating_add(1);
            continue;
        };

        // resolve one display string preferring Description over key name
        let display_name = read_registry_string(root, &key_name, "Description")
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| key_name.clone());

        rows.entry(key_name.clone()).or_insert(AsioDriverRow {
            key_name,
            display_name,
            class_id,
        });

        index = index.saturating_add(1);
    }

    // close one opened registry root handle
    unsafe {
        RegCloseKey(root);
    }
}

/// Read one UTF-16 registry string value.
fn read_registry_string(root: HKEY, subkey: &str, value_name: &str) -> Option<String> {
    let subkey = core_platform::wide_with_nul(subkey);
    let value_name = core_platform::wide_with_nul(value_name);

    let mut data_type = REG_SZ;
    let mut bytes = vec![0u8; 2048];
    let mut byte_len = bytes.len() as u32;

    // query one string payload from the selected key and value
    let status = unsafe {
        RegGetValueW(
            root,
            subkey.as_ptr(),
            value_name.as_ptr(),
            RRF_RT_REG_SZ,
            &mut data_type,
            bytes.as_mut_ptr() as *mut c_void,
            &mut byte_len,
        )
    };
    if status != 0 || byte_len < 2 {
        return None;
    }

    let char_len = (byte_len as usize) / 2;
    let words = unsafe { std::slice::from_raw_parts(bytes.as_ptr() as *const u16, char_len) };

    let nul_index = words
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(words.len());
    Some(String::from_utf16_lossy(&words[..nul_index]))
}

/// Parse one GUID string payload from the registry.
fn parse_guid(text: &str) -> Option<GUID> {
    let mut guid = GUID::from_u128(0);
    let wide = core_platform::wide_with_nul(text);

    // parse one COM class-id string into one GUID payload
    let status = unsafe { CLSIDFromString(wide.as_ptr(), &mut guid) };
    if !succeeded(status) {
        return None;
    }

    Some(guid)
}

/// Open and initialize one ASIO session for one registry row.
pub(super) fn open_session(row: &AsioDriverRow) -> RuntimeResult<(ComApartment, Arc<AsioSession>)> {
    let com = initialize_com_apartment()?;

    // activate one IASIO interface for this driver CLSID
    let driver_pointer = unsafe { asio_activate_driver(&row.class_id) }.map_err(|status| {
        RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("destack.audio.internal.asio.activate".to_string()),
            None,
            format!(
                "failed to activate ASIO driver {} (hresult 0x{status:08x})",
                row.display_name,
            ),
        ))
        .boxed()
    })?;

    let driver = Arc::new(AsioDriverInterface {
        raw: driver_pointer,
    });

    let mut driver_info = AsioDriverInfo {
        asio_version: 2,
        driver_version: 0,
        name: [0; ASIO_MAX_DRIVER_NAME_BYTES],
        error_message: [0; ASIO_MAX_ERROR_MESSAGE_BYTES],
        sys_ref: unsafe { GetForegroundWindow() as HWND as *mut c_void },
    };

    // initialize one ASIO driver session
    let init_status = unsafe { asio_driver_init(driver.raw, &mut driver_info) };
    if init_status != ASIO_TRUE {
        let mut error_message = [0i8; ASIO_MAX_ERROR_MESSAGE_BYTES];
        unsafe {
            asio_driver_get_error_message(driver.raw, error_message.as_mut_ptr());
        }
        let message = c_string_lossy(&error_message)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| "driver returned no init error message".to_string());

        return Err(asio_error(
            "destack.audio.internal.asio.init",
            init_status,
            &row.display_name,
            format!("failed to initialize ASIO driver: {message}"),
        ));
    }

    let mut driver_name_bytes = [0i8; ASIO_MAX_DRIVER_NAME_BYTES];
    unsafe {
        asio_driver_get_name(driver.raw, driver_name_bytes.as_mut_ptr());
    }
    let driver_name = c_string_lossy(&driver_name_bytes)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| row.display_name.clone());

    let session = Arc::new(AsioSession {
        driver,
        driver_name,
        buffers_created: AtomicBool::new(false),
        initialized: AtomicBool::new(true),
    });

    Ok((com, session))
}

/// Probe one ASIO device profile from one registry row.
pub(super) fn probe_device_profile(row: &AsioDriverRow) -> AsioDeviceProfile {
    let fallback = AsioDeviceProfile::fallback();

    let (com, session) = match open_session(row) {
        Ok(value) => value,
        Err(_) => return fallback,
    };

    let _com = com;

    let mut input_channels = 0i32;
    let mut output_channels = 0i32;

    // query one channel-count pair from the driver
    let channels_status = unsafe {
        asio_driver_get_channels(
            session.driver.raw,
            &mut input_channels,
            &mut output_channels,
        )
    };
    if channels_status != ASE_OK {
        return fallback;
    }

    let input_channels = input_channels.clamp(0, ASIO_MAX_PROBED_CHANNELS as i32) as u16;
    let output_channels = output_channels.clamp(0, ASIO_MAX_PROBED_CHANNELS as i32) as u16;
    if input_channels == 0 && output_channels == 0 {
        return fallback;
    }

    let mut current_sample_rate = 0.0f64;
    let sample_rate_status =
        unsafe { asio_driver_get_sample_rate(session.driver.raw, &mut current_sample_rate) };
    let preferred_sample_rate = if sample_rate_status == ASE_OK && current_sample_rate.is_finite() {
        current_sample_rate.round().clamp(1.0, u32::MAX as f64) as u32
    } else {
        fallback.preferred_sample_rate
    };

    let mut min_sample_rate = u32::MAX;
    let mut max_sample_rate = 0u32;

    // probe one stable sample-rate set
    for sample_rate in ASIO_PROBED_SAMPLE_RATES {
        let status = unsafe { asio_driver_can_sample_rate(session.driver.raw, sample_rate as f64) };
        if status == ASE_OK {
            min_sample_rate = min_sample_rate.min(sample_rate);
            max_sample_rate = max_sample_rate.max(sample_rate);
        }
    }

    if min_sample_rate == u32::MAX {
        min_sample_rate = preferred_sample_rate;
    }
    if max_sample_rate == 0 {
        max_sample_rate = preferred_sample_rate;
    }

    let mut minimum_period = 0i32;
    let mut maximum_period = 0i32;
    let mut preferred_period = 0i32;
    let mut granularity = 0i32;

    // query one ASIO period range tuple
    let buffer_status = unsafe {
        asio_driver_get_buffer_size(
            session.driver.raw,
            &mut minimum_period,
            &mut maximum_period,
            &mut preferred_period,
            &mut granularity,
        )
    };
    if buffer_status != ASE_OK {
        return AsioDeviceProfile {
            input_channels,
            output_channels,
            preferred_sample_rate,
            min_sample_rate,
            max_sample_rate,
            preferred_period_frames: fallback.preferred_period_frames,
            min_period_frames: fallback.min_period_frames,
            max_period_frames: fallback.max_period_frames,
            format_mask: fallback.format_mask,
        };
    }

    let minimum_period = minimum_period.max(audio_core::MIN_STREAM_PERIOD_FRAMES as i32) as u32;
    let maximum_period = maximum_period.max(minimum_period as i32) as u32;
    let preferred_period = preferred_period
        .max(minimum_period as i32)
        .min(maximum_period as i32) as u32;

    let mut format_mask = 0u32;

    // probe one representative output lane sample type
    if output_channels > 0
        && let Ok(channel) = query_channel_descriptor(&session, false, 0)
    {
        format_mask |= audio_core::sample_format_bit(channel.encoding.format);
    }

    // probe one representative input lane sample type
    if input_channels > 0
        && let Ok(channel) = query_channel_descriptor(&session, true, 0)
    {
        format_mask |= audio_core::sample_format_bit(channel.encoding.format);
    }

    if format_mask == 0 {
        format_mask = fallback.format_mask;
    }

    AsioDeviceProfile {
        input_channels,
        output_channels,
        preferred_sample_rate,
        min_sample_rate,
        max_sample_rate,
        preferred_period_frames: preferred_period,
        min_period_frames: minimum_period,
        max_period_frames: maximum_period,
        format_mask,
    }
}

/// Query one channel descriptor from one ASIO session.
pub(super) fn query_channel_descriptor(
    session: &Arc<AsioSession>,
    is_input: bool,
    channel_index: i32,
) -> RuntimeResult<AsioChannelDescriptor> {
    let mut channel_info = AsioChannelInfo {
        channel: channel_index,
        is_input: if is_input { ASIO_TRUE } else { ASIO_FALSE },
        is_active: 0,
        channel_group: 0,
        sample_type: 0,
        name: [0; 32],
    };

    // fetch one channel info payload from IASIO
    let status = unsafe { asio_driver_get_channel_info(session.driver.raw, &mut channel_info) };
    if status != ASE_OK {
        return Err(asio_error(
            "destack.audio.internal.asio.channelInfo",
            status,
            &session.driver_name,
            format!("failed to read ASIO channel info for channel {channel_index}"),
        ));
    }

    let encoding = map_sample_encoding(channel_info.sample_type).ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(format!(
            "destack.audio.stream.open unsupported ASIO sample type {}",
            channel_info.sample_type,
        )))
        .boxed()
    })?;

    Ok(AsioChannelDescriptor { encoding })
}

/// Map one raw ASIO sample type into one normalized encoding descriptor.
fn map_sample_encoding(sample_type: i32) -> Option<AsioSampleEncoding> {
    match sample_type {
        ASIO_ST_INT16_LSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::S16,
            bytes_per_sample: 2,
            is_big_endian: false,
            is_packed_24: false,
        }),
        ASIO_ST_INT16_MSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::S16,
            bytes_per_sample: 2,
            is_big_endian: true,
            is_packed_24: false,
        }),
        ASIO_ST_INT24_LSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::S24,
            bytes_per_sample: 3,
            is_big_endian: false,
            is_packed_24: true,
        }),
        ASIO_ST_INT24_MSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::S24,
            bytes_per_sample: 3,
            is_big_endian: true,
            is_packed_24: true,
        }),
        ASIO_ST_INT32_LSB | ASIO_ST_INT32_LSB24 => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::S32,
            bytes_per_sample: 4,
            is_big_endian: false,
            is_packed_24: false,
        }),
        ASIO_ST_INT32_MSB | ASIO_ST_INT32_MSB24 => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::S32,
            bytes_per_sample: 4,
            is_big_endian: true,
            is_packed_24: false,
        }),
        ASIO_ST_FLOAT32_LSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::F32,
            bytes_per_sample: 4,
            is_big_endian: false,
            is_packed_24: false,
        }),
        ASIO_ST_FLOAT32_MSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::F32,
            bytes_per_sample: 4,
            is_big_endian: true,
            is_packed_24: false,
        }),
        ASIO_ST_FLOAT64_LSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::F64,
            bytes_per_sample: 8,
            is_big_endian: false,
            is_packed_24: false,
        }),
        ASIO_ST_FLOAT64_MSB => Some(AsioSampleEncoding {
            format: audio_core::AudioSampleFormat::F64,
            bytes_per_sample: 8,
            is_big_endian: true,
            is_packed_24: false,
        }),
        _ => None,
    }
}

/// Decode one C string payload using lossy UTF-8 conversion.
fn c_string_lossy(bytes: &[i8]) -> Option<String> {
    let nul_index = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    if nul_index == 0 {
        return None;
    }

    let bytes = bytes[..nul_index]
        .iter()
        .map(|value| *value as u8)
        .collect::<Vec<_>>();
    Some(String::from_utf8_lossy(&bytes).to_string())
}

/// Return one canonical layout for one channel count.
pub(super) fn channel_layout(channels: u16) -> audio_core::AudioChannelLayout {
    match channels {
        1 => audio_core::AudioChannelLayout::Mono,
        2 => audio_core::AudioChannelLayout::Stereo,
        4 => audio_core::AudioChannelLayout::Quad,
        5 => audio_core::AudioChannelLayout::Surround41,
        6 => audio_core::AudioChannelLayout::Surround51,
        7 => audio_core::AudioChannelLayout::Surround61,
        8 => audio_core::AudioChannelLayout::Surround71,
        _ => audio_core::AudioChannelLayout::Unknown,
    }
}

/// Return one packed channel mask for one channel count.
pub(super) fn channel_mask(channels: u16) -> u64 {
    if channels == 0 {
        return 0;
    }

    let clamped_channels = channels.min(63);
    (1u64 << clamped_channels) - 1u64
}

/// Resolve one requested buffer size against one ASIO size contract.
pub(super) fn resolve_buffer_size(
    requested: u32,
    min_size: u32,
    max_size: u32,
    preferred: u32,
    granularity: i32,
) -> u32 {
    let mut resolved = if requested == 0 { preferred } else { requested };
    resolved = resolved.max(min_size).min(max_size.max(min_size));

    // align to one power-of-two range when required by the driver
    if granularity == -1 {
        let mut candidate = min_size.max(1).next_power_of_two();
        let mut best = candidate;
        let mut best_distance = candidate.abs_diff(resolved);

        while candidate <= max_size {
            let distance = candidate.abs_diff(resolved);
            if distance < best_distance {
                best = candidate;
                best_distance = distance;
            }

            if candidate > u32::MAX / 2 {
                break;
            }
            candidate = candidate.saturating_mul(2);
        }

        return best.max(min_size).min(max_size.max(min_size));
    }

    // align to one explicit granularity step when required by the driver
    if granularity > 1 {
        let step = granularity as u32;
        let rounded = resolved.div_ceil(step) * step;
        return rounded.max(min_size).min(max_size.max(min_size));
    }

    resolved
}

/// Return one runtime-friendly ASIO driver row by stable key name.
pub(super) fn driver_by_key_name(key_name: &str) -> RuntimeResult<AsioDriverRow> {
    enumerate_registered_drivers()
        .into_iter()
        .find(|row| row.key_name == key_name)
        .ok_or_else(|| {
            audio_core::audio_not_found(
                "destack.audio.stream.open",
                format!("asio driver not found: {key_name}"),
            )
        })
}
