use std::ffi::c_void;
use std::ptr;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use super::abi::{audio_client_reset, audio_client_start, audio_client_stop, release_com_pointer};
use super::constants::{
    DEFAULT_MAX_PERIOD_FRAMES, DEFAULT_MAX_SAMPLE_RATE, DEFAULT_MIN_SAMPLE_RATE,
    DEFAULT_PREFERRED_PERIOD_FRAMES, HUNDRED_NANOS_PER_SECOND, NANOS_PER_HUNDRED_NANOS,
    WASAPI_MAX_PROBED_CHANNELS,
};
use super::host::{channel_layout, channel_mask, failed, hresult_error, initialize_com};

use crate::diagnostic::RuntimeResult;
use crate::platform::audio::core as audio_core;

use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::Media::Audio::{EDataFlow, IAudioClient, eCapture, eRender};
use windows_sys::Win32::System::Com::CoUninitialize;
use windows_sys::Win32::System::Performance::{QueryPerformanceCounter, QueryPerformanceFrequency};

/// Return whether WASAPI backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    true
}

/// Return whether WASAPI stream support is implemented for this build.
pub(crate) fn is_stream_supported() -> bool {
    true
}

/// One cached QueryPerformanceCounter frequency.
static WASAPI_QPC_FREQUENCY: OnceLock<u64> = OnceLock::new();
/// One stable offset that maps QPC time into runtime monotonic nanoseconds.
static WASAPI_QPC_TO_MONO_OFFSET_NS: OnceLock<i128> = OnceLock::new();

/// Return one cached QueryPerformanceCounter frequency.
pub(super) fn qpc_frequency_hz() -> u64 {
    *WASAPI_QPC_FREQUENCY.get_or_init(|| {
        let mut frequency = 0i64;
        let status = unsafe { QueryPerformanceFrequency(&mut frequency) };
        if status == 0 || frequency <= 0 {
            return 0;
        }

        frequency as u64
    })
}

/// Convert one raw QPC tick value into 100ns units.
pub(super) fn qpc_ticks_to_hundred_nanos(qpc_ticks: u64) -> Option<u64> {
    let frequency = qpc_frequency_hz();
    if frequency == 0 {
        return None;
    }

    Some(
        ((u128::from(qpc_ticks).saturating_mul(HUNDRED_NANOS_PER_SECOND)) / u128::from(frequency))
            as u64,
    )
}

/// Sample current QPC time in 100ns units.
pub(super) fn qpc_now_hundred_nanos() -> Option<u64> {
    let mut counter = 0i64;
    let status = unsafe { QueryPerformanceCounter(&mut counter) };
    if status == 0 || counter < 0 {
        return None;
    }

    qpc_ticks_to_hundred_nanos(counter as u64)
}

/// Convert one WASAPI QPC timestamp in 100ns units into runtime monotonic nanoseconds.
pub(super) fn qpc_hundred_nanos_to_mono_ns(qpc_hundred_nanos: u64) -> Option<u64> {
    let qpc_now_hundred_nanos = qpc_now_hundred_nanos()?;
    let offset = WASAPI_QPC_TO_MONO_OFFSET_NS.get_or_init(|| {
        let mono_now = audio_core::host_monotonic_nanos() as i128;
        let qpc_now_ns =
            (qpc_now_hundred_nanos as i128).saturating_mul(NANOS_PER_HUNDRED_NANOS as i128);
        mono_now - qpc_now_ns
    });

    let qpc_ns = (qpc_hundred_nanos as i128).saturating_mul(NANOS_PER_HUNDRED_NANOS as i128);
    let mapped_ns = qpc_ns + *offset;
    if mapped_ns <= 0 {
        return Some(0);
    }

    Some(mapped_ns.min(u64::MAX as i128) as u64)
}

