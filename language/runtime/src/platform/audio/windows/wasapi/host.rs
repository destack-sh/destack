use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;

use super::abi::{
    audio_client_get_device_period, audio_client_get_mix_format, audio_client_get_service,
    audio_client_is_format_supported, imm_device_activate, imm_device_collection_get_count,
    imm_device_collection_item, imm_device_enumerator_enum_audio_endpoints,
    imm_device_enumerator_get_default_audio_endpoint, imm_device_enumerator_get_device,
    imm_device_get_id, imm_device_open_property_store, property_store_get_value,
};
use super::constants::{
    DEFAULT_MAX_PERIOD_FRAMES, DEFAULT_PREFERRED_PERIOD_FRAMES, DEVICE_FRIENDLY_NAME_PROPERTY_KEY,
    ENDPOINT_FORM_FACTOR_PROPERTY_KEY, HRESULT_OK, IID_IAUDIO_CAPTURE_CLIENT, IID_IAUDIO_CLIENT,
    IID_IAUDIO_RENDER_CLIENT, IID_IMM_DEVICE_ENUMERATOR, PROBED_SAMPLE_FORMATS,
    PROBED_SAMPLE_RATES, WASAPI_MAX_PROBED_CHANNELS, WAVE_FORMAT_EXTENSIBLE_EXTRA_BYTES,
    WAVE_FORMAT_EXTENSIBLE_TAG, WAVE_SUBTYPE_IEEE_FLOAT, WAVE_SUBTYPE_PCM,
};
use super::core::{ComApartment, ComPointer, WasapiEndpointProfile, WasapiEventHandle};
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;
use crate::platform::{PlatformError, core as core_platform};

use crate::platform::audio as audio_types;
use windows_sys::Win32::Foundation::{GetLastError, RPC_E_CHANGED_MODE};
use windows_sys::Win32::Media::Audio::{
    AUDCLNT_SHAREMODE_EXCLUSIVE, AUDCLNT_SHAREMODE_SHARED, DigitalAudioDisplayDevice, EDataFlow,
    ERole, Handset, Headphones, Headset, IAudioClient, IMMDevice, IMMDeviceCollection,
    IMMDeviceEnumerator, LineLevel, MMDeviceEnumerator, Microphone, RemoteNetworkDevice, SPDIF,
    Speakers, UnknownDigitalPassthrough, WAVE_FORMAT_PCM, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
    eConsole,
};
use windows_sys::Win32::Media::Multimedia::WAVE_FORMAT_IEEE_FLOAT;
use windows_sys::Win32::System::Com::StructuredStorage::{PROPVARIANT, PropVariantClear};
use windows_sys::Win32::System::Com::{
    CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree, STGM_READ,
};
use windows_sys::Win32::System::Threading::CreateEventW;
use windows_sys::Win32::System::Variant::VT_LPWSTR;
use windows_sys::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY};
use windows_sys::core::{HRESULT, PWSTR};

/// Initialize COM for one WASAPI call site.
pub(super) fn initialize_com() -> RuntimeResult<ComApartment> {
    // initialize one COM apartment for this thread
    let status = unsafe { CoInitializeEx(ptr::null(), COINIT_MULTITHREADED as u32) };
    if succeeded(status) {
        return Ok(ComApartment {
            should_uninitialize: true,
        });
    }

    // reuse one existing apartment in changed-mode contexts
    if status == RPC_E_CHANGED_MODE {
        return Ok(ComApartment {
            should_uninitialize: false,
        });
    }

    Err(hresult_error(
        "destack.audio.internal.com.initialize",
        status,
        "failed to initialize COM apartment",
    ))
}

/// Create one MMDevice endpoint enumerator.
pub(super) fn create_device_enumerator() -> RuntimeResult<ComPointer> {
    // create one MMDevice endpoint enumerator instance
    let mut enumerator_ptr = ptr::null_mut();
    let status = unsafe {
        CoCreateInstance(
            &MMDeviceEnumerator,
            ptr::null_mut(),
            CLSCTX_ALL,
            &IID_IMM_DEVICE_ENUMERATOR,
            &mut enumerator_ptr,
        )
    };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.internal.wasapi.enumerator",
            status,
            "failed to create WASAPI device enumerator",
        ));
    }

    Ok(ComPointer::new(enumerator_ptr))
}

