use std::ffi::c_void;
use std::ptr;
use std::sync::OnceLock;

use super::abi::*;
use super::constants::*;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, audio as audio_types};

/// Return whether OpenSL ES backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    cfg!(target_os = "android")
        && *OPENSLES_AVAILABLE_SLOT.get_or_init(|| probe_backend_available().is_ok())
}

/// One process-global backend probe slot.
static OPENSLES_AVAILABLE_SLOT: OnceLock<bool> = OnceLock::new();

/// One owned OpenSL ES object handle.
#[derive(Debug)]
pub(super) struct SlObjectHandle {
    /// Raw object interface pointer.
    pub(super) raw: SLObjectItf,
}

impl Drop for SlObjectHandle {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        let table = unsafe { *self.raw };
        if table.is_null() {
            return;
        }

        let Some(destroy) = (unsafe { (*table).Destroy }) else {
            return;
        };

        // destroy one realized OpenSL ES object
        unsafe {
            destroy(self.raw);
        }
    }
}

unsafe impl Send for SlObjectHandle {}
unsafe impl Sync for SlObjectHandle {}

/// One opened OpenSL ES engine payload.
#[derive(Debug)]
pub(super) struct OpenSlEngine {
    /// OpenSL ES engine object handle.
    pub(super) _engine_object: SlObjectHandle,
    /// OpenSL ES engine interface pointer.
    pub(super) engine_interface: SLEngineItf,
    /// OpenSL ES output-mix object handle.
    pub(super) output_mix_object: SlObjectHandle,
}

unsafe impl Send for OpenSlEngine {}
unsafe impl Sync for OpenSlEngine {}

/// Probe one backend engine creation roundtrip.
fn probe_backend_available() -> RuntimeResult<()> {
    let _ = create_engine("destack.audio.device.list")?;
    Ok(())
}

/// Return one normalized channel layout from one channel count.
pub(super) fn channel_layout(channel_count: u16) -> audio_types::AudioChannelLayout {
    match channel_count {
        1 => audio_types::AudioChannelLayout::Mono,
        2 => audio_types::AudioChannelLayout::Stereo,
        _ => audio_types::AudioChannelLayout::Unknown,
    }
}

/// Return one contiguous channel mask from one channel count.
pub(super) fn channel_mask(channel_count: u16) -> u64 {
    if channel_count == 0 {
        return 0;
    }

    if channel_count >= 64 {
        return u64::MAX;
    }

    (1u64 << channel_count) - 1
}

/// Return one conservative OpenSL ES sample-format mask.
pub(super) fn opensles_format_mask() -> u32 {
    audio_core::sample_format_bit(audio_types::AudioSampleFormat::S16)
}

/// Return one OpenSL ES PCM channel mask from one channel count.
pub(super) fn opensles_pcm_channel_mask(channel_count: u16) -> SLuint32 {
    if channel_count <= 1 {
        return SL_SPEAKER_FRONT_CENTER;
    }

    SL_SPEAKER_FRONT_LEFT | SL_SPEAKER_FRONT_RIGHT
}

/// Build one OpenSL ES PCM format payload.
pub(super) fn pcm_format(
    channel_count: u16,
    sample_rate_hz: u32,
) -> RuntimeResult<SLDataFormat_PCM> {
    let sample_rate_millihertz = sample_rate_hz.checked_mul(1000).ok_or_else(|| {
        RuntimeError::from(PlatformError::invalid_argument_value(
            "config.sampleRate",
            "config.sampleRate exceeds OpenSL ES limits",
        ))
        .boxed()
    })?;

    Ok(SLDataFormat_PCM {
        formatType: SL_DATAFORMAT_PCM,
        numChannels: channel_count as SLuint32,
        samplesPerSec: sample_rate_millihertz,
        bitsPerSample: SL_PCMSAMPLEFORMAT_FIXED_16,
        containerSize: SL_PCMSAMPLEFORMAT_FIXED_16,
        channelMask: opensles_pcm_channel_mask(channel_count),
        endianness: SL_BYTEORDER_LITTLEENDIAN,
    })
}

/// Return one OpenSL ES runtime error payload.
pub(super) fn opensles_error(
    operation: &'static str,
    status: Option<SLresult>,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    let message = message.into();
    let message = if let Some(status) = status {
        format!(
            "{message} (opensles status {status}: {})",
            opensles_status_text(status)
        )
    } else {
        message
    };

    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        message,
    ))
    .boxed()
}

/// Return one OpenSL ES non-supported error payload.
pub(super) fn opensles_not_supported(
    operation: &'static str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::not_supported(format!(
        "{operation}: {}",
        message.into()
    )))
    .boxed()
}

/// Return whether one OpenSL ES status code denotes success.
pub(super) fn opensles_succeeded(status: SLresult) -> bool {
    status == SL_RESULT_SUCCESS
}

/// Open one backend engine or return one not-supported error.
pub(super) fn require_backend_engine(operation: &'static str) -> RuntimeResult<OpenSlEngine> {
    if !cfg!(target_os = "android") {
        return Err(opensles_not_supported(
            operation,
            "OpenSL ES backend is only available on Android",
        ));
    }

    create_engine(operation)
}

/// Return one human-readable OpenSL ES status text.
fn opensles_status_text(status: SLresult) -> &'static str {
    match status {
        0 => "success",
        1 => "preconditions violated",
        2 => "parameter invalid",
        3 => "memory failure",
        4 => "resource error",
        5 => "resource lost",
        6 => "io error",
        7 => "buffer insufficient",
        8 => "content corrupted",
        9 => "content unsupported",
        10 => "content not found",
        11 => "permission denied",
        12 => "feature unsupported",
        13 => "internal error",
        14 => "unknown error",
        15 => "operation aborted",
        16 => "control lost",
        _ => "unknown status",
    }
}