/// Return one best-effort monotonic timestamp from WASAPI QPC correlation.
pub(super) fn qpc_now_mono_ns() -> u64 {
    if let Some(qpc_now_hundred_nanos) = qpc_now_hundred_nanos()
        && let Some(mapped_ns) = qpc_hundred_nanos_to_mono_ns(qpc_now_hundred_nanos)
    {
        return mapped_ns;
    }

    audio_core::host_monotonic_nanos()
}

/// One endpoint flow lane for stable-id encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EndpointFlow {
    /// One render endpoint lane.
    Render,
    /// One capture endpoint lane.
    Capture,
    /// One loopback endpoint lane over one render endpoint.
    Loopback,
}

impl EndpointFlow {
    /// Return one flow marker used in stable ids.
    pub(super) fn marker(self) -> &'static str {
        match self {
            EndpointFlow::Render => "render",
            EndpointFlow::Capture => "capture",
            EndpointFlow::Loopback => "loopback",
        }
    }

    /// Return one WASAPI data-flow selector.
    pub(super) fn data_flow(self) -> EDataFlow {
        match self {
            EndpointFlow::Render | EndpointFlow::Loopback => eRender,
            EndpointFlow::Capture => eCapture,
        }
    }
}

/// One selected endpoint parsed from one stable id.
#[derive(Debug, Clone)]
pub(super) struct SelectedEndpoint {
    /// The flow lane encoded in the stable id.
    pub(super) flow: EndpointFlow,
    /// The raw WASAPI endpoint id.
    pub(super) endpoint_id: String,
}

/// One probed endpoint capability snapshot used by descriptor normalization.
#[derive(Debug, Clone)]
pub(super) struct WasapiEndpointProfile {
    /// Preferred sample rate in hertz.
    pub(super) preferred_sample_rate: u32,
    /// Minimum supported sample rate in hertz.
    pub(super) min_sample_rate: u32,
    /// Maximum supported sample rate in hertz.
    pub(super) max_sample_rate: u32,
    /// Preferred period in frames.
    pub(super) preferred_period_frames: u32,
    /// Minimum period in frames.
    pub(super) min_period_frames: u32,
    /// Maximum period in frames.
    pub(super) max_period_frames: u32,
    /// Minimum supported channel count.
    pub(super) min_channels: u16,
    /// Maximum supported channel count.
    pub(super) max_channels: u16,
    /// Preferred channel layout.
    pub(super) preferred_layout: audio_core::AudioChannelLayout,
    /// Preferred channel mask.
    pub(super) preferred_channel_mask: u64,
    /// Supported channel mask.
    pub(super) supported_channel_mask: u64,
    /// Supported sample format mask.
    pub(super) format_mask: u32,
    /// Whether exclusive mode appears supported.
    pub(super) supports_exclusive_mode: bool,
    /// The inferred transport classification.
    pub(super) transport: &'static str,
}

impl WasapiEndpointProfile {
    /// Build one conservative fallback endpoint profile.
    pub(super) fn fallback() -> Self {
        let preferred_channel_count = 2u16;
        let preferred_channel_mask = channel_mask(preferred_channel_count);

        Self {
            preferred_sample_rate: 48_000,
            min_sample_rate: DEFAULT_MIN_SAMPLE_RATE,
            max_sample_rate: DEFAULT_MAX_SAMPLE_RATE,
            preferred_period_frames: DEFAULT_PREFERRED_PERIOD_FRAMES,
            min_period_frames: audio_core::MIN_STREAM_PERIOD_FRAMES,
            max_period_frames: DEFAULT_MAX_PERIOD_FRAMES,
            min_channels: 1,
            max_channels: WASAPI_MAX_PROBED_CHANNELS,
            preferred_layout: channel_layout(preferred_channel_count),
            preferred_channel_mask,
            supported_channel_mask: channel_mask(WASAPI_MAX_PROBED_CHANNELS),
            format_mask: audio_core::sample_format_bit(audio_core::AudioSampleFormat::F32),
            supports_exclusive_mode: false,
            transport: "wasapi",
        }
    }
}

