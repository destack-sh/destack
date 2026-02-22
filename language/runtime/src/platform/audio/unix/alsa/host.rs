use std::collections::HashSet;
use std::ffi::{CStr, CString, c_int, c_uint, c_void};
use std::ptr;
use std::sync::Arc;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;

use super::abi::{AlsaHardwareParams, AlsaPcm, AlsaUnsignedFrames};
use super::constants::*;
use super::core::{
    AlsaDeviceProfile, AlsaHintDirection, AlsaHintRow, AlsaLibrary, AlsaPcmHandle, alsa_error,
    alsa_succeeded, c_string,
};
use super::ffi::alsa_library;

/// One configured ALSA PCM handle with negotiated runtime parameters.
#[derive(Debug)]
pub(super) struct ConfiguredAlsaPcm {
    /// Opened PCM handle.
    pub(super) pcm: Arc<AlsaPcmHandle>,
    /// Negotiated sample rate in hertz.
    pub(super) sample_rate: u32,
    /// Negotiated channel count.
    pub(super) channels: u16,
    /// Negotiated period in frames.
    pub(super) period_frames: u32,
    /// Whether pause and resume is supported for this lane.
    pub(super) supports_pause: bool,
    /// Whether this lane appears to map to one hardware endpoint.
    pub(super) supports_exclusive: bool,
}

/// Return one loaded ALSA library or one not-supported error.
pub(super) fn require_alsa_library(
    operation: &'static str,
) -> RuntimeResult<&'static Arc<AlsaLibrary>> {
    alsa_library().ok_or_else(|| {
        RuntimeError::from(PlatformError::not_supported(format!(
            "{operation}: ALSA dynamic library is unavailable on this host",
        )))
        .boxed()
    })
}

/// One owned `snd_pcm_hw_params_t` payload.
#[derive(Debug)]
struct AlsaHardwareParamsOwner {
    /// Borrowed ALSA symbol table for deallocation.
    library: &'static Arc<AlsaLibrary>,
    /// Raw `snd_pcm_hw_params_t*` pointer.
    raw: *mut AlsaHardwareParams,
}

impl Drop for AlsaHardwareParamsOwner {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        // free one allocated hardware-params payload
        unsafe {
            (self.library.api.snd_pcm_hw_params_free)(self.raw);
        }
    }
}

/// Allocate one `snd_pcm_hw_params_t` payload.
fn allocate_hardware_params(
    library: &'static Arc<AlsaLibrary>,
    operation: &'static str,
) -> RuntimeResult<AlsaHardwareParamsOwner> {
    let mut raw = ptr::null_mut();

    // allocate one hardware-params object via ALSA
    let status = unsafe { (library.api.snd_pcm_hw_params_malloc)(&mut raw) };
    if !alsa_succeeded(status) || raw.is_null() {
        return Err(alsa_error(
            operation,
            status,
            "failed to allocate ALSA hardware-params payload",
        ));
    }

    Ok(AlsaHardwareParamsOwner { library, raw })
}

/// Return one utf8 string extracted from one ALSA hint payload.
fn hint_string(library: &AlsaLibrary, hint: *const c_void, key: &str) -> Option<String> {
    let key = CString::new(key).ok()?;

    // resolve one hint field string from one hint row
    let value_pointer = unsafe { (library.api.snd_device_name_get_hint)(hint, key.as_ptr()) };
    if value_pointer.is_null() {
        return None;
    }

    let value = unsafe { CStr::from_ptr(value_pointer).to_string_lossy().into_owned() };

    // release one string allocated by ALSA hint utilities
    unsafe {
        libc::free(value_pointer as *mut c_void);
    }

    let value = value.trim();
    if value.is_empty() {
        return None;
    }

    Some(value.to_string())
}

