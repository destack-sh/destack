use std::sync::Arc;

use crate::platform::core as core_platform;

use super::abi::AlsaApi;
use super::core::AlsaLibrary;

/// Return one loaded ALSA dynamic-library handle when available.
pub(super) fn alsa_library() -> Option<Arc<AlsaLibrary>> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    initialize_alsa_library().ok()
}

/// Return one ALSA dynamic-library load error when initialization failed.
pub(super) fn alsa_library_error() -> Option<String> {
    if !cfg!(target_os = "linux") {
        return None;
    }

    initialize_alsa_library().err()
}

/// Initialize one ALSA dynamic library and symbol table.
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

/// Load one complete ALSA symbol table.
unsafe fn load_alsa_api(
    library: &core_platform::DynamicLibrary,
    candidate: &str,
) -> Result<AlsaApi, String> {
    core_platform::load_dll_api_named!(library, candidate, AlsaApi {
        snd_strerror => "snd_strerror",
        snd_device_name_hint => "snd_device_name_hint",
        snd_device_name_get_hint => "snd_device_name_get_hint",
        snd_device_name_free_hint => "snd_device_name_free_hint",
        snd_pcm_open => "snd_pcm_open",
        snd_pcm_close => "snd_pcm_close",
        snd_pcm_nonblock => "snd_pcm_nonblock",
        snd_pcm_hw_params_malloc => "snd_pcm_hw_params_malloc",
        snd_pcm_hw_params_free => "snd_pcm_hw_params_free",
        snd_pcm_hw_params_any => "snd_pcm_hw_params_any",
        snd_pcm_hw_params_current => "snd_pcm_hw_params_current",
        snd_pcm_hw_params => "snd_pcm_hw_params",
        snd_pcm_hw_params_set_access => "snd_pcm_hw_params_set_access",
        snd_pcm_hw_params_set_format => "snd_pcm_hw_params_set_format",
        snd_pcm_hw_params_set_channels_near => "snd_pcm_hw_params_set_channels_near",
        snd_pcm_hw_params_set_rate_near => "snd_pcm_hw_params_set_rate_near",
        snd_pcm_hw_params_set_rate_resample => "snd_pcm_hw_params_set_rate_resample",
        snd_pcm_hw_params_set_period_size_near => "snd_pcm_hw_params_set_period_size_near",
        snd_pcm_hw_params_set_buffer_size_near => "snd_pcm_hw_params_set_buffer_size_near",
        snd_pcm_hw_params_get_rate => "snd_pcm_hw_params_get_rate",
        snd_pcm_hw_params_get_rate_min => "snd_pcm_hw_params_get_rate_min",
        snd_pcm_hw_params_get_rate_max => "snd_pcm_hw_params_get_rate_max",
        snd_pcm_hw_params_get_channels => "snd_pcm_hw_params_get_channels",
        snd_pcm_hw_params_get_channels_min => "snd_pcm_hw_params_get_channels_min",
        snd_pcm_hw_params_get_channels_max => "snd_pcm_hw_params_get_channels_max",
        snd_pcm_hw_params_get_period_size => "snd_pcm_hw_params_get_period_size",
        snd_pcm_hw_params_get_period_size_min => "snd_pcm_hw_params_get_period_size_min",
        snd_pcm_hw_params_get_period_size_max => "snd_pcm_hw_params_get_period_size_max",
        snd_pcm_hw_params_test_rate => "snd_pcm_hw_params_test_rate",
        snd_pcm_hw_params_test_format => "snd_pcm_hw_params_test_format",
        snd_pcm_hw_params_can_pause => "snd_pcm_hw_params_can_pause",
        snd_pcm_format_value => "snd_pcm_format_value",
        snd_pcm_prepare => "snd_pcm_prepare",
        snd_pcm_start => "snd_pcm_start",
        snd_pcm_pause => "snd_pcm_pause",
        snd_pcm_drop => "snd_pcm_drop",
        snd_pcm_reset => "snd_pcm_reset",
        snd_pcm_recover => "snd_pcm_recover",
        snd_pcm_wait => "snd_pcm_wait",
        snd_pcm_avail_update => "snd_pcm_avail_update",
        snd_pcm_delay => "snd_pcm_delay",
        snd_pcm_readi => "snd_pcm_readi",
        snd_pcm_writei => "snd_pcm_writei",
        snd_pcm_type => "snd_pcm_type",
    })
}
