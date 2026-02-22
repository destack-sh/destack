use std::ffi::{c_char, c_int, c_void};

/// One opaque PipeWire simple-stream payload.
#[repr(C)]
pub(super) struct PipewireSimple {
    /// Opaque bytes owned by PipeWire.
    _opaque: [u8; 0],
}

/// One PipeWire sample-spec payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct PipewireSampleSpec {
    /// One `pa_sample_format_t` selector.
    pub(super) format: c_int,
    /// Sample rate in hertz.
    pub(super) rate: u32,
    /// Channel count.
    pub(super) channels: u8,
}

/// One loaded PipeWire symbol table.
#[derive(Debug)]
pub(super) struct PipeWireApi {
    /// `pa_simple_new` function pointer.
    pub(super) pa_simple_new: unsafe extern "C" fn(
        *const c_char,
        *const c_char,
        c_int,
        *const c_char,
        *const c_char,
        *const PipewireSampleSpec,
        *const c_void,
        *const c_void,
        *mut c_int,
    ) -> *mut PipewireSimple,
    /// `pa_simple_free` function pointer.
    pub(super) pa_simple_free: unsafe extern "C" fn(*mut PipewireSimple),
    /// `pa_simple_read` function pointer.
    pub(super) pa_simple_read:
        unsafe extern "C" fn(*mut PipewireSimple, *mut c_void, usize, *mut c_int) -> c_int,
    /// `pa_simple_write` function pointer.
    pub(super) pa_simple_write:
        unsafe extern "C" fn(*mut PipewireSimple, *const c_void, usize, *mut c_int) -> c_int,
    /// `pa_simple_flush` function pointer.
    pub(super) pa_simple_flush: unsafe extern "C" fn(*mut PipewireSimple, *mut c_int) -> c_int,
    /// `pa_simple_drain` function pointer.
    pub(super) pa_simple_drain: unsafe extern "C" fn(*mut PipewireSimple, *mut c_int) -> c_int,
    /// `pa_strerror` function pointer.
    pub(super) pa_strerror: unsafe extern "C" fn(c_int) -> *const c_char,
}
