use std::ffi::{c_char, c_int, c_void};

/// One opaque PulseAudio simple-stream payload.
#[repr(C)]
pub(super) struct PulseSimple {
    /// Opaque bytes owned by PulseAudio.
    _opaque: [u8; 0],
}

/// One PulseAudio sample-spec payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct PulseSampleSpec {
    /// One `pa_sample_format_t` selector.
    pub(super) format: c_int,
    /// Sample rate in hertz.
    pub(super) rate: u32,
    /// Channel count.
    pub(super) channels: u8,
}

/// One loaded PulseAudio symbol table.
#[derive(Debug)]
pub(super) struct PulseAudioApi {
    /// `pa_simple_new` function pointer.
    pub(super) pa_simple_new: unsafe extern "C" fn(
        *const c_char,
        *const c_char,
        c_int,
        *const c_char,
        *const c_char,
        *const PulseSampleSpec,
        *const c_void,
        *const c_void,
        *mut c_int,
    ) -> *mut PulseSimple,
    /// `pa_simple_free` function pointer.
    pub(super) pa_simple_free: unsafe extern "C" fn(*mut PulseSimple),
    /// `pa_simple_read` function pointer.
    pub(super) pa_simple_read:
        unsafe extern "C" fn(*mut PulseSimple, *mut c_void, usize, *mut c_int) -> c_int,
    /// `pa_simple_write` function pointer.
    pub(super) pa_simple_write:
        unsafe extern "C" fn(*mut PulseSimple, *const c_void, usize, *mut c_int) -> c_int,
    /// `pa_simple_flush` function pointer.
    pub(super) pa_simple_flush: unsafe extern "C" fn(*mut PulseSimple, *mut c_int) -> c_int,
    /// `pa_simple_drain` function pointer.
    pub(super) pa_simple_drain: unsafe extern "C" fn(*mut PulseSimple, *mut c_int) -> c_int,
    /// `pa_strerror` function pointer.
    pub(super) pa_strerror: unsafe extern "C" fn(c_int) -> *const c_char,
}