/// Parse one ALSA hint direction marker from one IOID value.
fn hint_direction(value: Option<String>) -> Option<AlsaHintDirection> {
    let value = value?;

    if value.eq_ignore_ascii_case("Input") {
        return Some(AlsaHintDirection::Capture);
    }

    if value.eq_ignore_ascii_case("Output") {
        return Some(AlsaHintDirection::Playback);
    }

    None
}

/// Normalize one ALSA hint description into one concise display string.
fn normalize_hint_description(device_name: &str, description: Option<String>) -> String {
    let Some(description) = description else {
        return device_name.to_string();
    };

    let first_line = description.lines().next().unwrap_or_default().trim();
    if first_line.is_empty() {
        return device_name.to_string();
    }

    first_line.to_string()
}

/// Enumerate ALSA hint rows from `snd_device_name_hint`.
pub(super) fn enumerate_hint_rows() -> RuntimeResult<Vec<AlsaHintRow>> {
    let library = require_alsa_library("destack.audio.device.list")?;
    let interface = c_string(ALSA_HINT_PCM_INTERFACE, "interface")?;
    let mut hints = ptr::null_mut::<*mut c_void>();

    // query ALSA for one null-terminated hint row list
    let status = unsafe {
        (library.api.snd_device_name_hint)(
            -1,
            interface.as_ptr(),
            &mut hints as *mut *mut *mut c_void,
        )
    };
    if !alsa_succeeded(status) {
        return Ok(vec![AlsaHintRow {
            name: ALSA_DEFAULT_DEVICE_NAME.to_string(),
            description: ALSA_DEFAULT_DEVICE_NAME.to_string(),
            direction: None,
        }]);
    }

    let mut rows = Vec::new();
    let mut seen_names = HashSet::new();
    let mut index = 0usize;

    // decode each row in the null-terminated hint list
    loop {
        let hint = unsafe { *hints.add(index) };
        if hint.is_null() {
            break;
        }

        let Some(name) = hint_string(library, hint, ALSA_HINT_NAME_KEY) else {
            index = index.saturating_add(1);
            continue;
        };

        if !seen_names.insert(name.clone()) {
            index = index.saturating_add(1);
            continue;
        }

        let description = normalize_hint_description(
            &name,
            hint_string(library, hint, ALSA_HINT_DESCRIPTION_KEY),
        );
        let direction = hint_direction(hint_string(library, hint, ALSA_HINT_IOID_KEY));

        rows.push(AlsaHintRow {
            name,
            description,
            direction,
        });

        index = index.saturating_add(1);
    }

    // release ALSA-allocated hint rows
    unsafe {
        let _ = (library.api.snd_device_name_free_hint)(hints);
    }

    // always expose one fallback row for default-device routing
    if rows.is_empty() {
        rows.push(AlsaHintRow {
            name: ALSA_DEFAULT_DEVICE_NAME.to_string(),
            description: ALSA_DEFAULT_DEVICE_NAME.to_string(),
            direction: None,
        });
    }

    Ok(rows)
}

/// Open one raw ALSA PCM handle.
fn open_pcm(
    library: &AlsaLibrary,
    operation: &'static str,
    device_name: &str,
    stream_selector: c_int,
    open_flags: c_int,
) -> RuntimeResult<*mut AlsaPcm> {
    let device_name = c_string(device_name, "deviceName")?;
    let mut raw_pcm = ptr::null_mut();

    // open one ALSA pcm endpoint by one explicit stream selector
    let status = unsafe {
        (library.api.snd_pcm_open)(
            &mut raw_pcm,
            device_name.as_ptr(),
            stream_selector,
            open_flags,
        )
    };
    if !alsa_succeeded(status) || raw_pcm.is_null() {
        return Err(alsa_error(
            operation,
            status,
            format!("failed to open ALSA pcm {device_name:?}"),
        ));
    }

    Ok(raw_pcm)
}

/// Return one ALSA format enum value for one token.
fn format_value(library: &AlsaLibrary, format_name: &str) -> Option<c_int> {
    let format_name = CString::new(format_name).ok()?;

    // resolve one format token through ALSA format lookup
    let value = unsafe { (library.api.snd_pcm_format_value)(format_name.as_ptr()) };
    if value < 0 {
        return None;
    }

    Some(value)
}