/// Resolve the default endpoint id for one data-flow lane.
pub(super) fn default_endpoint_id(
    enumerator: IMMDeviceEnumerator,
    flow: EDataFlow,
) -> Option<String> {
    let endpoint = get_default_endpoint(enumerator, flow).ok()?;
    endpoint_id(endpoint.raw() as IMMDevice).ok()
}

/// Resolve one endpoint interface by endpoint id.
pub(super) fn get_endpoint_by_id(
    enumerator: IMMDeviceEnumerator,
    endpoint_id: &str,
) -> RuntimeResult<ComPointer> {
    // encode one endpoint id for one COM GetDevice lookup
    let endpoint_id_wide = core_platform::wide_with_nul(endpoint_id);
    let mut endpoint_ptr = ptr::null_mut();
    let status = unsafe {
        imm_device_enumerator_get_device(enumerator, endpoint_id_wide.as_ptr(), &mut endpoint_ptr)
    };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.stream.open",
            status,
            "failed to resolve WASAPI endpoint by id",
        ));
    }

    Ok(ComPointer::new(endpoint_ptr))
}

/// Activate one IAudioClient interface for one endpoint.
pub(super) fn activate_audio_client(device: IMMDevice) -> RuntimeResult<ComPointer> {
    // activate one IAudioClient interface from one endpoint
    let mut audio_client_ptr = ptr::null_mut();
    let status = unsafe {
        imm_device_activate(
            device,
            &IID_IAUDIO_CLIENT,
            CLSCTX_ALL,
            ptr::null(),
            &mut audio_client_ptr,
        )
    };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.stream.open",
            status,
            "failed to activate WASAPI audio client",
        ));
    }

    Ok(ComPointer::new(audio_client_ptr))
}

/// Resolve one IAudioRenderClient service from one audio client.
pub(super) fn get_render_client(audio_client: IAudioClient) -> RuntimeResult<ComPointer> {
    let mut render_client_ptr = ptr::null_mut();
    let status = unsafe {
        audio_client_get_service(
            audio_client,
            &IID_IAUDIO_RENDER_CLIENT,
            &mut render_client_ptr,
        )
    };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.stream.open",
            status,
            "failed to resolve WASAPI render service",
        ));
    }

    Ok(ComPointer::new(render_client_ptr))
}

/// Resolve one IAudioCaptureClient service from one audio client.
pub(super) fn get_capture_client(audio_client: IAudioClient) -> RuntimeResult<ComPointer> {
    let mut capture_client_ptr = ptr::null_mut();
    let status = unsafe {
        audio_client_get_service(
            audio_client,
            &IID_IAUDIO_CAPTURE_CLIENT,
            &mut capture_client_ptr,
        )
    };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.stream.open",
            status,
            "failed to resolve WASAPI capture service",
        ));
    }

    Ok(ComPointer::new(capture_client_ptr))
}

/// Create one auto-reset event handle for event-callback stream signaling.
pub(super) fn create_stream_event_handle() -> RuntimeResult<Arc<WasapiEventHandle>> {
    // create one unnamed auto-reset event in a nonsignaled state
    let event = unsafe { CreateEventW(ptr::null(), 0, 0, ptr::null()) };
    if event == 0 {
        let status = unsafe { GetLastError() };
        return Err(RuntimeError::from(PlatformError::io_with(
            Some(PlatformErrorCode::IoInvalidData),
            None,
            None,
            Some("destack.audio.stream.open".to_string()),
            None,
            format!("failed to create WASAPI event handle (win32 error {status})"),
        ))
        .boxed());
    }

    Ok(Arc::new(WasapiEventHandle { raw: event }))
}

/// Read one endpoint display name from the property store when available.
pub(super) fn endpoint_display_name(endpoint: IMMDevice, fallback_name: &str) -> String {
    let property_store = match open_endpoint_property_store(endpoint) {
        Ok(value) => value,
        Err(_) => return fallback_name.to_string(),
    };

    property_store_string_value(
        property_store.raw() as IPropertyStore,
        &DEVICE_FRIENDLY_NAME_PROPERTY_KEY,
    )
    .unwrap_or_else(|| fallback_name.to_string())
}

