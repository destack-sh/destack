use std::ffi::{c_char, c_int, c_uint, c_ulong, c_void};

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

/// One JACK MIDI event payload.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub(super) struct JackMidiEvent {
    /// Event frame offset within the current process block.
    pub(super) time: u32,
    /// Event payload length in bytes.
    pub(super) size: usize,
    /// Event payload pointer.
    pub(super) buffer: *mut u8,
}

/// One loaded JACK symbol table.
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
        c_ulong,
    ) -> *mut JackPort,
    /// `jack_port_unregister` function pointer.
    pub(super) jack_port_unregister: unsafe extern "C" fn(*mut JackClient, *mut JackPort) -> c_int,
    /// `jack_port_name` function pointer.
    pub(super) jack_port_name: unsafe extern "C" fn(*const JackPort) -> *const c_char,
    /// `jack_port_by_name` function pointer.
    pub(super) jack_port_by_name:
        unsafe extern "C" fn(*mut JackClient, *const c_char) -> *mut JackPort,
    /// `jack_port_flags` function pointer.
    pub(super) jack_port_flags: unsafe extern "C" fn(*const JackPort) -> c_ulong,
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
    /// `jack_get_cycle_times` function pointer.
    pub(super) jack_get_cycle_times:
        unsafe extern "C" fn(*const JackClient, *mut u32, *mut u64, *mut u64, *mut f32) -> c_int,
    /// `jack_get_time` function pointer.
    pub(super) jack_get_time: unsafe extern "C" fn() -> u64,
    /// `jack_free` function pointer.
    pub(super) jack_free: unsafe extern "C" fn(*mut c_void),
    /// `jack_midi_get_event_count` function pointer.
    pub(super) jack_midi_get_event_count: unsafe extern "C" fn(*mut c_void) -> c_uint,
    /// `jack_midi_event_get` function pointer.
    pub(super) jack_midi_event_get:
        unsafe extern "C" fn(*mut JackMidiEvent, *mut c_void, c_uint) -> c_int,
    /// `jack_midi_clear_buffer` function pointer.
    pub(super) jack_midi_clear_buffer: unsafe extern "C" fn(*mut c_void),
    /// `jack_midi_event_write` function pointer.
    pub(super) jack_midi_event_write:
        unsafe extern "C" fn(*mut c_void, u32, *const u8, usize) -> c_int,
}
