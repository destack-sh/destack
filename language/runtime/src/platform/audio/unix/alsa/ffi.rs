use std::ffi::{c_char, c_int, c_uint, c_void};
use std::sync::Arc;

use crate::platform::core as core_platform;

use super::abi::{AlsaApi, AlsaHardwareParams, AlsaPcm, AlsaSignedFrames, AlsaUnsignedFrames};
use super::core::{ALSA_LIBRARY_SLOT, AlsaLibrary};

/// Return one loaded ALSA dynamic-library handle when available.
pub(super) fn alsa_library() -> Option<&'static Arc<AlsaLibrary>> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    ALSA_LIBRARY_SLOT
        .get_or_init(initialize_alsa_library)
        .as_ref()
}

/// Initialize one ALSA dynamic library and symbol table.
fn initialize_alsa_library() -> Option<Arc<AlsaLibrary>> {
    let library_names = ["libasound.so.2", "libasound.so"];

    for library_name in library_names {
        let Some(library) = load_alsa_library_by_name(library_name) else {
            continue;
        };

        return Some(Arc::new(library));
    }

    None
}

/// Load one ALSA dynamic library by one candidate filename.
fn load_alsa_library_by_name(library_name: &str) -> Option<AlsaLibrary> {
    // open one dynamic-library handle for ALSA host functions
    let handle = core_platform::open_dynamic_library(library_name)?;

    // resolve one complete ALSA symbol table from one loaded library
    let api = unsafe {
        match load_alsa_api(handle) {
            Some(api) => api,
            None => {
                core_platform::close_dynamic_library(handle);
                return None;
            }
        }
    };

    Some(AlsaLibrary { handle, api })
}

/// Load one typed ALSA symbol from one open dynamic-library handle.
unsafe fn load_symbol<T>(handle: *mut c_void, symbol: &str) -> Option<T>
where
    T: Copy,
{
    core_platform::load_dynamic_symbol_named(handle, symbol)
}

