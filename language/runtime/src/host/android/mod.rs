#[cfg(any(test, target_os = "android"))]
mod abi;
#[cfg(target_os = "android")]
mod backend;
#[cfg(any(test, target_os = "android"))]
mod bindings;
#[cfg(any(test, target_os = "android"))]
mod callback;
#[cfg(any(test, target_os = "android"))]
mod credentials;
#[cfg(any(test, target_os = "android"))]
mod crypto;
#[cfg(any(test, target_os = "android"))]
mod ffi;
#[cfg(target_os = "android")]
mod message;
#[cfg(any(test, target_os = "android"))]
mod midi;
#[cfg(any(test, target_os = "android"))]
mod registry;
#[cfg(test)]
mod tests;

#[cfg(target_os = "android")]
pub(crate) use backend::AndroidHost;
#[cfg(any(test, target_os = "android"))]
pub use bindings::{AndroidHostBindings, destack_host_android_register_bindings};
#[cfg(any(test, target_os = "android"))]
pub use callback::{
    AndroidActivityLifecycle, android_notify_activity_lifecycle,
    android_notify_intent_custom_action, android_notify_intent_open_file,
    android_notify_intent_open_url, android_notify_intent_share_files,
    android_notify_intent_share_text, android_notify_interruption_changed,
    android_notify_memory_pressure_changed, android_notify_permission_result,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "android"))]
pub use credentials::{
    AndroidHostCredentialsAuthenticateCallback, AndroidHostCredentialsCallbacks,
    AndroidHostCredentialsContainsCallback, AndroidHostCredentialsDeleteCallback,
    AndroidHostCredentialsReadCallback, AndroidHostCredentialsWriteCallback,
    destack_host_android_credentials_authenticate, destack_host_android_credentials_contains,
    destack_host_android_credentials_delete, destack_host_android_credentials_read,
    destack_host_android_credentials_write,
};
#[cfg(any(test, target_os = "android"))]
pub use crypto::{
    AndroidHostComputeHardwareMacCallback, AndroidHostCryptoCallbacks,
    AndroidHostDecryptHardwareKeyCallback, AndroidHostDecryptHardwareSecretKeyCallback,
    AndroidHostDeleteCertificateCallback, AndroidHostDeleteHardwareKeyCallback,
    AndroidHostDeriveHardwareSharedSecretCallback, AndroidHostEncryptHardwareSecretKeyCallback,
    AndroidHostExportHardwarePublicKeyCallback, AndroidHostGenerateHardwareKeyPairCallback,
    AndroidHostGenerateHardwareSecretKeyCallback, AndroidHostImportCertificateCallback,
    AndroidHostSignHardwareKeyCallback, AndroidHostSupportsHardwareKeyCallback,
    destack_host_android_crypto_compute_hardware_mac,
    destack_host_android_crypto_decrypt_hardware_key,
    destack_host_android_crypto_decrypt_hardware_secret_key,
    destack_host_android_crypto_delete_certificate,
    destack_host_android_crypto_delete_hardware_key,
    destack_host_android_crypto_derive_hardware_shared_secret,
    destack_host_android_crypto_encrypt_hardware_secret_key,
    destack_host_android_crypto_export_hardware_public_key,
    destack_host_android_crypto_generate_hardware_key_pair,
    destack_host_android_crypto_generate_hardware_secret_key,
    destack_host_android_crypto_import_certificate, destack_host_android_crypto_sign_hardware_key,
    destack_host_android_crypto_supports_certificate_write,
    destack_host_android_crypto_supports_hardware_key,
    destack_host_android_crypto_supports_hardware_key_pair,
    destack_host_android_crypto_supports_hardware_secret_key,
};
#[cfg(any(test, target_os = "android"))]
pub use ffi::{
    destack_host_android_notify_activity_lifecycle,
    destack_host_android_notify_intent_custom_action, destack_host_android_notify_intent_open_file,
    destack_host_android_notify_intent_open_url, destack_host_android_notify_intent_share_files,
    destack_host_android_notify_intent_share_text,
    destack_host_android_notify_interruption_changed,
    destack_host_android_notify_memory_pressure_changed,
    destack_host_android_notify_permission_result, destack_host_android_notify_power_mode_changed,
    destack_host_android_notify_thermal_state_changed, destack_host_android_notify_wake,
    destack_host_android_notify_wall_clock_changed,
};
#[cfg(any(test, target_os = "android"))]
pub use midi::{
    AndroidHostMidiCallbacks, AndroidHostMidiDescribeBackendCallback,
    AndroidHostMidiEventCloseCallback, AndroidHostMidiEventHeader,
    AndroidHostMidiEventOpenCallback, AndroidHostMidiEventReadCallback,
    AndroidHostMidiInputPortCloseCallback, AndroidHostMidiInputPortListCallback,
    AndroidHostMidiInputPortOpenCallback, AndroidHostMidiInputReadCallback,
    AndroidHostMidiInputRecordHeader, AndroidHostMidiInputVirtualCreateCallback,
    AndroidHostMidiOpenedPortHeader, AndroidHostMidiOutputPortCloseCallback,
    AndroidHostMidiOutputPortListCallback, AndroidHostMidiOutputPortOpenCallback,
    AndroidHostMidiOutputRecordHeader, AndroidHostMidiOutputVirtualCreateCallback,
    AndroidHostMidiOutputWriteCallback, AndroidHostMidiPortDescriptorHeader,
    destack_host_android_midi_describe_backend, destack_host_android_midi_event_close,
    destack_host_android_midi_event_open, destack_host_android_midi_event_read,
    destack_host_android_midi_input_port_close, destack_host_android_midi_input_port_list,
    destack_host_android_midi_input_port_open, destack_host_android_midi_input_read,
    destack_host_android_midi_input_virtual_create, destack_host_android_midi_output_port_close,
    destack_host_android_midi_output_port_list, destack_host_android_midi_output_port_open,
    destack_host_android_midi_output_virtual_create, destack_host_android_midi_output_write,
};
#[cfg(any(test, target_os = "android"))]
pub(crate) use registry::unregister_android_bindings;

/// Remove runtime-scoped Android host interop state.
#[cfg(any(test, target_os = "android"))]
pub(crate) fn unregister_android_host_runtime(runtime_id: u64) {
    unregister_android_bindings(runtime_id);
}