/// Return ALSA format tokens for one runtime sample format.
fn format_tokens(format: audio_core::AudioSampleFormat) -> &'static [&'static str] {
    match format {
        audio_core::AudioSampleFormat::U8 => &["U8"],
        audio_core::AudioSampleFormat::S16 => &["S16_LE"],
        audio_core::AudioSampleFormat::S24 => &["S24_LE", "S32_LE"],
        audio_core::AudioSampleFormat::S32 => &["S32_LE"],
        audio_core::AudioSampleFormat::F32 => &["FLOAT_LE"],
        audio_core::AudioSampleFormat::F64 => &["FLOAT64_LE"],
    }
}

/// Apply one stream format, channel, rate, and period contract.
fn configure_pcm(
    library: &'static Arc<AlsaLibrary>,
    raw_pcm: *mut AlsaPcm,
    config: audio_core::AudioStreamConfig,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<(u32, u16, u32, bool, bool)> {
    // enforce backend no-resample mode when the caller requests it
    let request_no_resample = (backend_flags.0 & audio_core::BACKEND_OPEN_ALSA_NO_RESAMPLE.0) != 0;

    let parameters = allocate_hardware_params(library, "destack.audio.stream.open")?;

    // initialize one full hardware-configuration search space
    let status = unsafe { (library.api.snd_pcm_hw_params_any)(raw_pcm, parameters.raw) };
    if !alsa_succeeded(status) {
        return Err(alsa_error(
            "destack.audio.stream.open",
            status,
            "failed to initialize ALSA hardware-params space",
        ));
    }

    // disable ALSA resampling so opens fail instead of implicit conversion
    if request_no_resample {
        let status = unsafe {
            (library.api.snd_pcm_hw_params_set_rate_resample)(
                raw_pcm,
                parameters.raw,
                ALSA_FALSE as c_uint,
            )
        };
        if !alsa_succeeded(status) {
            return Err(alsa_error(
                "destack.audio.stream.open",
                status,
                "failed to disable ALSA sample-rate resampling",
            ));
        }
    }

    // enforce interleaved transfer layout for runtime queue interoperability
    let status = unsafe {
        (library.api.snd_pcm_hw_params_set_access)(
            raw_pcm,
            parameters.raw,
            ALSA_ACCESS_RW_INTERLEAVED,
        )
    };
    if !alsa_succeeded(status) {
        return Err(alsa_error(
            "destack.audio.stream.open",
            status,
            "failed to set ALSA interleaved access mode",
        ));
    }

    let mut selected_format = None;

    // select one compatible ALSA sample format for this runtime stream format
    for token in format_tokens(config.format) {
        let Some(value) = format_value(library, token) else {
            continue;
        };

        let status =
            unsafe { (library.api.snd_pcm_hw_params_set_format)(raw_pcm, parameters.raw, value) };
        if alsa_succeeded(status) {
            selected_format = Some(value);
            break;
        }
    }

    if selected_format.is_none() {
        return Err(RuntimeError::from(PlatformError::not_supported(
            "destack.audio.stream.open ALSA sample format",
        ))
        .boxed());
    }

    let mut negotiated_channels = config.channels.max(1) as c_uint;

    // negotiate one near-supported channel count
    let status = unsafe {
        (library.api.snd_pcm_hw_params_set_channels_near)(
            raw_pcm,
            parameters.raw,
            &mut negotiated_channels,
        )
    };
    if !alsa_succeeded(status) {
        return Err(alsa_error(
            "destack.audio.stream.open",
            status,
            "failed to negotiate ALSA channel count",
        ));
    }

    let mut negotiated_rate = config.sample_rate.max(1) as c_uint;
    let mut rate_direction = 0;

    // negotiate one near-supported sample rate
    let status = unsafe {
        (library.api.snd_pcm_hw_params_set_rate_near)(
            raw_pcm,
            parameters.raw,
            &mut negotiated_rate,
            &mut rate_direction,
        )
    };
    if !alsa_succeeded(status) {
        return Err(alsa_error(
            "destack.audio.stream.open",
            status,
            "failed to negotiate ALSA sample rate",
        ));
    }

    let mut negotiated_period = config
        .period_frames
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES)
        as AlsaUnsignedFrames;
    let mut period_direction = 0;

    // negotiate one near-supported period size
    let status = unsafe {
        (library.api.snd_pcm_hw_params_set_period_size_near)(
            raw_pcm,
            parameters.raw,
            &mut negotiated_period,
            &mut period_direction,
        )
    };
    if !alsa_succeeded(status) {
        return Err(alsa_error(
            "destack.audio.stream.open",
            status,
            "failed to negotiate ALSA period size",
        ));
    }

    let mut buffer_size = negotiated_period.saturating_mul(4);

    // request one conservative ring-buffer size target
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_set_buffer_size_near)(
            raw_pcm,
            parameters.raw,
            &mut buffer_size,
        )
    };

    // install one selected hardware configuration on this PCM lane
    let status = unsafe { (library.api.snd_pcm_hw_params)(raw_pcm, parameters.raw) };
    if !alsa_succeeded(status) {
        return Err(alsa_error(
            "destack.audio.stream.open",
            status,
            "failed to apply ALSA hardware parameters",
        ));
    }

    // refresh one hardware-params snapshot from negotiated device state
    let _ = unsafe { (library.api.snd_pcm_hw_params_current)(raw_pcm, parameters.raw) };

    let mut effective_rate = 0u32;
    let mut effective_rate_direction = 0;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_rate)(
            parameters.raw,
            &mut effective_rate as *mut u32,
            &mut effective_rate_direction,
        )
    };

    let mut effective_channels = 0u32;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_channels)(
            parameters.raw,
            &mut effective_channels as *mut u32,
        )
    };

    let mut effective_period = 0 as AlsaUnsignedFrames;
    let mut effective_period_direction = 0;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_period_size)(
            parameters.raw,
            &mut effective_period,
            &mut effective_period_direction,
        )
    };

    let supports_pause =
        unsafe { (library.api.snd_pcm_hw_params_can_pause)(parameters.raw) == ALSA_TRUE };

    let supports_exclusive =
        unsafe { (library.api.snd_pcm_type)(raw_pcm) == ALSA_PCM_TYPE_HARDWARE };

    Ok((
        effective_rate.max(1),
        effective_channels
            .max(1)
            .min(ALSA_MAX_PROBED_CHANNELS as u32) as u16,
        (effective_period as u32).max(audio_core::MIN_STREAM_PERIOD_FRAMES),
        supports_pause,
        supports_exclusive,
    ))
}