/// Load one complete ALSA symbol table.
unsafe fn load_alsa_api(handle: *mut c_void) -> Option<AlsaApi> {
    macro_rules! load {
        ($name:literal, $type:ty) => {{
            // load one required ALSA symbol
            let symbol = unsafe { load_symbol::<$type>(handle, $name) };
            symbol?
        }};
    }

    Some(AlsaApi {
        snd_strerror: load!("snd_strerror", unsafe extern "C" fn(c_int) -> *const c_char),
        snd_device_name_hint: load!(
            "snd_device_name_hint",
            unsafe extern "C" fn(c_int, *const c_char, *mut *mut *mut c_void) -> c_int
        ),
        snd_device_name_get_hint: load!(
            "snd_device_name_get_hint",
            unsafe extern "C" fn(*const c_void, *const c_char) -> *mut c_char
        ),
        snd_device_name_free_hint: load!(
            "snd_device_name_free_hint",
            unsafe extern "C" fn(*mut *mut c_void) -> c_int
        ),
        snd_pcm_open: load!(
            "snd_pcm_open",
            unsafe extern "C" fn(*mut *mut AlsaPcm, *const c_char, c_int, c_int) -> c_int
        ),
        snd_pcm_close: load!("snd_pcm_close", unsafe extern "C" fn(*mut AlsaPcm) -> c_int),
        snd_pcm_nonblock: load!(
            "snd_pcm_nonblock",
            unsafe extern "C" fn(*mut AlsaPcm, c_int) -> c_int
        ),
        snd_pcm_hw_params_malloc: load!(
            "snd_pcm_hw_params_malloc",
            unsafe extern "C" fn(*mut *mut AlsaHardwareParams) -> c_int
        ),
        snd_pcm_hw_params_free: load!(
            "snd_pcm_hw_params_free",
            unsafe extern "C" fn(*mut AlsaHardwareParams)
        ),
        snd_pcm_hw_params_any: load!(
            "snd_pcm_hw_params_any",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams) -> c_int
        ),
        snd_pcm_hw_params_current: load!(
            "snd_pcm_hw_params_current",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams) -> c_int
        ),
        snd_pcm_hw_params: load!(
            "snd_pcm_hw_params",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams) -> c_int
        ),
        snd_pcm_hw_params_set_access: load!(
            "snd_pcm_hw_params_set_access",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_int) -> c_int
        ),
        snd_pcm_hw_params_set_format: load!(
            "snd_pcm_hw_params_set_format",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_int) -> c_int
        ),
        snd_pcm_hw_params_set_channels_near: load!(
            "snd_pcm_hw_params_set_channels_near",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, *mut c_uint) -> c_int
        ),
        snd_pcm_hw_params_set_rate_near: load!(
            "snd_pcm_hw_params_set_rate_near",
            unsafe extern "C" fn(
                *mut AlsaPcm,
                *mut AlsaHardwareParams,
                *mut c_uint,
                *mut c_int,
            ) -> c_int
        ),
        snd_pcm_hw_params_set_rate_resample: load!(
            "snd_pcm_hw_params_set_rate_resample",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_uint) -> c_int
        ),
        snd_pcm_hw_params_set_period_size_near: load!(
            "snd_pcm_hw_params_set_period_size_near",
            unsafe extern "C" fn(
                *mut AlsaPcm,
                *mut AlsaHardwareParams,
                *mut AlsaUnsignedFrames,
                *mut c_int,
            ) -> c_int
        ),
        snd_pcm_hw_params_set_buffer_size_near: load!(
            "snd_pcm_hw_params_set_buffer_size_near",
            unsafe extern "C" fn(
                *mut AlsaPcm,
                *mut AlsaHardwareParams,
                *mut AlsaUnsignedFrames,
            ) -> c_int
        ),
        snd_pcm_hw_params_get_rate: load!(
            "snd_pcm_hw_params_get_rate",
            unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint, *mut c_int) -> c_int
        ),
        snd_pcm_hw_params_get_rate_min: load!(
            "snd_pcm_hw_params_get_rate_min",
            unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint, *mut c_int) -> c_int
        ),
        snd_pcm_hw_params_get_rate_max: load!(
            "snd_pcm_hw_params_get_rate_max",
            unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint, *mut c_int) -> c_int
        ),
        snd_pcm_hw_params_get_channels: load!(
            "snd_pcm_hw_params_get_channels",
            unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint) -> c_int
        ),
        snd_pcm_hw_params_get_channels_min: load!(
            "snd_pcm_hw_params_get_channels_min",
            unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint) -> c_int
        ),
        snd_pcm_hw_params_get_channels_max: load!(
            "snd_pcm_hw_params_get_channels_max",
            unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint) -> c_int
        ),
        snd_pcm_hw_params_get_period_size: load!(
            "snd_pcm_hw_params_get_period_size",
            unsafe extern "C" fn(
                *const AlsaHardwareParams,
                *mut AlsaUnsignedFrames,
                *mut c_int,
            ) -> c_int
        ),
        snd_pcm_hw_params_get_period_size_min: load!(
            "snd_pcm_hw_params_get_period_size_min",
            unsafe extern "C" fn(
                *const AlsaHardwareParams,
                *mut AlsaUnsignedFrames,
                *mut c_int,
            ) -> c_int
        ),
        snd_pcm_hw_params_get_period_size_max: load!(
            "snd_pcm_hw_params_get_period_size_max",
            unsafe extern "C" fn(
                *const AlsaHardwareParams,
                *mut AlsaUnsignedFrames,
                *mut c_int,
            ) -> c_int
        ),
        snd_pcm_hw_params_test_rate: load!(
            "snd_pcm_hw_params_test_rate",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_uint, c_int) -> c_int
        ),
        snd_pcm_hw_params_test_format: load!(
            "snd_pcm_hw_params_test_format",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_int) -> c_int
        ),
        snd_pcm_hw_params_can_pause: load!(
            "snd_pcm_hw_params_can_pause",
            unsafe extern "C" fn(*const AlsaHardwareParams) -> c_int
        ),
        snd_pcm_format_value: load!(
            "snd_pcm_format_value",
            unsafe extern "C" fn(*const c_char) -> c_int
        ),
        snd_pcm_prepare: load!(
            "snd_pcm_prepare",
            unsafe extern "C" fn(*mut AlsaPcm) -> c_int
        ),
        snd_pcm_start: load!("snd_pcm_start", unsafe extern "C" fn(*mut AlsaPcm) -> c_int),
        snd_pcm_pause: load!(
            "snd_pcm_pause",
            unsafe extern "C" fn(*mut AlsaPcm, c_int) -> c_int
        ),
        snd_pcm_drop: load!("snd_pcm_drop", unsafe extern "C" fn(*mut AlsaPcm) -> c_int),
        snd_pcm_reset: load!("snd_pcm_reset", unsafe extern "C" fn(*mut AlsaPcm) -> c_int),
        snd_pcm_recover: load!(
            "snd_pcm_recover",
            unsafe extern "C" fn(*mut AlsaPcm, c_int, c_int) -> c_int
        ),
        snd_pcm_wait: load!(
            "snd_pcm_wait",
            unsafe extern "C" fn(*mut AlsaPcm, c_int) -> c_int
        ),
        snd_pcm_avail_update: load!(
            "snd_pcm_avail_update",
            unsafe extern "C" fn(*mut AlsaPcm) -> AlsaSignedFrames
        ),
        snd_pcm_delay: load!(
            "snd_pcm_delay",
            unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaSignedFrames) -> c_int
        ),
        snd_pcm_readi: load!(
            "snd_pcm_readi",
            unsafe extern "C" fn(*mut AlsaPcm, *mut c_void, AlsaUnsignedFrames) -> AlsaSignedFrames
        ),
        snd_pcm_writei: load!(
            "snd_pcm_writei",
            unsafe extern "C" fn(
                *mut AlsaPcm,
                *const c_void,
                AlsaUnsignedFrames,
            ) -> AlsaSignedFrames
        ),
        snd_pcm_type: load!("snd_pcm_type", unsafe extern "C" fn(*mut AlsaPcm) -> c_int),
    })
}
