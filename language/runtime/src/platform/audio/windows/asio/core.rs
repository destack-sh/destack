use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock, Weak};

use super::abi::{
    asio_driver_dispose_buffers, asio_driver_exit, asio_driver_release, asio_driver_start,
    asio_driver_stop,
};
use super::callback;
use super::constants::{
    ASE_OK, ASIO_FALLBACK_MAX_PERIOD_FRAMES, ASIO_FALLBACK_MAX_SAMPLE_RATE,
    ASIO_FALLBACK_MIN_PERIOD_FRAMES, ASIO_FALLBACK_MIN_SAMPLE_RATE,
    ASIO_FALLBACK_PREFERRED_PERIOD_FRAMES,
};
use super::host::enumerate_registered_drivers;

use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::audio::core as audio_core;
use crate::platform::diagnostic::PlatformErrorCode;

use crate::platform::audio as audio_types;
use windows_sys::Win32::Foundation::RPC_E_CHANGED_MODE;
use windows_sys::Win32::System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx, CoUninitialize};
use windows_sys::core::{GUID, HRESULT};

/// Return whether ASIO backend support is implemented for this build.
pub(crate) fn is_backend_supported() -> bool {
    !enumerate_registered_drivers().is_empty()
}

/// Return whether ASIO stream support is implemented for this build.
pub(crate) fn is_stream_supported() -> bool {
    is_backend_supported()
}

/// One initialized COM apartment guard.
#[derive(Debug)]
pub(super) struct ComApartment {
    /// Whether this guard owns one matching uninitialize call.
    should_uninitialize: bool,
}

impl Drop for ComApartment {
    fn drop(&mut self) {
        if !self.should_uninitialize {
            return;
        }

        // pair one successful apartment initialization call
        unsafe {
            CoUninitialize();
        }
    }
}

/// One registry-discovered ASIO driver row.
#[derive(Clone)]
pub(super) struct AsioDriverRow {
    /// Stable registry key name.
    pub(super) key_name: String,
    /// Host-facing display name.
    pub(super) display_name: String,
    /// Driver class identifier.
    pub(super) class_id: GUID,
}

/// One normalized ASIO stream lane selector.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AsioDirectionLane {
    /// One playback lane.
    Playback,
    /// One capture lane.
    Capture,
    /// One duplex lane.
    Duplex,
}

/// One parsed ASIO stable id payload.
#[derive(Debug, Clone)]
pub(super) struct ParsedAsioStableId {
    /// Requested lane encoded in the id prefix.
    pub(super) lane: AsioDirectionLane,
    /// Registry key identity encoded in the id suffix.
    pub(super) key_name: String,
}

/// One probed ASIO endpoint profile.
#[derive(Debug, Clone)]
pub(super) struct AsioDeviceProfile {
    /// Maximum input channel count.
    pub(super) input_channels: u16,
    /// Maximum output channel count.
    pub(super) output_channels: u16,
    /// Preferred sample rate in hertz.
    pub(super) preferred_sample_rate: u32,
    /// Minimum sample rate in hertz.
    pub(super) min_sample_rate: u32,
    /// Maximum sample rate in hertz.
    pub(super) max_sample_rate: u32,
    /// Preferred period in frames.
    pub(super) preferred_period_frames: u32,
    /// Minimum period in frames.
    pub(super) min_period_frames: u32,
    /// Maximum period in frames.
    pub(super) max_period_frames: u32,
    /// Supported sample format mask.
    pub(super) format_mask: u32,
}

impl AsioDeviceProfile {
    /// Build one conservative fallback profile.
    pub(super) fn fallback() -> Self {
        Self {
            input_channels: 2,
            output_channels: 2,
            preferred_sample_rate: 48_000,
            min_sample_rate: ASIO_FALLBACK_MIN_SAMPLE_RATE,
            max_sample_rate: ASIO_FALLBACK_MAX_SAMPLE_RATE,
            preferred_period_frames: ASIO_FALLBACK_PREFERRED_PERIOD_FRAMES,
            min_period_frames: ASIO_FALLBACK_MIN_PERIOD_FRAMES,
            max_period_frames: ASIO_FALLBACK_MAX_PERIOD_FRAMES,
            format_mask: audio_types::sample_format_bit(audio_types::AudioSampleFormat::F32),
        }
    }
}