/// Open and configure one ALSA PCM lane.
pub(super) fn open_configured_pcm(
    operation: &'static str,
    device_name: &str,
    stream_selector: c_int,
    config: audio_core::AudioStreamConfig,
    backend_flags: audio_core::AudioBackendOpenFlags,
) -> RuntimeResult<ConfiguredAlsaPcm> {
    let library = require_alsa_library(operation)?;
    let raw_pcm = open_pcm(
        library,
        operation,
        device_name,
        stream_selector,
        ALSA_FLAG_NONE,
    )?;

    let configured = match configure_pcm(library, raw_pcm, config, backend_flags) {
        Ok(configured) => configured,
        Err(error) => {
            unsafe {
                let _ = (library.api.snd_pcm_close)(raw_pcm);
            }
            return Err(error);
        }
    };

    // put this pcm lane into nonblocking mode for worker-loop control
    let status = unsafe { (library.api.snd_pcm_nonblock)(raw_pcm, ALSA_NONBLOCK_ENABLED) };
    if !alsa_succeeded(status) {
        unsafe {
            let _ = (library.api.snd_pcm_close)(raw_pcm);
        }

        return Err(alsa_error(
            operation,
            status,
            "failed to enable ALSA nonblocking mode",
        ));
    }

    // prepare this lane for first start or transfer call
    let status = unsafe { (library.api.snd_pcm_prepare)(raw_pcm) };
    if !alsa_succeeded(status) {
        unsafe {
            let _ = (library.api.snd_pcm_close)(raw_pcm);
        }

        return Err(alsa_error(
            operation,
            status,
            "failed to prepare ALSA stream lane",
        ));
    }

    let (sample_rate, channels, period_frames, supports_pause, supports_exclusive) = configured;

    Ok(ConfiguredAlsaPcm {
        pcm: Arc::new(AlsaPcmHandle { raw: raw_pcm }),
        sample_rate,
        channels,
        period_frames,
        supports_pause,
        supports_exclusive,
    })
}

