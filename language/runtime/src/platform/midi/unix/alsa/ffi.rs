use std::sync::Arc;

use crate::platform::core as core_platform;

use super::abi::AlsaApi;
use super::core::AlsaLibrary;

/// Return one loaded ALSA sequencer dynamic-library handle when available.
pub(super) fn alsa_library() -> Option<Arc<AlsaLibrary>> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    initialize_alsa_library().ok()
}

/// Initialize one ALSA sequencer dynamic library and symbol table.
fn initialize_alsa_library() -> Result<Arc<AlsaLibrary>, String> {
    let (library, api) = core_platform::load_library_with_api(
        &["libasound.so.2", "libasound.so"],
        |library, candidate| unsafe { load_alsa_api(library, candidate) },
    )?;

    Ok(Arc::new(AlsaLibrary {
        _library: library,
        api,
    }))
}

/// Load one complete ALSA sequencer symbol table.
unsafe fn load_alsa_api(
    library: &core_platform::DynamicLibrary,
    candidate: &str,
) -> Result<AlsaApi, String> {
    core_platform::load_dll_api_named!(library, candidate, AlsaApi {
        snd_strerror => "snd_strerror",
        snd_seq_open => "snd_seq_open",
        snd_seq_close => "snd_seq_close",
        snd_seq_set_client_name => "snd_seq_set_client_name",
        snd_seq_client_id => "snd_seq_client_id",
        snd_seq_create_simple_port => "snd_seq_create_simple_port",
        snd_seq_delete_simple_port => "snd_seq_delete_simple_port",
        snd_seq_get_port_info => "snd_seq_get_port_info",
        snd_seq_set_port_info => "snd_seq_set_port_info",
        snd_seq_connect_from => "snd_seq_connect_from",
        snd_seq_connect_to => "snd_seq_connect_to",
        snd_seq_disconnect_from => "snd_seq_disconnect_from",
        snd_seq_disconnect_to => "snd_seq_disconnect_to",
        snd_seq_event_input => "snd_seq_event_input",
        snd_seq_free_event => "snd_seq_free_event",
        snd_seq_event_output_direct => "snd_seq_event_output_direct",
        snd_seq_event_output => "snd_seq_event_output",
        snd_seq_drain_output => "snd_seq_drain_output",
        snd_seq_alloc_named_queue => "snd_seq_alloc_named_queue",
        snd_seq_free_queue => "snd_seq_free_queue",
        snd_seq_control_queue => "snd_seq_control_queue",
        snd_seq_event_length => "snd_seq_event_length",
        snd_seq_client_info_malloc => "snd_seq_client_info_malloc",
        snd_seq_client_info_free => "snd_seq_client_info_free",
        snd_seq_client_info_set_client => "snd_seq_client_info_set_client",
        snd_seq_query_next_client => "snd_seq_query_next_client",
        snd_seq_client_info_get_client => "snd_seq_client_info_get_client",
        snd_seq_client_info_get_name => "snd_seq_client_info_get_name",
        snd_seq_port_info_malloc => "snd_seq_port_info_malloc",
        snd_seq_port_info_free => "snd_seq_port_info_free",
        snd_seq_port_info_set_client => "snd_seq_port_info_set_client",
        snd_seq_port_info_set_port => "snd_seq_port_info_set_port",
        snd_seq_query_next_port => "snd_seq_query_next_port",
        snd_seq_port_info_get_port => "snd_seq_port_info_get_port",
        snd_seq_port_info_get_name => "snd_seq_port_info_get_name",
        snd_seq_port_info_get_capability => "snd_seq_port_info_get_capability",
        snd_seq_port_info_get_type => "snd_seq_port_info_get_type",
        snd_seq_port_info_set_timestamping => "snd_seq_port_info_set_timestamping",
        snd_seq_port_info_set_timestamp_real => "snd_seq_port_info_set_timestamp_real",
        snd_seq_port_info_set_timestamp_queue => "snd_seq_port_info_set_timestamp_queue",
        snd_seq_poll_descriptors_count => "snd_seq_poll_descriptors_count",
        snd_seq_poll_descriptors => "snd_seq_poll_descriptors",
        snd_seq_port_subscribe_malloc => "snd_seq_port_subscribe_malloc",
        snd_seq_port_subscribe_free => "snd_seq_port_subscribe_free",
        snd_seq_port_subscribe_set_sender => "snd_seq_port_subscribe_set_sender",
        snd_seq_port_subscribe_set_dest => "snd_seq_port_subscribe_set_dest",
        snd_seq_port_subscribe_set_queue => "snd_seq_port_subscribe_set_queue",
        snd_seq_port_subscribe_set_time_update => "snd_seq_port_subscribe_set_time_update",
        snd_seq_port_subscribe_set_time_real => "snd_seq_port_subscribe_set_time_real",
        snd_seq_subscribe_port => "snd_seq_subscribe_port",
        snd_seq_unsubscribe_port => "snd_seq_unsubscribe_port",
        snd_midi_event_new => "snd_midi_event_new",
        snd_midi_event_free => "snd_midi_event_free",
        snd_midi_event_init => "snd_midi_event_init",
        snd_midi_event_no_status => "snd_midi_event_no_status",
        snd_midi_event_encode => "snd_midi_event_encode",
        snd_midi_event_decode => "snd_midi_event_decode",
    })
}
