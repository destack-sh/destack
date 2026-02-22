use std::ffi::{c_char, c_int, c_ulong, c_void};

/// One opaque JACK client payload.
#[repr(C)]
pub(super) struct JackClient {
    /// Opaque bytes owned by JACK.
    _opaque: [u8; 0],
}

/// One opaque JACK port payload.
#[repr(C)]
pub(super) struct JackPort {
    /// Opaque bytes owned by JACK.
    _opaque: [u8; 0],
}

/// One JACK loaded symbol table.
#[derive(Debug)]
pub(super) struct JackApi {
    /// `jack_client_open` function pointer.
    pub(super) jack_client_open:
        unsafe extern "C" fn(*const c_char, u32, *mut u32, ...) -> *mut JackClient,
    /// `jack_client_close` function pointer.
    pub(super) jack_client_close: unsafe extern "C" fn(*mut JackClient) -> c_int,
    /// `jack_activate` function pointer.
    pub(super) jack_activate: unsafe extern "C" fn(*mut JackClient) -> c_int,
    /// `jack_deactivate` function pointer.
    pub(super) jack_deactivate: unsafe extern "C" fn(*mut JackClient) -> c_int,
    /// `jack_set_process_callback` function pointer.
    pub(super) jack_set_process_callback: unsafe extern "C" fn(
        *mut JackClient,
        Option<unsafe extern "C" fn(u32, *mut c_void) -> c_int>,
        *mut c_void,
    ) -> c_int,
    /// `jack_set_port_registration_callback` function pointer.
    pub(super) jack_set_port_registration_callback: unsafe extern "C" fn(
        *mut JackClient,
        Option<unsafe extern "C" fn(u32, c_int, *mut c_void)>,
        *mut c_void,
    ) -> c_int,
    /// `jack_set_port_connect_callback` function pointer.
    pub(super) jack_set_port_connect_callback: unsafe extern "C" fn(
        *mut JackClient,
        Option<unsafe extern "C" fn(u32, u32, c_int, *mut c_void)>,
        *mut c_void,
    ) -> c_int,
    /// `jack_on_shutdown` function pointer.
    pub(super) jack_on_shutdown: unsafe extern "C" fn(
        *mut JackClient,
        Option<unsafe extern "C" fn(*mut c_void)>,
        *mut c_void,
    ),
    /// `jack_port_register` function pointer.
    pub(super) jack_port_register: unsafe extern "C" fn(
        *mut JackClient,
        *const c_char,
        *const c_char,
        c_ulong,
        u64,
    ) -> *mut JackPort,
    /// `jack_port_name` function pointer.
    pub(super) jack_port_name: unsafe extern "C" fn(*const JackPort) -> *const c_char,
    /// `jack_port_get_buffer` function pointer.
    pub(super) jack_port_get_buffer: unsafe extern "C" fn(*mut JackPort, u32) -> *mut c_void,
    /// `jack_get_ports` function pointer.
    pub(super) jack_get_ports: unsafe extern "C" fn(
        *mut JackClient,
        *const c_char,
        *const c_char,
        c_ulong,
    ) -> *mut *const c_char,
    /// `jack_connect` function pointer.
    pub(super) jack_connect:
        unsafe extern "C" fn(*mut JackClient, *const c_char, *const c_char) -> c_int,
    /// `jack_get_sample_rate` function pointer.
    pub(super) jack_get_sample_rate: unsafe extern "C" fn(*mut JackClient) -> u32,
    /// `jack_get_buffer_size` function pointer.
    pub(super) jack_get_buffer_size: unsafe extern "C" fn(*mut JackClient) -> u32,
    /// `jack_free` function pointer.
    pub(super) jack_free: unsafe extern "C" fn(*mut c_void),
}