/// Probe one device profile for one stream selector.
fn probe_device_profile(device_name: &str, stream_selector: c_int) -> Option<AlsaDeviceProfile> {
    let library = alsa_library()?;
    let raw_pcm = open_pcm(
        library,
        "destack.audio.internal.alsa.probe",
        device_name,
        stream_selector,
        ALSA_FLAG_NONE,
    )
    .ok()?;

    let parameters = match allocate_hardware_params(library, "destack.audio.internal.alsa.probe") {
        Ok(parameters) => parameters,
        Err(_) => {
            unsafe {
                let _ = (library.api.snd_pcm_close)(raw_pcm);
            }
            return None;
        }
    };

    // initialize probe configuration space for one PCM lane
    let status = unsafe { (library.api.snd_pcm_hw_params_any)(raw_pcm, parameters.raw) };
    if !alsa_succeeded(status) {
        unsafe {
            let _ = (library.api.snd_pcm_close)(raw_pcm);
        }
        return None;
    }

    let mut min_rate = 0u32;
    let mut min_rate_direction = 0;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_rate_min)(
            parameters.raw,
            &mut min_rate as *mut u32,
            &mut min_rate_direction,
        )
    };

    let mut max_rate = 0u32;
    let mut max_rate_direction = 0;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_rate_max)(
            parameters.raw,
            &mut max_rate as *mut u32,
            &mut max_rate_direction,
        )
    };

    let mut min_channels = 0u32;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_channels_min)(
            parameters.raw,
            &mut min_channels as *mut u32,
        )
    };

    let mut max_channels = 0u32;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_channels_max)(
            parameters.raw,
            &mut max_channels as *mut u32,
        )
    };

    let mut min_period = 0 as AlsaUnsignedFrames;
    let mut min_period_direction = 0;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_period_size_min)(
            parameters.raw,
            &mut min_period,
            &mut min_period_direction,
        )
    };

    let mut max_period = 0 as AlsaUnsignedFrames;
    let mut max_period_direction = 0;
    let _ = unsafe {
        (library.api.snd_pcm_hw_params_get_period_size_max)(
            parameters.raw,
            &mut max_period,
            &mut max_period_direction,
        )
    };

    let mut format_mask = 0u32;

    // probe each runtime format candidate against this ALSA configuration space
    for candidate in ALSA_FORMAT_CANDIDATES {
        let Some(format_value) = format_value(library, candidate.alsa_name) else {
            continue;
        };

        let status = unsafe {
            (library.api.snd_pcm_hw_params_test_format)(raw_pcm, parameters.raw, format_value)
        };
        if alsa_succeeded(status) {
            format_mask |= audio_core::sample_format_bit(candidate.runtime_format);
        }
    }

    if format_mask == 0 {
        format_mask = audio_core::sample_format_bit(audio_core::AudioSampleFormat::F32);
    }

    let preferred_sample_rate = ALSA_PROBED_SAMPLE_RATES
        .iter()
        .copied()
        .find(|rate| {
            let status = unsafe {
                (library.api.snd_pcm_hw_params_test_rate)(
                    raw_pcm,
                    parameters.raw,
                    *rate,
                    ALSA_FALSE,
                )
            };

            alsa_succeeded(status)
        })
        .unwrap_or(ALSA_FALLBACK_PREFERRED_SAMPLE_RATE);

    let min_rate = min_rate.max(1);
    let max_rate = max_rate.max(min_rate);

    let preferred_sample_rate = preferred_sample_rate.clamp(min_rate, max_rate);

    let min_channels = min_channels.max(1) as u16;
    let max_channels = max_channels
        .max(min_channels as u32)
        .min(ALSA_MAX_PROBED_CHANNELS as u32) as u16;

    let preferred_channels = if (2u16 >= min_channels) && (2u16 <= max_channels) {
        2
    } else {
        min_channels
    };

    let min_period_frames = (min_period as u32).max(audio_core::MIN_STREAM_PERIOD_FRAMES);
    let max_period_frames = (max_period as u32).max(min_period_frames);
    let preferred_period_frames =
        ALSA_FALLBACK_PREFERRED_PERIOD_FRAMES.clamp(min_period_frames, max_period_frames);

    let supports_pause =
        unsafe { (library.api.snd_pcm_hw_params_can_pause)(parameters.raw) == ALSA_TRUE };
    let supports_exclusive =
        unsafe { (library.api.snd_pcm_type)(raw_pcm) == ALSA_PCM_TYPE_HARDWARE };

    unsafe {
        let _ = (library.api.snd_pcm_close)(raw_pcm);
    }

    Some(AlsaDeviceProfile {
        preferred_sample_rate,
        min_sample_rate: min_rate,
        max_sample_rate: max_rate,
        preferred_period_frames,
        min_period_frames,
        max_period_frames,
        preferred_channels,
        min_channels,
        max_channels,
        format_mask,
        supports_pause,
        supports_exclusive,
    })
}

