use std::ffi::{c_char, c_int, c_long, c_short, c_uchar, c_uint, c_ulong, c_void};

/// Open the ALSA sequencer for input.
pub(super) const SND_SEQ_OPEN_INPUT: c_int = 1;
/// Open the ALSA sequencer for duplex access.
pub(super) const SND_SEQ_OPEN_DUPLEX: c_int = 3;

/// Mark one direct queue target for immediate delivery.
pub(super) const SND_SEQ_QUEUE_DIRECT: c_uchar = 253;

/// Route one event to all current subscribers.
pub(super) const SND_SEQ_ADDRESS_SUBSCRIBERS: c_uchar = 254;
/// Use one realtime timestamp payload.
pub(super) const SND_SEQ_TIME_STAMP_REAL: c_uchar = 1 << 0;
/// Use one relative timestamp payload.
pub(super) const SND_SEQ_TIME_MODE_REL: c_uchar = 1 << 1;

/// Start one ALSA queue.
pub(super) const SND_SEQ_EVENT_START: c_int = 30;

/// The ALSA system client id.
pub(super) const SND_SEQ_CLIENT_SYSTEM: c_int = 0;
/// The ALSA system announce port id.
pub(super) const SND_SEQ_PORT_SYSTEM_ANNOUNCE: c_int = 1;

/// Allow one port to act as one readable event source.
pub(super) const SND_SEQ_PORT_CAP_READ: c_uint = 1 << 0;
/// Allow one port to act as one writable event destination.
pub(super) const SND_SEQ_PORT_CAP_WRITE: c_uint = 1 << 1;
/// Allow one source port to accept subscriptions.
pub(super) const SND_SEQ_PORT_CAP_SUBS_READ: c_uint = 1 << 5;
/// Allow one destination port to accept subscriptions.
pub(super) const SND_SEQ_PORT_CAP_SUBS_WRITE: c_uint = 1 << 6;
/// Hide one internal port from exported topology.
pub(super) const SND_SEQ_PORT_CAP_NO_EXPORT: c_uint = 1 << 7;

/// Mark one application owned port.
pub(super) const SND_SEQ_PORT_TYPE_APPLICATION: c_uint = 1 << 20;

/// One poll input bit.
pub(super) const POLLIN: c_short = 0x0001;

/// One opaque ALSA sequencer handle.
#[repr(C)]
pub(super) struct snd_seq_t {
    _opaque: [u8; 0],
}

/// One opaque ALSA client-info payload.
#[repr(C)]
pub(super) struct snd_seq_client_info_t {
    _opaque: [u8; 0],
}

/// One opaque ALSA port-info payload.
#[repr(C)]
pub(super) struct snd_seq_port_info_t {
    _opaque: [u8; 0],
}

/// One opaque ALSA port-subscribe payload.
#[repr(C)]
pub(super) struct snd_seq_port_subscribe_t {
    _opaque: [u8; 0],
}

/// One opaque ALSA MIDI parser payload.
#[repr(C)]
pub(super) struct snd_midi_event_t {
    _opaque: [u8; 0],
}

/// One ALSA endpoint address.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct snd_seq_addr_t {
    /// The client id.
    pub(super) client: c_uchar,
    /// The port id.
    pub(super) port: c_uchar,
}

/// One ALSA realtime timestamp.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct snd_seq_real_time_t {
    /// Seconds since the queue epoch.
    pub(super) tv_sec: c_uint,
    /// Nanoseconds inside the current second.
    pub(super) tv_nsec: c_uint,
}

/// One ALSA timestamp payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) union snd_seq_timestamp_t {
    /// Tick timestamp.
    pub(super) tick: c_uint,
    /// Realtime timestamp.
    pub(super) time: snd_seq_real_time_t,
}

/// One ALSA time selector.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct snd_seq_time_t {
    /// Time mode flags.
    pub(super) mode: c_uchar,
    /// Reserved bytes.
    pub(super) _reserved: [c_uchar; 3],
    /// Timestamp payload.
    pub(super) timestamp: snd_seq_timestamp_t,
}

/// One ALSA variable-length event payload.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct snd_seq_ev_ext_t {
    /// The variable payload length.
    pub(super) len: c_uint,
    /// The variable payload bytes.
    pub(super) ptr: *mut c_void,
}