/// Return one OpenSL ES sample format for one runtime format.
pub(super) fn opensles_sample_format(
    format: audio_types::AudioSampleFormat,
) -> Option<audio_types::AudioSampleFormat> {
    match format {
        audio_types::AudioSampleFormat::S16 => Some(audio_types::AudioSampleFormat::S16),
        _ => None,
    }
}

/// Return one requested interface id pointer.
pub(super) fn interface_id(
    operation: &'static str,
    name: &'static str,
    value: SLInterfaceID,
) -> RuntimeResult<SLInterfaceID> {
    if value.is_null() {
        return Err(opensles_not_supported(
            operation,
            format!("OpenSL ES interface id {name} is unavailable"),
        ));
    }

    Ok(value)
}

/// Create one OpenSL ES engine and output mix pair.
pub(super) fn create_engine(operation: &'static str) -> RuntimeResult<OpenSlEngine> {
    let mut engine_object = ptr::null::<*const SLObjectItf_>();
    let status = unsafe {
        slCreateEngine(
            &mut engine_object,
            0,
            ptr::null(),
            0,
            ptr::null(),
            ptr::null(),
        )
    };
    if !opensles_succeeded(status) || engine_object.is_null() {
        return Err(opensles_not_supported(
            operation,
            "failed to create OpenSL ES engine object",
        ));
    }

    let engine_object = SlObjectHandle { raw: engine_object };
    realize_object(engine_object.raw, operation, "engine object")?;

    let engine_iid = interface_id(operation, "SL_IID_ENGINE", unsafe { SL_IID_ENGINE })?;
    let engine_interface =
        get_object_interface(engine_object.raw, engine_iid, operation, "engine interface")?
            as SLEngineItf;
    if engine_interface.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "failed to resolve OpenSL ES engine interface pointer",
        ));
    }

    let mut output_mix_object = ptr::null::<*const SLObjectItf_>();
    let engine_table = unsafe { *engine_interface };
    if engine_table.is_null() {
        return Err(opensles_error(
            operation,
            None,
            "failed to load OpenSL ES engine interface table",
        ));
    }

    let Some(create_output_mix) = (unsafe { (*engine_table).CreateOutputMix }) else {
        return Err(opensles_not_supported(
            operation,
            "OpenSL ES output mix creation is unavailable",
        ));
    };

    // create one default output mix object
    let status = unsafe {
        create_output_mix(
            engine_interface,
            &mut output_mix_object,
            0,
            ptr::null(),
            ptr::null(),
        )
    };
    if !opensles_succeeded(status) || output_mix_object.is_null() {
        return Err(opensles_error(
            operation,
            Some(status),
            "failed to create OpenSL ES output mix object",
        ));
    }

    let output_mix_object = SlObjectHandle {
        raw: output_mix_object,
    };
    realize_object(output_mix_object.raw, operation, "output mix object")?;

    Ok(OpenSlEngine {
        _engine_object: engine_object,
        engine_interface,
        output_mix_object,
    })
}

/// Realize one OpenSL ES object handle.
pub(super) fn realize_object(
    object: SLObjectItf,
    operation: &'static str,
    object_name: &'static str,
) -> RuntimeResult<()> {
    if object.is_null() {
        return Err(opensles_error(
            operation,
            None,
            format!("cannot realize null OpenSL ES {object_name}"),
        ));
    }

    let table = unsafe { *object };
    if table.is_null() {
        return Err(opensles_error(
            operation,
            None,
            format!("cannot realize OpenSL ES {object_name}: missing vtable"),
        ));
    }

    let Some(realize) = (unsafe { (*table).Realize }) else {
        return Err(opensles_not_supported(
            operation,
            format!("OpenSL ES {object_name} realize operation is unavailable"),
        ));
    };

    let status = unsafe { realize(object, SL_BOOLEAN_FALSE) };
    if opensles_succeeded(status) {
        return Ok(());
    }

    Err(opensles_error(
        operation,
        Some(status),
        format!("failed to realize OpenSL ES {object_name}"),
    ))
}

/// Read one interface pointer from one OpenSL ES object.
pub(super) fn get_object_interface(
    object: SLObjectItf,
    interface_id: SLInterfaceID,
    operation: &'static str,
    interface_name: &'static str,
) -> RuntimeResult<*mut c_void> {
    if object.is_null() {
        return Err(opensles_error(
            operation,
            None,
            format!("cannot read OpenSL ES {interface_name} from null object"),
        ));
    }

    let table = unsafe { *object };
    if table.is_null() {
        return Err(opensles_error(
            operation,
            None,
            format!("cannot read OpenSL ES {interface_name}: missing object vtable"),
        ));
    }

    let Some(get_interface) = (unsafe { (*table).GetInterface }) else {
        return Err(opensles_not_supported(
            operation,
            format!("OpenSL ES getInterface for {interface_name} is unavailable"),
        ));
    };

    let mut interface = ptr::null_mut::<c_void>();
    let status = unsafe {
        get_interface(
            object,
            interface_id,
            (&mut interface as *mut *mut c_void).cast::<c_void>(),
        )
    };
    if !opensles_succeeded(status) || interface.is_null() {
        return Err(opensles_error(
            operation,
            Some(status),
            format!("failed to resolve OpenSL ES {interface_name}"),
        ));
    }

    Ok(interface)
}