/// Probe one endpoint capability profile using native WASAPI queries.
pub(super) fn probe_endpoint_profile(endpoint: IMMDevice) -> WasapiEndpointProfile {
    let fallback = WasapiEndpointProfile::fallback();
    let audio_client = match activate_audio_client(endpoint) {
        Ok(value) => value,
        Err(_) => return fallback,
    };
    let audio_client = audio_client.raw() as IAudioClient;

    let property_store = open_endpoint_property_store(endpoint).ok();
    let endpoint_form_factor = property_store.as_ref().and_then(|property_store| {
        property_store_u32_value(
            property_store.raw() as IPropertyStore,
            &ENDPOINT_FORM_FACTOR_PROPERTY_KEY,
        )
    });
    let transport = endpoint_transport(endpoint_form_factor);

    let mut mix_format_pointer = ptr::null_mut();
    let mix_format_status =
        unsafe { audio_client_get_mix_format(audio_client, &mut mix_format_pointer) };
    if failed(mix_format_status) || mix_format_pointer.is_null() {
        let mut fallback_profile = fallback;
        fallback_profile.transport = transport;
        return fallback_profile;
    }

    let preferred_sample_rate = unsafe { (*mix_format_pointer).nSamplesPerSec.max(1) };
    let preferred_channels = unsafe {
        (*mix_format_pointer)
            .nChannels
            .clamp(1, WASAPI_MAX_PROBED_CHANNELS)
    };
    let preferred_format = sample_format_from_wave_format(mix_format_pointer)
        .unwrap_or(audio_types::AudioSampleFormat::F32);
    let preferred_channel_mask = wave_channel_mask(mix_format_pointer, preferred_channels);
    let preferred_layout = channel_layout(preferred_channels);

    let (preferred_period_frames, min_period_frames, max_period_frames) =
        period_frames_range(audio_client, preferred_sample_rate);

    let sample_rate_range = probe_sample_rate_range(
        audio_client,
        preferred_format,
        preferred_channels,
        preferred_sample_rate,
    );
    let channel_range = probe_channel_range(audio_client, preferred_format, preferred_sample_rate);
    let format_mask = probe_format_mask(
        audio_client,
        preferred_format,
        preferred_channels,
        preferred_sample_rate,
    );
    let supports_exclusive_mode = probe_exclusive_mode_support(
        audio_client,
        preferred_format,
        preferred_channels,
        preferred_sample_rate,
    );

    unsafe {
        CoTaskMemFree(mix_format_pointer as *const c_void);
    }

    let mut profile = WasapiEndpointProfile {
        preferred_sample_rate,
        min_sample_rate: sample_rate_range.0,
        max_sample_rate: sample_rate_range.1,
        preferred_period_frames,
        min_period_frames,
        max_period_frames,
        min_channels: channel_range.0,
        max_channels: channel_range.1,
        preferred_layout,
        preferred_channel_mask,
        supported_channel_mask: channel_mask(channel_range.1),
        format_mask,
        supports_exclusive_mode,
        transport,
    };

    if profile.max_sample_rate < profile.min_sample_rate {
        profile.max_sample_rate = profile.min_sample_rate;
    }

    if profile.max_channels < profile.min_channels {
        profile.max_channels = profile.min_channels;
    }

    if profile.max_period_frames < profile.min_period_frames {
        profile.max_period_frames = profile.min_period_frames;
    }

    if profile.format_mask == 0 {
        profile.format_mask = audio_types::sample_format_bit(preferred_format);
    }

    profile
}

/// Map one endpoint form-factor value into one coarse transport class.
fn endpoint_transport(endpoint_form_factor: Option<u32>) -> &'static str {
    let endpoint_form_factor = match endpoint_form_factor {
        Some(value) => value as i32,
        None => return "wasapi",
    };

    if endpoint_form_factor == Speakers
        || endpoint_form_factor == LineLevel
        || endpoint_form_factor == DigitalAudioDisplayDevice
        || endpoint_form_factor == SPDIF
        || endpoint_form_factor == UnknownDigitalPassthrough
    {
        return "speaker";
    }

    if endpoint_form_factor == Headphones
        || endpoint_form_factor == Headset
        || endpoint_form_factor == Handset
    {
        return "headphone";
    }

    if endpoint_form_factor == Microphone {
        return "microphone";
    }

    if endpoint_form_factor == RemoteNetworkDevice {
        return "network";
    }

    "wasapi"
}