/// One normalized ASIO lane sample encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct AsioSampleEncoding {
    /// Runtime sample format family.
    pub(super) format: audio_types::AudioSampleFormat,
    /// Bytes used per scalar sample in driver buffers.
    pub(super) bytes_per_sample: usize,
    /// Whether sample byte order is big-endian.
    pub(super) is_big_endian: bool,
    /// Whether this lane uses packed 24-bit samples in 3-byte slots.
    pub(super) is_packed_24: bool,
}

/// One ASIO driver channel descriptor payload.
#[derive(Debug, Clone, Copy)]
pub(super) struct AsioChannelDescriptor {
    /// Channel sample encoding.
    pub(super) encoding: AsioSampleEncoding,
}

/// One resolved ASIO driver interface pointer.
#[derive(Debug)]
pub(super) struct AsioDriverInterface {
    /// Raw IASIO pointer.
    pub(super) raw: *mut c_void,
}

impl Drop for AsioDriverInterface {
    fn drop(&mut self) {
        if self.raw.is_null() {
            return;
        }

        // release one COM interface reference for IASIO
        unsafe {
            asio_driver_release(self.raw);
        }
    }
}

unsafe impl Send for AsioDriverInterface {}
unsafe impl Sync for AsioDriverInterface {}

/// One opened ASIO stream callback buffer lane.
#[derive(Debug, Clone, Copy)]
pub(super) struct AsioBufferLane {
    /// Whether this lane is one input channel.
    pub(super) is_input: bool,
    /// Buffer half `A` pointer.
    pub(super) buffer_a: *mut u8,
    /// Buffer half `B` pointer.
    pub(super) buffer_b: *mut u8,
}

unsafe impl Send for AsioBufferLane {}
unsafe impl Sync for AsioBufferLane {}

/// One opened ASIO session payload.
#[derive(Debug)]
pub(super) struct AsioSession {
    /// Owned IASIO interface pointer.
    pub(super) driver: Arc<AsioDriverInterface>,
    /// Driver display name used in diagnostics.
    pub(super) driver_name: String,
    /// Whether `createBuffers` succeeded for this session.
    pub(super) buffers_created: AtomicBool,
    /// Whether `init` succeeded for this session.
    pub(super) initialized: AtomicBool,
}

impl Drop for AsioSession {
    fn drop(&mut self) {
        // stop one running driver before releasing callback buffers
        unsafe {
            let _ = asio_driver_stop(self.driver.raw);
        }

        // release one created buffer set when present
        if self.buffers_created.load(Ordering::Acquire) {
            unsafe {
                let _ = asio_driver_dispose_buffers(self.driver.raw);
            }
        }

        // terminate one initialized session before interface release
        if self.initialized.load(Ordering::Acquire) {
            unsafe {
                let _ = asio_driver_exit(self.driver.raw);
            }
        }
    }
}

unsafe impl Send for AsioSession {}
unsafe impl Sync for AsioSession {}

/// One runtime payload for one opened ASIO stream.
#[derive(Debug)]
pub(super) struct AsioStreamRuntime {
    /// Effective stream sample rate.
    pub(super) sample_rate: u32,
    /// Effective stream period frames.
    pub(super) period_frames: u32,
    /// Effective stream channel count.
    pub(super) channels: u16,
    /// Output lane encoding.
    pub(super) output_encoding: Option<AsioSampleEncoding>,
    /// Input lane encoding.
    pub(super) input_encoding: Option<AsioSampleEncoding>,
    /// Playback lanes used in callback transfer.
    pub(super) output_lanes: Vec<AsioBufferLane>,
    /// Capture lanes used in callback transfer.
    pub(super) input_lanes: Vec<AsioBufferLane>,
    /// Owned ASIO driver session.
    pub(super) session: Arc<AsioSession>,
    /// Weak link to the stream host state.
    pub(super) stream_state: OnceLock<Weak<audio_core::AudioStreamHostState>>,
}