/// One opened endpoint runtime payload.
pub(super) struct OpenedWasapiEndpoint {
    /// Endpoint audio client owner.
    pub(super) client: Arc<WasapiAudioClient>,
    /// Endpoint render service when opened in playback mode.
    pub(super) render_client: Option<Arc<WasapiRenderClient>>,
    /// Endpoint capture service when opened in capture mode.
    pub(super) capture_client: Option<Arc<WasapiCaptureClient>>,
    /// Endpoint host buffer size in frames.
    pub(super) buffer_frames: u32,
    /// Endpoint transfer event handle.
    pub(super) event_handle: Arc<WasapiEventHandle>,
}

/// One initialized COM apartment guard.
#[derive(Debug)]
pub(super) struct ComApartment {
    /// Whether this guard owns one matching `CoUninitialize` call.
    pub(super) should_uninitialize: bool,
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if self.should_uninitialize {
            // pair one successful CoInitializeEx call
            unsafe {
                CoUninitialize();
            }
        }
    }
}

/// One owned COM interface pointer.
#[derive(Debug)]
pub(super) struct ComPointer {
    /// The raw COM pointer.
    raw: *mut c_void,
}

impl ComPointer {
    /// Build one owned COM pointer wrapper.
    pub(super) fn new(raw: *mut c_void) -> Self {
        Self { raw }
    }

    /// Return one raw COM pointer.
    pub(super) fn raw(&self) -> *mut c_void {
        self.raw
    }

    /// Consume one wrapper and return the raw pointer.
    pub(super) fn into_raw(mut self) -> *mut c_void {
        let raw = self.raw;
        self.raw = ptr::null_mut();
        raw
    }
}

impl Drop for ComPointer {
    fn drop(&mut self) {
        // release one COM interface pointer when owned
        unsafe {
            release_com_pointer(self.raw);
        }
    }
}

unsafe impl Send for ComPointer {}
unsafe impl Sync for ComPointer {}

/// One WASAPI audio-client owner used by host stream controls.
#[derive(Debug)]
pub(super) struct WasapiAudioClient {
    /// The owned IAudioClient interface pointer.
    pub(super) raw: *mut c_void,
}

impl Drop for WasapiAudioClient {
    fn drop(&mut self) {
        // release one IAudioClient pointer during stream teardown
        unsafe {
            release_com_pointer(self.raw);
        }
    }
}

unsafe impl Send for WasapiAudioClient {}
unsafe impl Sync for WasapiAudioClient {}

/// One WASAPI render-client owner.
#[derive(Debug)]
pub(super) struct WasapiRenderClient {
    /// The owned IAudioRenderClient interface pointer.
    pub(super) raw: *mut c_void,
}

impl Drop for WasapiRenderClient {
    fn drop(&mut self) {
        // release one IAudioRenderClient pointer during stream teardown
        unsafe {
            release_com_pointer(self.raw);
        }
    }
}

unsafe impl Send for WasapiRenderClient {}
unsafe impl Sync for WasapiRenderClient {}

/// One WASAPI capture-client owner.
#[derive(Debug)]
pub(super) struct WasapiCaptureClient {
    /// The owned IAudioCaptureClient interface pointer.
    pub(super) raw: *mut c_void,
}

impl Drop for WasapiCaptureClient {
    fn drop(&mut self) {
        // release one IAudioCaptureClient pointer during stream teardown
        unsafe {
            release_com_pointer(self.raw);
        }
    }
}

unsafe impl Send for WasapiCaptureClient {}
unsafe impl Sync for WasapiCaptureClient {}

/// One owned event handle for event-callback stream signaling.
#[derive(Debug)]
pub(super) struct WasapiEventHandle {
    /// The raw event handle.
    pub(super) raw: HANDLE,
}

impl Drop for WasapiEventHandle {
    fn drop(&mut self) {
        if self.raw == 0 {
            return;
        }

        // close one owned event handle during stream teardown
        unsafe {
            let _ = CloseHandle(self.raw);
        }
    }
}