/// Build one native WAVEFORMATEX from one stream config.
pub(super) fn build_wave_format(
    config: audio_types::AudioStreamConfig,
) -> RuntimeResult<WAVEFORMATEX> {
    // map one runtime sample format to one native tag and bits-per-sample
    let (format_tag, bits_per_sample) = match config.format {
        audio_types::AudioSampleFormat::U8 => (
            windows_sys::Win32::Media::Audio::WAVE_FORMAT_PCM as u16,
            8u16,
        ),
        audio_types::AudioSampleFormat::S16 => (
            windows_sys::Win32::Media::Audio::WAVE_FORMAT_PCM as u16,
            16u16,
        ),
        audio_types::AudioSampleFormat::S24 => (
            windows_sys::Win32::Media::Audio::WAVE_FORMAT_PCM as u16,
            24u16,
        ),
        audio_types::AudioSampleFormat::S32 => (
            windows_sys::Win32::Media::Audio::WAVE_FORMAT_PCM as u16,
            32u16,
        ),
        audio_types::AudioSampleFormat::F32 => (WAVE_FORMAT_IEEE_FLOAT as u16, 32u16),
        audio_types::AudioSampleFormat::F64 => {
            return Err(RuntimeError::from(PlatformError::not_supported(
                "destack.audio.stream.open sample format f64 on wasapi",
            ))
            .boxed());
        }
    };

    let bytes_per_sample = (bits_per_sample as u32 / 8).max(1);
    let block_align = config
        .channels
        .checked_mul(bytes_per_sample as u16)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "config.channels",
                "channel count overflows native block alignment",
            ))
            .boxed()
        })?;
    let average_bytes_per_second = config
        .sample_rate
        .checked_mul(block_align as u32)
        .ok_or_else(|| {
            RuntimeError::from(PlatformError::invalid_argument_value(
                "config.sampleRate",
                "sample rate and channel count overrun native byte-rate limits",
            ))
            .boxed()
        })?;

    Ok(WAVEFORMATEX {
        wFormatTag: format_tag,
        nChannels: config.channels,
        nSamplesPerSec: config.sample_rate,
        nAvgBytesPerSec: average_bytes_per_second,
        nBlockAlign: block_align,
        wBitsPerSample: bits_per_sample,
        cbSize: 0,
    })
}

/// Decode one NUL-terminated UTF-16 pointer into one UTF-8 string.
fn string_from_pwstr(pointer: PWSTR) -> String {
    if pointer.is_null() {
        return String::new();
    }

    // scan one NUL-terminated UTF-16 buffer
    let mut length = 0usize;
    unsafe {
        while *pointer.add(length) != 0 {
            length = length.saturating_add(1);
        }
    }

    unsafe { String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length)) }
}

/// Build one runtime I/O error from one HRESULT payload.
pub(super) fn hresult_error(
    operation: &'static str,
    status: HRESULT,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!("{} (hresult 0x{status:08x})", message.into()),
    ))
    .boxed()
}

/// Return whether one HRESULT represents success.
pub(super) fn succeeded(status: HRESULT) -> bool {
    status >= 0
}

/// Return whether one HRESULT represents failure.
pub(super) fn failed(status: HRESULT) -> bool {
    status < 0
}

/// Resolve one endpoint id string from one endpoint interface.
pub(super) fn endpoint_id(device: IMMDevice) -> RuntimeResult<String> {
    // fetch one endpoint id wide string and free COM ownership
    let mut endpoint_id_pointer = ptr::null_mut();
    let status = unsafe { imm_device_get_id(device, &mut endpoint_id_pointer) };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.device.list",
            status,
            "failed to read WASAPI endpoint id",
        ));
    }

    let endpoint_id = string_from_pwstr(endpoint_id_pointer);
    unsafe {
        CoTaskMemFree(endpoint_id_pointer as *const c_void);
    }

    Ok(endpoint_id)
}