unsafe impl Send for AsioStreamRuntime {}
unsafe impl Sync for AsioStreamRuntime {}

/// One host stream ops payload for ASIO control calls.
#[derive(Debug)]
pub(super) struct AsioHostStreamOps {
    /// Shared ASIO runtime payload.
    pub(super) runtime: Arc<AsioStreamRuntime>,
}

impl audio_core::AudioHostStreamOps for AsioHostStreamOps {
    fn start(&self) -> RuntimeResult<()> {
        let status = unsafe { asio_driver_start(self.runtime.session.driver.raw) };
        if status != ASE_OK {
            return Err(asio_error(
                "destack.audio.stream.start",
                status,
                &self.runtime.session.driver_name,
                "failed to start ASIO stream",
            ));
        }

        Ok(())
    }

    fn pause(&self, pause: bool) -> RuntimeResult<()> {
        if pause {
            let status = unsafe { asio_driver_stop(self.runtime.session.driver.raw) };
            if status != ASE_OK {
                return Err(asio_error(
                    "destack.audio.stream.pause",
                    status,
                    &self.runtime.session.driver_name,
                    "failed to pause ASIO stream",
                ));
            }

            return Ok(());
        }

        let status = unsafe { asio_driver_start(self.runtime.session.driver.raw) };
        if status != ASE_OK {
            return Err(asio_error(
                "destack.audio.stream.pause",
                status,
                &self.runtime.session.driver_name,
                "failed to resume ASIO stream",
            ));
        }

        Ok(())
    }

    fn stop(&self) -> RuntimeResult<()> {
        let status = unsafe { asio_driver_stop(self.runtime.session.driver.raw) };
        if status != ASE_OK {
            return Err(asio_error(
                "destack.audio.stream.stop",
                status,
                &self.runtime.session.driver_name,
                "failed to stop ASIO stream",
            ));
        }

        Ok(())
    }

    fn flush(&self) -> RuntimeResult<()> {
        let status = unsafe { asio_driver_stop(self.runtime.session.driver.raw) };
        if status != ASE_OK {
            return Err(asio_error(
                "destack.audio.stream.flush",
                status,
                &self.runtime.session.driver_name,
                "failed to stop ASIO stream before flush",
            ));
        }

        Ok(())
    }
}

impl Drop for AsioHostStreamOps {
    fn drop(&mut self) {
        callback::clear_active_runtime(Arc::as_ptr(&self.runtime));
    }
}

/// Build one runtime I/O error from one ASIO status code.
pub(super) fn asio_error(
    operation: &'static str,
    status: i32,
    driver_name: &str,
    message: impl Into<String>,
) -> Box<RuntimeError> {
    RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some(operation.to_string()),
        None,
        format!(
            "{} (driver {driver_name}, status {})",
            message.into(),
            status
        ),
    ))
    .boxed()
}

/// Return whether one HRESULT is successful.
pub(super) fn succeeded(status: HRESULT) -> bool {
    status >= 0
}

/// Return one initialized COM apartment guard.
pub(super) fn initialize_com_apartment() -> RuntimeResult<ComApartment> {
    // initialize one single-thread apartment as expected by ASIO drivers
    let status = unsafe { CoInitializeEx(std::ptr::null(), COINIT_APARTMENTTHREADED as u32) };
    if succeeded(status) {
        return Ok(ComApartment {
            should_uninitialize: true,
        });
    }

    // reuse one already initialized apartment on this thread
    if status == RPC_E_CHANGED_MODE {
        return Ok(ComApartment {
            should_uninitialize: false,
        });
    }

    Err(RuntimeError::from(PlatformError::io_with(
        Some(PlatformErrorCode::IoInvalidData),
        None,
        None,
        Some("destack.audio.internal.asio.com.initialize".to_string()),
        None,
        format!("failed to initialize COM apartment (hresult 0x{status:08x})"),
    ))
    .boxed())
}
