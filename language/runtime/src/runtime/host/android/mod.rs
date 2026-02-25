#[cfg(any(test, target_os = "android"))]
mod abi;
#[cfg(target_os = "android")]
mod adapter;
#[cfg(any(test, target_os = "android"))]
mod callback;
#[cfg(any(test, target_os = "android"))]
mod crypto;
#[cfg(any(test, target_os = "android"))]
mod ffi;

#[cfg(any(test, target_os = "android"))]
pub use abi::{
    HOST_STATUS_BUFFER_TOO_SMALL, HOST_STATUS_FAILED, HOST_STATUS_INVALID_ARGUMENT,
    HOST_STATUS_NOT_FOUND, HOST_STATUS_NOT_SUPPORTED, HOST_STATUS_OK,
    HOST_STATUS_PERMISSION_DENIED,
};
#[cfg(target_os = "android")]
pub(super) use adapter::AndroidHostAdapter;
#[cfg(any(test, target_os = "android"))]
pub use callback::{
    AndroidActivityLifecycle, android_notify_activity_lifecycle,
    android_notify_interruption_changed, android_notify_memory_pressure_changed,
    android_notify_permission_request_in_flight, android_notify_permission_result,
    android_notify_power_mode_changed, android_notify_thermal_state_changed, android_notify_wake,
    android_notify_wall_clock_changed, android_notify_window_available,
    android_notify_window_focus_changed, android_notify_window_resized,
    android_notify_window_terminated,
};
#[cfg(any(test, target_os = "android"))]
pub use crypto::{
    AndroidHostCryptoCallbacks, AndroidHostDecryptHardwareKeyCallback,
    AndroidHostDeleteHardwareKeyCallback, AndroidHostDeriveHardwareSharedSecretCallback,
    AndroidHostExportHardwarePublicKeyCallback, AndroidHostGenerateHardwareKeyPairCallback,
    AndroidHostSignHardwareKeyCallback, AndroidHostSupportsHardwareKeyCallback,
    destack_runtime_host_android_crypto_callbacks_abi_version,
    destack_runtime_host_android_crypto_decrypt_hardware_key,
    destack_runtime_host_android_crypto_delete_hardware_key,
    destack_runtime_host_android_crypto_derive_hardware_shared_secret,
    destack_runtime_host_android_crypto_export_hardware_public_key,
    destack_runtime_host_android_crypto_generate_hardware_key_pair,
    destack_runtime_host_android_crypto_set_callbacks,
    destack_runtime_host_android_crypto_sign_hardware_key,
    destack_runtime_host_android_crypto_supports_hardware_key, set_android_host_crypto_callbacks,
};
#[cfg(any(test, target_os = "android"))]
pub use ffi::{
    destack_runtime_host_android_notify_activity_lifecycle,
    destack_runtime_host_android_notify_interruption_changed,
    destack_runtime_host_android_notify_memory_pressure_changed,
    destack_runtime_host_android_notify_permission_request_in_flight,
    destack_runtime_host_android_notify_permission_result,
    destack_runtime_host_android_notify_power_mode_changed,
    destack_runtime_host_android_notify_thermal_state_changed,
    destack_runtime_host_android_notify_wake,
    destack_runtime_host_android_notify_wall_clock_changed,
    destack_runtime_host_android_notify_window_available,
    destack_runtime_host_android_notify_window_focus_changed,
    destack_runtime_host_android_notify_window_resized,
    destack_runtime_host_android_notify_window_terminated,
};
