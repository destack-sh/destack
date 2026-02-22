use std::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

/// One opaque ALSA PCM handle payload.
#[repr(C)]
pub(super) struct AlsaPcm {
    /// Opaque bytes owned by ALSA.
    _opaque: [u8; 0],
}

/// One opaque ALSA hardware-params payload.
#[repr(C)]
pub(super) struct AlsaHardwareParams {
    /// Opaque bytes owned by ALSA.
    _opaque: [u8; 0],
}

/// One ALSA unsigned frame-count type.
pub(super) type AlsaUnsignedFrames = c_ulong;
/// One ALSA signed frame-count type.
pub(super) type AlsaSignedFrames = c_long;

/// One loaded ALSA symbol table.
#[derive(Debug)]
pub(super) struct AlsaApi {
    /// `snd_strerror` function pointer.
    pub(super) snd_strerror: unsafe extern "C" fn(c_int) -> *const c_char,
    /// `snd_device_name_hint` function pointer.
    pub(super) snd_device_name_hint:
        unsafe extern "C" fn(c_int, *const c_char, *mut *mut *mut c_void) -> c_int,
    /// `snd_device_name_get_hint` function pointer.
    pub(super) snd_device_name_get_hint:
        unsafe extern "C" fn(*const c_void, *const c_char) -> *mut c_char,
    /// `snd_device_name_free_hint` function pointer.
    pub(super) snd_device_name_free_hint: unsafe extern "C" fn(*mut *mut c_void) -> c_int,
    /// `snd_pcm_open` function pointer.
    pub(super) snd_pcm_open:
        unsafe extern "C" fn(*mut *mut AlsaPcm, *const c_char, c_int, c_int) -> c_int,
    /// `snd_pcm_close` function pointer.
    pub(super) snd_pcm_close: unsafe extern "C" fn(*mut AlsaPcm) -> c_int,
    /// `snd_pcm_nonblock` function pointer.
    pub(super) snd_pcm_nonblock: unsafe extern "C" fn(*mut AlsaPcm, c_int) -> c_int,
    /// `snd_pcm_hw_params_malloc` function pointer.
    pub(super) snd_pcm_hw_params_malloc:
        unsafe extern "C" fn(*mut *mut AlsaHardwareParams) -> c_int,
    /// `snd_pcm_hw_params_free` function pointer.
    pub(super) snd_pcm_hw_params_free: unsafe extern "C" fn(*mut AlsaHardwareParams),
    /// `snd_pcm_hw_params_any` function pointer.
    pub(super) snd_pcm_hw_params_any:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams) -> c_int,
    /// `snd_pcm_hw_params_current` function pointer.
    pub(super) snd_pcm_hw_params_current:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams) -> c_int,
    /// `snd_pcm_hw_params` function pointer.
    pub(super) snd_pcm_hw_params:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams) -> c_int,
    /// `snd_pcm_hw_params_set_access` function pointer.
    pub(super) snd_pcm_hw_params_set_access:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_int) -> c_int,
    /// `snd_pcm_hw_params_set_format` function pointer.
    pub(super) snd_pcm_hw_params_set_format:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_int) -> c_int,
    /// `snd_pcm_hw_params_set_channels_near` function pointer.
    pub(super) snd_pcm_hw_params_set_channels_near:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, *mut c_uint) -> c_int,
    /// `snd_pcm_hw_params_set_rate_near` function pointer.
    pub(super) snd_pcm_hw_params_set_rate_near: unsafe extern "C" fn(
        *mut AlsaPcm,
        *mut AlsaHardwareParams,
        *mut c_uint,
        *mut c_int,
    ) -> c_int,
    /// `snd_pcm_hw_params_set_rate_resample` function pointer.
    pub(super) snd_pcm_hw_params_set_rate_resample:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_uint) -> c_int,
    /// `snd_pcm_hw_params_set_period_size_near` function pointer.
    pub(super) snd_pcm_hw_params_set_period_size_near: unsafe extern "C" fn(
        *mut AlsaPcm,
        *mut AlsaHardwareParams,
        *mut AlsaUnsignedFrames,
        *mut c_int,
    ) -> c_int,
    /// `snd_pcm_hw_params_set_buffer_size_near` function pointer.
    pub(super) snd_pcm_hw_params_set_buffer_size_near: unsafe extern "C" fn(
        *mut AlsaPcm,
        *mut AlsaHardwareParams,
        *mut AlsaUnsignedFrames,
    ) -> c_int,
    /// `snd_pcm_hw_params_get_rate` function pointer.
    pub(super) snd_pcm_hw_params_get_rate:
        unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint, *mut c_int) -> c_int,
    /// `snd_pcm_hw_params_get_rate_min` function pointer.
    pub(super) snd_pcm_hw_params_get_rate_min:
        unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint, *mut c_int) -> c_int,
    /// `snd_pcm_hw_params_get_rate_max` function pointer.
    pub(super) snd_pcm_hw_params_get_rate_max:
        unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint, *mut c_int) -> c_int,
    /// `snd_pcm_hw_params_get_channels` function pointer.
    pub(super) snd_pcm_hw_params_get_channels:
        unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint) -> c_int,
    /// `snd_pcm_hw_params_get_channels_min` function pointer.
    pub(super) snd_pcm_hw_params_get_channels_min:
        unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint) -> c_int,
    /// `snd_pcm_hw_params_get_channels_max` function pointer.
    pub(super) snd_pcm_hw_params_get_channels_max:
        unsafe extern "C" fn(*const AlsaHardwareParams, *mut c_uint) -> c_int,
    /// `snd_pcm_hw_params_get_period_size` function pointer.
    pub(super) snd_pcm_hw_params_get_period_size: unsafe extern "C" fn(
        *const AlsaHardwareParams,
        *mut AlsaUnsignedFrames,
        *mut c_int,
    ) -> c_int,
    /// `snd_pcm_hw_params_get_period_size_min` function pointer.
    pub(super) snd_pcm_hw_params_get_period_size_min: unsafe extern "C" fn(
        *const AlsaHardwareParams,
        *mut AlsaUnsignedFrames,
        *mut c_int,
    ) -> c_int,
    /// `snd_pcm_hw_params_get_period_size_max` function pointer.
    pub(super) snd_pcm_hw_params_get_period_size_max: unsafe extern "C" fn(
        *const AlsaHardwareParams,
        *mut AlsaUnsignedFrames,
        *mut c_int,
    ) -> c_int,
    /// `snd_pcm_hw_params_test_rate` function pointer.
    pub(super) snd_pcm_hw_params_test_rate:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_uint, c_int) -> c_int,
    /// `snd_pcm_hw_params_test_format` function pointer.
    pub(super) snd_pcm_hw_params_test_format:
        unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaHardwareParams, c_int) -> c_int,
    /// `snd_pcm_hw_params_can_pause` function pointer.
    pub(super) snd_pcm_hw_params_can_pause:
        unsafe extern "C" fn(*const AlsaHardwareParams) -> c_int,
    /// `snd_pcm_format_value` function pointer.
    pub(super) snd_pcm_format_value: unsafe extern "C" fn(*const c_char) -> c_int,
    /// `snd_pcm_prepare` function pointer.
    pub(super) snd_pcm_prepare: unsafe extern "C" fn(*mut AlsaPcm) -> c_int,
    /// `snd_pcm_start` function pointer.
    pub(super) snd_pcm_start: unsafe extern "C" fn(*mut AlsaPcm) -> c_int,
    /// `snd_pcm_pause` function pointer.
    pub(super) snd_pcm_pause: unsafe extern "C" fn(*mut AlsaPcm, c_int) -> c_int,
    /// `snd_pcm_drop` function pointer.
    pub(super) snd_pcm_drop: unsafe extern "C" fn(*mut AlsaPcm) -> c_int,
    /// `snd_pcm_reset` function pointer.
    pub(super) snd_pcm_reset: unsafe extern "C" fn(*mut AlsaPcm) -> c_int,
    /// `snd_pcm_recover` function pointer.
    pub(super) snd_pcm_recover: unsafe extern "C" fn(*mut AlsaPcm, c_int, c_int) -> c_int,
    /// `snd_pcm_wait` function pointer.
    pub(super) snd_pcm_wait: unsafe extern "C" fn(*mut AlsaPcm, c_int) -> c_int,
    /// `snd_pcm_avail_update` function pointer.
    pub(super) snd_pcm_avail_update: unsafe extern "C" fn(*mut AlsaPcm) -> AlsaSignedFrames,
    /// `snd_pcm_delay` function pointer.
    pub(super) snd_pcm_delay: unsafe extern "C" fn(*mut AlsaPcm, *mut AlsaSignedFrames) -> c_int,
    /// `snd_pcm_readi` function pointer.
    pub(super) snd_pcm_readi:
        unsafe extern "C" fn(*mut AlsaPcm, *mut c_void, AlsaUnsignedFrames) -> AlsaSignedFrames,
    /// `snd_pcm_writei` function pointer.
    pub(super) snd_pcm_writei:
        unsafe extern "C" fn(*mut AlsaPcm, *const c_void, AlsaUnsignedFrames) -> AlsaSignedFrames,
    /// `snd_pcm_type` function pointer.
    pub(super) snd_pcm_type: unsafe extern "C" fn(*mut AlsaPcm) -> c_int,
}