/// Return one probed playback and capture profile pair for one hint row.
pub(super) fn probe_row_profiles(
    row: &AlsaHintRow,
) -> (Option<AlsaDeviceProfile>, Option<AlsaDeviceProfile>) {
    // honor one explicit capture-only hint marker when present
    let playback_profile = if row.direction == Some(AlsaHintDirection::Capture) {
        None
    } else {
        probe_device_profile(&row.name, ALSA_STREAM_PLAYBACK)
    };

    // honor one explicit playback-only hint marker when present
    let capture_profile = if row.direction == Some(AlsaHintDirection::Playback) {
        None
    } else {
        probe_device_profile(&row.name, ALSA_STREAM_CAPTURE)
    };

    (playback_profile, capture_profile)
}

/// Recover one ALSA lane from one xrun or suspend error.
pub(super) fn recover_pcm(
    library: &AlsaLibrary,
    pcm: *mut AlsaPcm,
    status: c_int,
    operation: &'static str,
) -> RuntimeResult<()> {
    // recover one stream lane from one ALSA transient error
    let recovered = unsafe { (library.api.snd_pcm_recover)(pcm, status, ALSA_RECOVER_SILENT) };
    if alsa_succeeded(recovered) {
        return Ok(());
    }

    Err(alsa_error(
        operation,
        recovered,
        "failed to recover ALSA stream lane",
    ))
}

/// Wait until one ALSA lane becomes readable or writable.
pub(super) fn wait_for_pcm_ready(pcm: *mut AlsaPcm) {
    let Some(library) = alsa_library() else {
        return;
    };

    // wait one short timeslice for one ready transfer window
    let status = unsafe { (library.api.snd_pcm_wait)(pcm, ALSA_WAIT_TIMEOUT_MILLISECONDS) };
    if status >= 0 {
        return;
    }

    let _ = recover_pcm(
        library,
        pcm,
        status,
        "destack.audio.internal.alsa.transfer.wait",
    );
}