/// Resolve the default endpoint interface for one data-flow lane.
pub(super) fn get_default_endpoint(
    enumerator: IMMDeviceEnumerator,
    flow: EDataFlow,
) -> RuntimeResult<ComPointer> {
    let mut endpoint_ptr = ptr::null_mut();
    let status = unsafe {
        imm_device_enumerator_get_default_audio_endpoint(
            enumerator,
            flow,
            eConsole as ERole,
            &mut endpoint_ptr,
        )
    };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.device.default",
            status,
            "failed to read WASAPI default endpoint",
        ));
    }

    Ok(ComPointer::new(endpoint_ptr))
}

/// Enumerate active endpoints for one data-flow lane.
pub(super) fn enum_audio_endpoints(
    enumerator: IMMDeviceEnumerator,
    flow: EDataFlow,
) -> RuntimeResult<ComPointer> {
    let mut collection_ptr = ptr::null_mut();
    let status = unsafe {
        imm_device_enumerator_enum_audio_endpoints(enumerator, flow, &mut collection_ptr)
    };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.device.list",
            status,
            "failed to enumerate WASAPI endpoints",
        ));
    }

    Ok(ComPointer::new(collection_ptr))
}

/// Query endpoint collection count.
pub(super) fn collection_count(collection: IMMDeviceCollection) -> RuntimeResult<u32> {
    let mut count = 0u32;
    let status = unsafe { imm_device_collection_get_count(collection, &mut count) };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.device.list",
            status,
            "failed to query WASAPI endpoint count",
        ));
    }

    Ok(count)
}

/// Resolve one endpoint item from one collection index.
pub(super) fn collection_item(
    collection: IMMDeviceCollection,
    index: u32,
) -> RuntimeResult<ComPointer> {
    let mut endpoint_ptr = ptr::null_mut();
    let status = unsafe { imm_device_collection_item(collection, index, &mut endpoint_ptr) };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.device.list",
            status,
            "failed to query WASAPI endpoint item",
        ));
    }

    Ok(ComPointer::new(endpoint_ptr))
}

/// Open one endpoint property store for read-only property queries.
pub(super) fn open_endpoint_property_store(endpoint: IMMDevice) -> RuntimeResult<ComPointer> {
    let mut property_store_pointer = ptr::null_mut();
    let status =
        unsafe { imm_device_open_property_store(endpoint, STGM_READ, &mut property_store_pointer) };
    if failed(status) {
        return Err(hresult_error(
            "destack.audio.device.list",
            status,
            "failed to open WASAPI endpoint property store",
        ));
    }

    Ok(ComPointer::new(property_store_pointer))
}

/// Read one UTF-16 string property from one endpoint property store.
fn property_store_string_value(
    property_store: IPropertyStore,
    property_key: &PROPERTYKEY,
) -> Option<String> {
    let mut property_value: PROPVARIANT = unsafe { std::mem::zeroed() };
    let status =
        unsafe { property_store_get_value(property_store, property_key, &mut property_value) };
    if failed(status) {
        return None;
    }

    let mut value = None;
    unsafe {
        let variant = property_value.Anonymous.Anonymous;
        if variant.vt == VT_LPWSTR {
            let pointer = variant.Anonymous.pwszVal;
            if !pointer.is_null() {
                value = Some(string_from_pwstr(pointer));
            }
        }
    }

    unsafe {
        let _ = PropVariantClear(&mut property_value);
    }

    value
}

/// Read one `u32` property from one endpoint property store.
fn property_store_u32_value(
    property_store: IPropertyStore,
    property_key: &PROPERTYKEY,
) -> Option<u32> {
    let mut property_value: PROPVARIANT = unsafe { std::mem::zeroed() };
    let status =
        unsafe { property_store_get_value(property_store, property_key, &mut property_value) };
    if failed(status) {
        return None;
    }

    let mut value = None;
    unsafe {
        let variant = property_value.Anonymous.Anonymous;
        if variant.vt == windows_sys::Win32::System::Variant::VT_UI4 {
            value = Some(variant.Anonymous.ulVal);
        }
    }

    unsafe {
        let _ = PropVariantClear(&mut property_value);
    }

    value
}