/// One ALSA event payload union.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) union snd_seq_event_data_t {
    /// Variable-length payload data.
    pub(super) ext: snd_seq_ev_ext_t,
    /// Raw words large enough for the fixed payload cases we do not inspect directly.
    pub(super) raw: [u32; 12],
}

/// One ALSA sequencer event.
#[repr(C)]
#[derive(Clone, Copy)]
pub(super) struct snd_seq_event_t {
    /// The event type.
    pub(super) type_: c_uchar,
    /// Event flags.
    pub(super) flags: c_uchar,
    /// Event tag.
    pub(super) tag: c_uchar,
    /// Target queue id.
    pub(super) queue: c_uchar,
    /// Event timestamp.
    pub(super) time: snd_seq_time_t,
    /// Source endpoint.
    pub(super) source: snd_seq_addr_t,
    /// Destination endpoint.
    pub(super) dest: snd_seq_addr_t,
    /// Event payload.
    pub(super) data: snd_seq_event_data_t,
}

/// One POSIX pollfd payload.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct pollfd {
    /// The watched file descriptor.
    pub(super) fd: c_int,
    /// The requested events.
    pub(super) events: c_short,
    /// The returned events.
    pub(super) revents: c_short,
}

/// One loaded ALSA sequencer symbol table.
#[derive(Debug)]
pub(super) struct AlsaApi {
    /// `snd_strerror`
    pub(super) snd_strerror: unsafe extern "C" fn(c_int) -> *const c_char,
    /// `snd_seq_open`
    pub(super) snd_seq_open:
        unsafe extern "C" fn(*mut *mut snd_seq_t, *const c_char, c_int, c_int) -> c_int,
    /// `snd_seq_close`
    pub(super) snd_seq_close: unsafe extern "C" fn(*mut snd_seq_t) -> c_int,
    /// `snd_seq_set_client_name`
    pub(super) snd_seq_set_client_name:
        unsafe extern "C" fn(*mut snd_seq_t, *const c_char) -> c_int,
    /// `snd_seq_client_id`
    pub(super) snd_seq_client_id: unsafe extern "C" fn(*mut snd_seq_t) -> c_int,
    /// `snd_seq_create_simple_port`
    pub(super) snd_seq_create_simple_port:
        unsafe extern "C" fn(*mut snd_seq_t, *const c_char, c_uint, c_uint) -> c_int,
    /// `snd_seq_delete_simple_port`
    pub(super) snd_seq_delete_simple_port: unsafe extern "C" fn(*mut snd_seq_t, c_int) -> c_int,
    /// `snd_seq_get_port_info`
    pub(super) snd_seq_get_port_info:
        unsafe extern "C" fn(*mut snd_seq_t, c_int, *mut snd_seq_port_info_t) -> c_int,
    /// `snd_seq_set_port_info`
    pub(super) snd_seq_set_port_info:
        unsafe extern "C" fn(*mut snd_seq_t, c_int, *mut snd_seq_port_info_t) -> c_int,
    /// `snd_seq_connect_from`
    pub(super) snd_seq_connect_from:
        unsafe extern "C" fn(*mut snd_seq_t, c_int, c_int, c_int) -> c_int,
    /// `snd_seq_connect_to`
    pub(super) snd_seq_connect_to:
        unsafe extern "C" fn(*mut snd_seq_t, c_int, c_int, c_int) -> c_int,
    /// `snd_seq_disconnect_from`
    pub(super) snd_seq_disconnect_from:
        unsafe extern "C" fn(*mut snd_seq_t, c_int, c_int, c_int) -> c_int,
    /// `snd_seq_disconnect_to`
    pub(super) snd_seq_disconnect_to:
        unsafe extern "C" fn(*mut snd_seq_t, c_int, c_int, c_int) -> c_int,
    /// `snd_seq_event_input`
    pub(super) snd_seq_event_input:
        unsafe extern "C" fn(*mut snd_seq_t, *mut *mut snd_seq_event_t) -> c_int,
    /// `snd_seq_free_event`
    pub(super) snd_seq_free_event: unsafe extern "C" fn(*mut snd_seq_event_t),
    /// `snd_seq_event_output_direct`
    pub(super) snd_seq_event_output_direct:
        unsafe extern "C" fn(*mut snd_seq_t, *mut snd_seq_event_t) -> c_int,
    /// `snd_seq_event_output`
    pub(super) snd_seq_event_output:
        unsafe extern "C" fn(*mut snd_seq_t, *mut snd_seq_event_t) -> c_int,
    /// `snd_seq_drain_output`
    pub(super) snd_seq_drain_output: unsafe extern "C" fn(*mut snd_seq_t) -> c_int,
    /// `snd_seq_alloc_named_queue`
    pub(super) snd_seq_alloc_named_queue:
        unsafe extern "C" fn(*mut snd_seq_t, *const c_char) -> c_int,
    /// `snd_seq_free_queue`
    pub(super) snd_seq_free_queue: unsafe extern "C" fn(*mut snd_seq_t, c_int) -> c_int,
    /// `snd_seq_control_queue`
    pub(super) snd_seq_control_queue:
        unsafe extern "C" fn(*mut snd_seq_t, c_int, c_int, c_int, *mut snd_seq_event_t) -> c_int,
    /// `snd_seq_event_length`
    pub(super) snd_seq_event_length: unsafe extern "C" fn(*mut snd_seq_event_t) -> c_long,
    /// `snd_seq_client_info_malloc`
    pub(super) snd_seq_client_info_malloc:
        unsafe extern "C" fn(*mut *mut snd_seq_client_info_t) -> c_int,
    /// `snd_seq_client_info_free`
    pub(super) snd_seq_client_info_free: unsafe extern "C" fn(*mut snd_seq_client_info_t),
    /// `snd_seq_client_info_set_client`
    pub(super) snd_seq_client_info_set_client:
        unsafe extern "C" fn(*mut snd_seq_client_info_t, c_int),
    /// `snd_seq_query_next_client`
    pub(super) snd_seq_query_next_client:
        unsafe extern "C" fn(*mut snd_seq_t, *mut snd_seq_client_info_t) -> c_int,
    /// `snd_seq_client_info_get_client`
    pub(super) snd_seq_client_info_get_client:
        unsafe extern "C" fn(*const snd_seq_client_info_t) -> c_int,
    /// `snd_seq_client_info_get_name`
    pub(super) snd_seq_client_info_get_name:
        unsafe extern "C" fn(*const snd_seq_client_info_t) -> *const c_char,
    /// `snd_seq_port_info_malloc`
    pub(super) snd_seq_port_info_malloc:
        unsafe extern "C" fn(*mut *mut snd_seq_port_info_t) -> c_int,
    /// `snd_seq_port_info_free`
    pub(super) snd_seq_port_info_free: unsafe extern "C" fn(*mut snd_seq_port_info_t),
    /// `snd_seq_port_info_set_client`
    pub(super) snd_seq_port_info_set_client: unsafe extern "C" fn(*mut snd_seq_port_info_t, c_int),
    /// `snd_seq_port_info_set_port`
    pub(super) snd_seq_port_info_set_port: unsafe extern "C" fn(*mut snd_seq_port_info_t, c_int),
    /// `snd_seq_query_next_port`
    pub(super) snd_seq_query_next_port:
        unsafe extern "C" fn(*mut snd_seq_t, *mut snd_seq_port_info_t) -> c_int,
    /// `snd_seq_port_info_get_port`
    pub(super) snd_seq_port_info_get_port:
        unsafe extern "C" fn(*const snd_seq_port_info_t) -> c_int,
    /// `snd_seq_port_info_get_name`
    pub(super) snd_seq_port_info_get_name:
        unsafe extern "C" fn(*const snd_seq_port_info_t) -> *const c_char,
    /// `snd_seq_port_info_get_capability`
    pub(super) snd_seq_port_info_get_capability:
        unsafe extern "C" fn(*const snd_seq_port_info_t) -> c_uint,
    /// `snd_seq_port_info_get_type`
    pub(super) snd_seq_port_info_get_type:
        unsafe extern "C" fn(*const snd_seq_port_info_t) -> c_uint,
    /// `snd_seq_port_info_set_timestamping`
    pub(super) snd_seq_port_info_set_timestamping:
        unsafe extern "C" fn(*mut snd_seq_port_info_t, c_int),
    /// `snd_seq_port_info_set_timestamp_real`
    pub(super) snd_seq_port_info_set_timestamp_real:
        unsafe extern "C" fn(*mut snd_seq_port_info_t, c_int),
    /// `snd_seq_port_info_set_timestamp_queue`
    pub(super) snd_seq_port_info_set_timestamp_queue:
        unsafe extern "C" fn(*mut snd_seq_port_info_t, c_int),
    /// `snd_seq_poll_descriptors_count`
    pub(super) snd_seq_poll_descriptors_count:
        unsafe extern "C" fn(*mut snd_seq_t, c_short) -> c_int,
    /// `snd_seq_poll_descriptors`
    pub(super) snd_seq_poll_descriptors:
        unsafe extern "C" fn(*mut snd_seq_t, *mut pollfd, c_uint, c_short) -> c_int,
    /// `snd_seq_port_subscribe_malloc`
    pub(super) snd_seq_port_subscribe_malloc:
        unsafe extern "C" fn(*mut *mut snd_seq_port_subscribe_t) -> c_int,
    /// `snd_seq_port_subscribe_free`
    pub(super) snd_seq_port_subscribe_free: unsafe extern "C" fn(*mut snd_seq_port_subscribe_t),
    /// `snd_seq_port_subscribe_set_sender`
    pub(super) snd_seq_port_subscribe_set_sender:
        unsafe extern "C" fn(*mut snd_seq_port_subscribe_t, *const snd_seq_addr_t),
    /// `snd_seq_port_subscribe_set_dest`
    pub(super) snd_seq_port_subscribe_set_dest:
        unsafe extern "C" fn(*mut snd_seq_port_subscribe_t, *const snd_seq_addr_t),
    /// `snd_seq_port_subscribe_set_queue`
    pub(super) snd_seq_port_subscribe_set_queue:
        unsafe extern "C" fn(*mut snd_seq_port_subscribe_t, c_int),
    /// `snd_seq_port_subscribe_set_time_update`
    pub(super) snd_seq_port_subscribe_set_time_update:
        unsafe extern "C" fn(*mut snd_seq_port_subscribe_t, c_int),
    /// `snd_seq_port_subscribe_set_time_real`
    pub(super) snd_seq_port_subscribe_set_time_real:
        unsafe extern "C" fn(*mut snd_seq_port_subscribe_t, c_int),
    /// `snd_seq_subscribe_port`
    pub(super) snd_seq_subscribe_port:
        unsafe extern "C" fn(*mut snd_seq_t, *mut snd_seq_port_subscribe_t) -> c_int,
    /// `snd_seq_unsubscribe_port`
    pub(super) snd_seq_unsubscribe_port:
        unsafe extern "C" fn(*mut snd_seq_t, *mut snd_seq_port_subscribe_t) -> c_int,
    /// `snd_midi_event_new`
    pub(super) snd_midi_event_new: unsafe extern "C" fn(c_int, *mut *mut snd_midi_event_t) -> c_int,
    /// `snd_midi_event_free`
    pub(super) snd_midi_event_free: unsafe extern "C" fn(*mut snd_midi_event_t),
    /// `snd_midi_event_init`
    pub(super) snd_midi_event_init: unsafe extern "C" fn(*mut snd_midi_event_t),
    /// `snd_midi_event_no_status`
    pub(super) snd_midi_event_no_status: unsafe extern "C" fn(*mut snd_midi_event_t, c_int),
    /// `snd_midi_event_encode`
    pub(super) snd_midi_event_encode: unsafe extern "C" fn(
        *mut snd_midi_event_t,
        *const c_uchar,
        c_long,
        *mut snd_seq_event_t,
    ) -> c_long,
    /// `snd_midi_event_decode`
    pub(super) snd_midi_event_decode: unsafe extern "C" fn(
        *mut snd_midi_event_t,
        *mut c_uchar,
        c_long,
        *mut snd_seq_event_t,
    ) -> c_long,
}

unsafe extern "C" {
    /// Poll one POSIX fd set.
    pub(super) fn poll(fds: *mut pollfd, nfds: c_ulong, timeout: c_int) -> c_int;
}