unsafe impl Send for WasapiEventHandle {}
unsafe impl Sync for WasapiEventHandle {}

/// One runtime payload for one opened WASAPI stream.
#[derive(Debug)]
pub(super) struct WasapiStreamRuntime {
    /// Opened stream direction.
    pub(super) direction: audio_core::AudioDeviceDirection,
    /// Opened stream sample format.
    pub(super) format: audio_core::AudioSampleFormat,
    /// Opened stream sample rate in hertz.
    pub(super) sample_rate: u32,
    /// Opened stream channel count.
    pub(super) channels: u16,
    /// Opened stream period in frames.
    pub(super) period_frames: u32,
    /// Opened stream frame size in bytes.
    pub(super) frame_bytes: usize,
    /// Worker loop cadence for host transfer calls.
    pub(super) poll_period: Duration,
    /// Playback-side host buffer size in frames.
    pub(super) playback_buffer_frames: u32,
    /// Optional playback-side audio client owner.
    pub(super) playback_client: Option<Arc<WasapiAudioClient>>,
    /// Optional capture-side audio client owner.
    pub(super) capture_client_owner: Option<Arc<WasapiAudioClient>>,
    /// Optional render-client owner for playback lanes.
    pub(super) render_client: Option<Arc<WasapiRenderClient>>,
    /// Optional capture-client owner for capture lanes.
    pub(super) capture_client: Option<Arc<WasapiCaptureClient>>,
    /// Optional playback transfer event handle.
    pub(super) playback_event: Option<Arc<WasapiEventHandle>>,
    /// Optional capture transfer event handle.
    pub(super) capture_event: Option<Arc<WasapiEventHandle>>,
}

/// One WASAPI host stream operation payload.
#[derive(Debug)]
pub(super) struct WasapiHostStreamOps {
    /// The owned audio-client interfaces for transport control calls.
    pub(super) clients: Vec<Arc<WasapiAudioClient>>,
}

impl audio_core::AudioHostStreamOps for WasapiHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        let _com = initialize_com()?;
        for client in &self.clients {
            let status = unsafe { audio_client_start(client.raw as IAudioClient) };
            if failed(status) {
                return Err(hresult_error(
                    "destack.audio.stream.start",
                    status,
                    "failed to start WASAPI stream",
                ));
            }
        }

        Ok(())
    }

    fn pause(&self, pause: bool) -> RuntimeResult<()> {
        let _com = initialize_com()?;
        for client in &self.clients {
            let status = if pause {
                unsafe { audio_client_stop(client.raw as IAudioClient) }
            } else {
                unsafe { audio_client_start(client.raw as IAudioClient) }
            };
            if failed(status) {
                return Err(hresult_error(
                    "destack.audio.stream.pause",
                    status,
                    "failed to change WASAPI stream pause state",
                ));
            }
        }

        Ok(())
    }

    fn stop(&self) -> RuntimeResult<()> {
        let _com = initialize_com()?;
        for client in &self.clients {
            let status = unsafe { audio_client_stop(client.raw as IAudioClient) };
            if failed(status) {
                return Err(hresult_error(
                    "destack.audio.stream.stop",
                    status,
                    "failed to stop WASAPI stream",
                ));
            }
        }

        Ok(())
    }

    fn flush(&self) -> RuntimeResult<()> {
        let _com = initialize_com()?;
        for client in &self.clients {
            let stop_status = unsafe { audio_client_stop(client.raw as IAudioClient) };
            if failed(stop_status) {
                return Err(hresult_error(
                    "destack.audio.stream.flush",
                    stop_status,
                    "failed to stop WASAPI stream before reset",
                ));
            }

            let reset_status = unsafe { audio_client_reset(client.raw as IAudioClient) };
            if failed(reset_status) {
                return Err(hresult_error(
                    "destack.audio.stream.flush",
                    reset_status,
                    "failed to reset WASAPI stream",
                ));
            }
        }

        Ok(())
    }
}