/// Return one exact-format support decision for one probe wave format.
fn is_exact_format_supported(
    audio_client: IAudioClient,
    share_mode: i32,
    wave_format: &WAVEFORMATEX,
) -> bool {
    let mut closest_match = ptr::null_mut();
    let closest_match_pointer = if share_mode == AUDCLNT_SHAREMODE_SHARED {
        &mut closest_match
    } else {
        ptr::null_mut()
    };

    let status = unsafe {
        audio_client_is_format_supported(
            audio_client,
            share_mode,
            wave_format as *const WAVEFORMATEX,
            closest_match_pointer,
        )
    };

    if !closest_match.is_null() {
        unsafe {
            CoTaskMemFree(closest_match as *const c_void);
        }
    }

    status == HRESULT_OK
}

/// Probe one sample-rate capability range for one endpoint profile.
fn probe_sample_rate_range(
    audio_client: IAudioClient,
    format: audio_types::AudioSampleFormat,
    channels: u16,
    preferred_sample_rate: u32,
) -> (u32, u32) {
    let mut supported_rates = Vec::new();

    // scan one stable set of common sample rates
    for sample_rate in PROBED_SAMPLE_RATES {
        let wave_format = match probe_wave_format(format, channels, sample_rate) {
            Some(value) => value,
            None => continue,
        };

        if is_exact_format_supported(audio_client, AUDCLNT_SHAREMODE_SHARED, &wave_format) {
            supported_rates.push(sample_rate);
        }
    }

    if supported_rates.is_empty() {
        return (preferred_sample_rate, preferred_sample_rate);
    }

    let minimum_sample_rate = supported_rates
        .iter()
        .copied()
        .min()
        .unwrap_or(preferred_sample_rate);
    let maximum_sample_rate = supported_rates
        .iter()
        .copied()
        .max()
        .unwrap_or(preferred_sample_rate);
    (minimum_sample_rate, maximum_sample_rate)
}

/// Probe one channel-count range for one endpoint profile.
fn probe_channel_range(
    audio_client: IAudioClient,
    format: audio_types::AudioSampleFormat,
    preferred_sample_rate: u32,
) -> (u16, u16) {
    let mut supported_channels = Vec::new();

    // scan one bounded channel-count range expected by common engines
    for channels in 1..=WASAPI_MAX_PROBED_CHANNELS {
        let wave_format = match probe_wave_format(format, channels, preferred_sample_rate) {
            Some(value) => value,
            None => continue,
        };

        if is_exact_format_supported(audio_client, AUDCLNT_SHAREMODE_SHARED, &wave_format) {
            supported_channels.push(channels);
        }
    }

    if supported_channels.is_empty() {
        return (1, 2);
    }

    let minimum_channels = supported_channels.iter().copied().min().unwrap_or(1);
    let maximum_channels = supported_channels
        .iter()
        .copied()
        .max()
        .unwrap_or(minimum_channels);
    (minimum_channels, maximum_channels)
}

/// Probe one sample-format mask for one endpoint profile.
fn probe_format_mask(
    audio_client: IAudioClient,
    preferred_format: audio_types::AudioSampleFormat,
    preferred_channels: u16,
    preferred_sample_rate: u32,
) -> u32 {
    let mut format_mask = 0u32;

    // probe one prioritized format set so descriptor rows reflect endpoint reality
    for format in PROBED_SAMPLE_FORMATS {
        let wave_format = match probe_wave_format(format, preferred_channels, preferred_sample_rate)
        {
            Some(value) => value,
            None => continue,
        };

        if is_exact_format_supported(audio_client, AUDCLNT_SHAREMODE_SHARED, &wave_format) {
            format_mask |= audio_types::sample_format_bit(format);
        }
    }

    if format_mask == 0 {
        return audio_types::sample_format_bit(preferred_format);
    }

    format_mask
}

/// Probe whether exclusive mode appears supported for one endpoint profile.
fn probe_exclusive_mode_support(
    audio_client: IAudioClient,
    preferred_format: audio_types::AudioSampleFormat,
    preferred_channels: u16,
    preferred_sample_rate: u32,
) -> bool {
    let preferred_wave_format =
        match probe_wave_format(preferred_format, preferred_channels, preferred_sample_rate) {
            Some(value) => value,
            None => return false,
        };

    if is_exact_format_supported(
        audio_client,
        AUDCLNT_SHAREMODE_EXCLUSIVE,
        &preferred_wave_format,
    ) {
        return true;
    }

    // fall back to a compact candidate set when preferred format is not exact-exclusive
    for format in PROBED_SAMPLE_FORMATS {
        let wave_format = match probe_wave_format(format, 2, 48_000) {
            Some(value) => value,
            None => continue,
        };

        if is_exact_format_supported(audio_client, AUDCLNT_SHAREMODE_EXCLUSIVE, &wave_format) {
            return true;
        }
    }

    false
}

/// Build one probe wave format for one sample format and endpoint lane shape.
fn probe_wave_format(
    format: audio_types::AudioSampleFormat,
    channels: u16,
    sample_rate: u32,
) -> Option<WAVEFORMATEX> {
    if channels == 0 || sample_rate == 0 {
        return None;
    }

    // map one runtime sample format into one native probe waveform shape
    let (format_tag, bits_per_sample) = match format {
        audio_types::AudioSampleFormat::U8 => (WAVE_FORMAT_PCM as u16, 8u16),
        audio_types::AudioSampleFormat::S16 => (WAVE_FORMAT_PCM as u16, 16u16),
        audio_types::AudioSampleFormat::S24 => (WAVE_FORMAT_PCM as u16, 24u16),
        audio_types::AudioSampleFormat::S32 => (WAVE_FORMAT_PCM as u16, 32u16),
        audio_types::AudioSampleFormat::F32 => (WAVE_FORMAT_IEEE_FLOAT as u16, 32u16),
        audio_types::AudioSampleFormat::F64 => (WAVE_FORMAT_IEEE_FLOAT as u16, 64u16),
    };

    let bytes_per_sample = (bits_per_sample / 8).max(1);
    let block_align = channels.checked_mul(bytes_per_sample)?;
    let average_bytes_per_second = sample_rate.checked_mul(block_align as u32)?;

    Some(WAVEFORMATEX {
        wFormatTag: format_tag,
        nChannels: channels,
        nSamplesPerSec: sample_rate,
        nAvgBytesPerSec: average_bytes_per_second,
        nBlockAlign: block_align,
        wBitsPerSample: bits_per_sample,
        cbSize: 0,
    })
}

/// Derive one runtime sample format from one WAVEFORMATEX payload.
fn sample_format_from_wave_format(
    wave_format: *const WAVEFORMATEX,
) -> Option<audio_types::AudioSampleFormat> {
    if wave_format.is_null() {
        return None;
    }

    let wave_format_pointer = wave_format;
    let wave_format = unsafe { &*wave_format_pointer };
    let bits_per_sample = wave_format.wBitsPerSample;

    if wave_format.wFormatTag == WAVE_FORMAT_PCM as u16 {
        return match bits_per_sample {
            8 => Some(audio_types::AudioSampleFormat::U8),
            16 => Some(audio_types::AudioSampleFormat::S16),
            24 => Some(audio_types::AudioSampleFormat::S24),
            32 => Some(audio_types::AudioSampleFormat::S32),
            _ => None,
        };
    }

    if wave_format.wFormatTag == WAVE_FORMAT_IEEE_FLOAT as u16 {
        return match bits_per_sample {
            32 => Some(audio_types::AudioSampleFormat::F32),
            64 => Some(audio_types::AudioSampleFormat::F64),
            _ => None,
        };
    }

    if wave_format.wFormatTag == WAVE_FORMAT_EXTENSIBLE_TAG
        && wave_format.cbSize >= WAVE_FORMAT_EXTENSIBLE_EXTRA_BYTES
    {
        let wave_format_extensible_pointer = wave_format_pointer as *const WAVEFORMATEXTENSIBLE;
        let sub_format =
            unsafe { ptr::addr_of!((*wave_format_extensible_pointer).SubFormat).read_unaligned() };
        if core_platform::com_guid_equals(&sub_format, &WAVE_SUBTYPE_PCM) {
            return match bits_per_sample {
                8 => Some(audio_types::AudioSampleFormat::U8),
                16 => Some(audio_types::AudioSampleFormat::S16),
                24 => Some(audio_types::AudioSampleFormat::S24),
                32 => Some(audio_types::AudioSampleFormat::S32),
                _ => None,
            };
        }

        if core_platform::com_guid_equals(&sub_format, &WAVE_SUBTYPE_IEEE_FLOAT) {
            return match bits_per_sample {
                32 => Some(audio_types::AudioSampleFormat::F32),
                64 => Some(audio_types::AudioSampleFormat::F64),
                _ => None,
            };
        }
    }

    None
}

/// Derive one channel mask from one wave format payload.
fn wave_channel_mask(wave_format: *const WAVEFORMATEX, channels: u16) -> u64 {
    if wave_format.is_null() {
        return channel_mask(channels);
    }

    let wave_format_pointer = wave_format;
    let wave_format = unsafe { &*wave_format_pointer };
    if wave_format.wFormatTag != WAVE_FORMAT_EXTENSIBLE_TAG
        || wave_format.cbSize < WAVE_FORMAT_EXTENSIBLE_EXTRA_BYTES
    {
        return channel_mask(channels);
    }

    let wave_format_extensible_pointer = wave_format_pointer as *const WAVEFORMATEXTENSIBLE;
    let channel_mask_value =
        unsafe { ptr::addr_of!((*wave_format_extensible_pointer).dwChannelMask).read_unaligned() };
    if channel_mask_value == 0 {
        return channel_mask(channels);
    }

    channel_mask_value as u64
}

/// Resolve one period-frame range from one endpoint audio client.
fn period_frames_range(audio_client: IAudioClient, sample_rate: u32) -> (u32, u32, u32) {
    let mut default_period_hns = 0i64;
    let mut minimum_period_hns = 0i64;
    let period_status = unsafe {
        audio_client_get_device_period(
            audio_client,
            &mut default_period_hns,
            &mut minimum_period_hns,
        )
    };
    if failed(period_status) {
        return (
            DEFAULT_PREFERRED_PERIOD_FRAMES,
            audio_core::MIN_STREAM_PERIOD_FRAMES,
            DEFAULT_MAX_PERIOD_FRAMES,
        );
    }

    let preferred_period_frames =
        frames_from_hns(default_period_hns, sample_rate).max(audio_core::MIN_STREAM_PERIOD_FRAMES);
    let minimum_period_frames =
        frames_from_hns(minimum_period_hns, sample_rate).max(audio_core::MIN_STREAM_PERIOD_FRAMES);
    let maximum_period_frames = preferred_period_frames
        .saturating_mul(8)
        .max(DEFAULT_MAX_PERIOD_FRAMES)
        .max(minimum_period_frames);

    (
        preferred_period_frames,
        minimum_period_frames,
        maximum_period_frames,
    )
}

/// Convert one 100ns device period into one frame count.
fn frames_from_hns(period_hns: i64, sample_rate: u32) -> u32 {
    if period_hns <= 0 || sample_rate == 0 {
        return audio_core::MIN_STREAM_PERIOD_FRAMES;
    }

    let numerator = (period_hns as u128).saturating_mul(sample_rate as u128);
    let rounded = numerator.saturating_add(9_999_999u128) / 10_000_000u128;
    rounded
        .max(audio_core::MIN_STREAM_PERIOD_FRAMES as u128)
        .min(u32::MAX as u128) as u32
}

/// Return one canonical channel layout for one channel count.
pub(super) fn channel_layout(channels: u16) -> audio_types::AudioChannelLayout {
    match channels {
        1 => audio_types::AudioChannelLayout::Mono,
        2 => audio_types::AudioChannelLayout::Stereo,
        4 => audio_types::AudioChannelLayout::Quad,
        5 => audio_types::AudioChannelLayout::Surround41,
        6 => audio_types::AudioChannelLayout::Surround51,
        7 => audio_types::AudioChannelLayout::Surround61,
        8 => audio_types::AudioChannelLayout::Surround71,
        _ => audio_types::AudioChannelLayout::Unknown,
    }
}

/// Return one packed channel mask for one channel count.
pub(super) fn channel_mask(channels: u16) -> u64 {
    if channels == 0 {
        return 0;
    }

    if channels >= 64 {
        return u64::MAX;
    }

    (1u64 << channels) - 1
}
